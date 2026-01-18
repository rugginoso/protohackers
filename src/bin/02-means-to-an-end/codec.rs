use tokio_util::bytes::{Buf, BufMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::service::{MeansToAnEndRequest, MeansToAnEndResponse};
use crate::state::{Price, Timestamp};

pub struct MeansToAnEndCodec;

const FRAME_LEN: usize = 9;

impl Decoder for MeansToAnEndCodec {
    type Item = MeansToAnEndRequest;
    type Error = MeansToAnEndCodecError;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < FRAME_LEN {
            src.reserve(FRAME_LEN - src.len());
            return Ok(None);
        }

        match src.get_u8() {
            b'I' => Ok(Some(MeansToAnEndRequest::Insert {
                timestamp: Timestamp::new(src.get_i32()),
                price: Price::new(src.get_i32()),
            })),
            b'Q' => Ok(Some(MeansToAnEndRequest::Query(
                Timestamp::new(src.get_i32())..=Timestamp::new(src.get_i32()),
            ))),
            other => Err(MeansToAnEndCodecError::Protocol(format!(
                "invalid request type {other}"
            ))),
        }
    }
}

impl Encoder<MeansToAnEndResponse> for MeansToAnEndCodec {
    type Error = MeansToAnEndCodecError;

    fn encode(
        &mut self,
        item: MeansToAnEndResponse,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        if let MeansToAnEndResponse::Mean(mean) = item {
            dst.put_i32(mean.inner());
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MeansToAnEndCodecError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
}

#[cfg(test)]
mod tests {
    use tokio_util::bytes::BytesMut;

    use super::*;

    fn append_request(buf: &mut impl BufMut, id: u8, payload: [i32; 2]) {
        buf.put_u8(id);
        buf.put_i32(payload[0]);
        buf.put_i32(payload[1]);
    }

    fn append_insert_request(buf: &mut impl BufMut, timestamp: i32, price: i32) {
        append_request(buf, b'I', [timestamp, price]);
    }

    fn append_query_request(buf: &mut impl BufMut, start: i32, end: i32) {
        append_request(buf, b'Q', [start, end]);
    }

    #[test]
    fn test_decode_insert_request() {
        let mut codec = MeansToAnEndCodec;

        let mut data = BytesMut::new();
        append_insert_request(&mut data, 1, 10);
        let req = codec.decode(&mut data);
        assert_eq!(
            req.ok().unwrap(),
            Some(MeansToAnEndRequest::Insert {
                timestamp: Timestamp::new(1),
                price: Price::new(10),
            })
        );
    }

    #[test]
    fn test_decode_query_request() {
        let mut codec = MeansToAnEndCodec;

        let mut data = BytesMut::new();
        append_query_request(&mut data, 1, 10);
        let req = codec.decode(&mut data);
        assert_eq!(
            req.ok().unwrap(),
            Some(MeansToAnEndRequest::Query(
                Timestamp::new(1)..=Timestamp::new(10)
            ))
        );
    }

    #[test]
    fn test_decode_multiple_requests() {
        let mut codec = MeansToAnEndCodec;

        let mut data = BytesMut::new();
        append_insert_request(&mut data, 1, 10);
        append_query_request(&mut data, 2, 20);

        let excpected = [
            Some(MeansToAnEndRequest::Insert {
                timestamp: Timestamp::new(1),
                price: Price::new(10),
            }),
            Some(MeansToAnEndRequest::Query(
                Timestamp::new(2)..=Timestamp::new(20),
            )),
        ];

        for exp in excpected {
            let req = codec.decode(&mut data);
            assert_eq!(req.ok().unwrap(), exp,);
        }
    }

    #[test]
    fn test_decode_malformed_request() {
        let mut codec = MeansToAnEndCodec;

        let mut data = BytesMut::new();
        append_request(&mut data, b'!', [0, 0]);

        assert!(codec.decode(&mut data).is_err());
    }

    #[test]
    fn test_request_more_data() {
        let mut codec = MeansToAnEndCodec;
        let mut data = BytesMut::new();

        assert!(matches!(codec.decode(&mut data), Ok(None)));

        data.put_u8(b'I');
        assert!(matches!(codec.decode(&mut data), Ok(None)));

        data.put_i32(10);
        assert!(matches!(codec.decode(&mut data), Ok(None)));

        data.put_i32(20);
        let req = codec.decode(&mut data);
        assert_eq!(
            req.ok().unwrap(),
            Some(MeansToAnEndRequest::Insert {
                timestamp: Timestamp::new(10),
                price: Price::new(20),
            })
        );
    }
}
