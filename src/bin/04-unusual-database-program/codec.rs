use tokio_util::{
    bytes::BytesMut,
    codec::{Decoder, Encoder},
};

use crate::service::{UDPRequest, UDPResponse};

const PAYLOAD_MAX_LENGTH: usize = 1000;

#[derive(Default)]
pub struct UDPCodec {
    /// `frame_read` is required to correctly handle empty UDP datagrams.
    ///
    /// When using `tokio_util::codec::UdpFramed`, incoming UDP packets are
    /// parsed exclusively via `Decoder::decode_eof`, which is repeatedly
    /// invoked until it returns `Ok(None)`.
    ///
    /// In this protocol, an *empty UDP payload is a valid request*
    /// (i.e. `Retrieve { key: "" }`). However, after the empty datagram
    /// has been parsed once, subsequent calls to `decode_eof` receive
    /// the same empty `BytesMut`, making it impossible to distinguish:
    ///
    /// - a *new* empty datagram (valid request)
    /// - from a buffer that has already been consumed
    ///
    /// using the buffer state alone.
    ///
    /// This flag acts as a per-datagram one-shot guard, ensuring that:
    /// - exactly one request is emitted per UDP packet
    /// - empty payloads are handled correctly
    /// - infinite emission loops in `decode_eof` are avoided
    ///
    /// This state is not a workaround but a consequence of using the
    /// stream-oriented `Decoder` API for a message-oriented transport
    /// with valid zero-length frames.
    frame_read: bool,
}

impl Decoder for UDPCodec {
    type Item = UDPRequest;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.frame_read {
            self.frame_read = false;
            src.clear();
            return Ok(None);
        }

        self.frame_read = true;

        if src.len() >= PAYLOAD_MAX_LENGTH {
            src.clear();
            return Err(Error::TooLong);
        }

        let buf = src.split();
        let text = str::from_utf8(&buf)?;
        let req = match text.split_once('=') {
            Some((k, v)) => UDPRequest::Insert {
                key: k.to_owned().into(),
                value: v.to_owned().into(),
            },
            None => UDPRequest::Retrieve {
                key: text.to_owned().into(),
            },
        };

        Ok(Some(req))
    }
}

impl Encoder<UDPResponse> for UDPCodec {
    type Error = Error;

    fn encode(&mut self, item: UDPResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload = format!("{}={}", item.key, item.value);
        let bytes = payload.as_bytes();
        if bytes.len() >= PAYLOAD_MAX_LENGTH {
            Err(Error::TooLong)
        } else {
            dst.extend_from_slice(payload.as_bytes());
            Ok(())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("too long")]
    TooLong,
}

#[cfg(test)]
mod tests {
    mod decoder {
        use tokio_util::{bytes::BytesMut, codec::Decoder};

        use crate::{
            codec::UDPCodec,
            service::UDPRequest,
            state::{Key, Value},
        };

        #[test]
        fn test_decode_insert() {
            let mut codec = UDPCodec::default();

            let mut buf = BytesMut::from("foo=bar");
            assert_eq!(
                codec.decode_eof(&mut buf).unwrap(),
                Some(UDPRequest::Insert {
                    key: Key::from("foo"),
                    value: Value::from("bar")
                })
            );

            assert_eq!(codec.decode_eof(&mut buf).unwrap(), None);
        }

        #[test]
        fn test_decode_retrieve() {
            let mut codec = UDPCodec::default();

            let mut buf = BytesMut::from("foo");
            assert_eq!(
                codec.decode_eof(&mut buf).unwrap(),
                Some(UDPRequest::Retrieve {
                    key: Key::from("foo"),
                })
            );

            assert_eq!(codec.decode_eof(&mut buf).unwrap(), None);
        }

        #[test]
        fn test_decode_retrieve_empty_key() {
            let mut codec = UDPCodec::default();

            let mut buf = BytesMut::from("");
            assert_eq!(
                codec.decode_eof(&mut buf).unwrap(),
                Some(UDPRequest::Retrieve { key: Key::from("") })
            );

            assert_eq!(codec.decode_eof(&mut buf).unwrap(), None);
        }
    }

    mod encoder {
        use tokio_util::{bytes::BytesMut, codec::Encoder};

        use crate::{codec::UDPCodec, service::UDPResponse};

        #[test]
        fn test_encode_response() {
            let mut codec = UDPCodec::default();
            let mut buf = BytesMut::new();
            let resp = UDPResponse {
                key: "foo".to_owned().into(),
                value: "bar".to_owned().into(),
            };
            assert!(codec.encode(resp, &mut buf).is_ok());
            assert_eq!(str::from_utf8(&buf).unwrap(), "foo=bar");
        }
    }
}
