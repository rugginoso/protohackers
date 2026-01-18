use std::str::FromStr;
use tokio_util::codec::{Decoder, Encoder, LinesCodec, LinesCodecError};

use crate::service::{PrimeTimeRequest, PrimeTimeResponse};

const METHOD_NAME: &str = "isPrime";
const INVALID_METHOD_NAME: &str = "invalid";

mod private {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize)]
    pub struct PrimeTimeRequestPayload {
        pub method: String,
        pub number: serde_json::Number,
    }

    #[derive(Debug, Serialize)]
    pub struct PrimeTimeResponsePayload {
        pub method: &'static str,
        pub prime: bool,
    }
}

#[derive(Clone, Debug)]
pub struct PrimeTimeCodec {
    inner: LinesCodec,
}

impl PrimeTimeCodec {
    pub fn new() -> Self {
        Self {
            inner: LinesCodec::new(),
        }
    }
}

impl Decoder for PrimeTimeCodec {
    type Item = PrimeTimeRequest;
    type Error = PrimeTimeCodecError;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let line = match self.inner.decode(src)? {
            Some(line) => line,
            None => return Ok(None),
        };

        Ok(Some(line.parse()?))
    }
}

impl FromStr for PrimeTimeRequest {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match serde_json::from_str::<private::PrimeTimeRequestPayload>(s) {
            Ok(payload) => {
                if payload.method != METHOD_NAME {
                    Ok(PrimeTimeRequest::Invalid)
                } else {
                    // payload.number.as_u64() returns Some(n) if number can be represented as u64,
                    // None otherwise
                    Ok(PrimeTimeRequest::IsPrime(payload.number.as_u64()))
                }
            }
            Err(_) => Ok(PrimeTimeRequest::Invalid),
        }
    }
}

impl Encoder<PrimeTimeResponse> for PrimeTimeCodec {
    type Error = PrimeTimeCodecError;

    fn encode(
        &mut self,
        item: PrimeTimeResponse,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        let payload = match item {
            PrimeTimeResponse::IsPrime(prime) => private::PrimeTimeResponsePayload {
                method: METHOD_NAME,
                prime,
            },
            PrimeTimeResponse::Invalid => private::PrimeTimeResponsePayload {
                method: INVALID_METHOD_NAME,
                prime: false,
            },
        };

        let json = serde_json::to_string(&payload)?;
        self.inner.encode(json, dst).map_err(Into::into)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PrimeTimeCodecError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lines codec error: {0}")]
    LinesCodec(#[from] LinesCodecError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    mod decoder {
        use tokio_util::{bytes::BytesMut, codec::Decoder};

        use crate::{codec::PrimeTimeCodec, service::PrimeTimeRequest};

        #[test]
        fn valid_input() {
            let input = concat!(r#"{"method": "isPrime", "number": 7}"#, "\n");
            let mut buf = BytesMut::from(input);
            let mut codec = PrimeTimeCodec::new();

            assert_eq!(
                codec.decode(&mut buf).unwrap(),
                Some(PrimeTimeRequest::IsPrime(Some(7)))
            );
        }

        #[test]
        fn extra_fields() {
            let input = concat!(r#"{"method": "isPrime", "foo": "bar", "number": 7}"#, "\n");
            let mut buf = BytesMut::from(input);
            let mut codec = PrimeTimeCodec::new();

            assert_eq!(
                codec.decode(&mut buf).unwrap(),
                Some(PrimeTimeRequest::IsPrime(Some(7)))
            );
        }

        #[test]
        fn number_not_an_integer() {
            let input = concat!(r#"{"method": "isPrime", "number": 7.5}"#, "\n");
            let mut buf = BytesMut::from(input);
            let mut codec = PrimeTimeCodec::new();

            assert_eq!(
                codec.decode(&mut buf).unwrap(),
                Some(PrimeTimeRequest::IsPrime(None))
            );
        }

        #[test]
        fn number_not_positive() {
            let input = concat!(r#"{"method": "isPrime", "number": -7}"#, "\n");
            let mut buf = BytesMut::from(input);
            let mut codec = PrimeTimeCodec::new();

            assert_eq!(
                codec.decode(&mut buf).unwrap(),
                Some(PrimeTimeRequest::IsPrime(None))
            );
        }

        #[test]
        fn number_missing() {
            let input = concat!(r#"{"method": "isPrime"}"#, "\n");
            let mut buf = BytesMut::from(input);
            let mut codec = PrimeTimeCodec::new();

            assert_eq!(
                codec.decode(&mut buf).unwrap(),
                Some(PrimeTimeRequest::Invalid)
            );
        }

        #[test]
        fn method_invalid() {
            let input = concat!(r#"{"method": "notvalid", "number": 7}"#, "\n");
            let mut buf = BytesMut::from(input);
            let mut codec = PrimeTimeCodec::new();

            assert_eq!(
                codec.decode(&mut buf).unwrap(),
                Some(PrimeTimeRequest::Invalid)
            );
        }

        #[test]
        fn method_missing() {
            let input = concat!(r#"{"number": 7}"#, "\n");
            let mut buf = BytesMut::from(input);
            let mut codec = PrimeTimeCodec::new();

            assert_eq!(
                codec.decode(&mut buf).unwrap(),
                Some(PrimeTimeRequest::Invalid)
            );
        }

        #[test]
        fn invalid_json() {
            let input = concat!(r#"{"number}"#, "\n");
            let mut buf = BytesMut::from(input);
            let mut codec = PrimeTimeCodec::new();

            assert_eq!(
                codec.decode(&mut buf).unwrap(),
                Some(PrimeTimeRequest::Invalid)
            );
        }

        #[test]
        fn incomplete_input() {
            let input = r#"{"number"#;
            let mut buf = BytesMut::from(input);
            let mut codec = PrimeTimeCodec::new();

            assert_eq!(codec.decode(&mut buf).unwrap(), None);
        }
    }

    mod encoder {
        use tokio_util::{bytes::BytesMut, codec::Encoder};

        use crate::{codec::PrimeTimeCodec, service::PrimeTimeResponse};

        #[test]
        fn is_prime() {
            let mut buf = BytesMut::new();
            let mut codec = PrimeTimeCodec::new();

            assert!(
                codec
                    .encode(PrimeTimeResponse::IsPrime(true), &mut buf)
                    .is_ok()
            );
            assert_eq!(
                String::from_utf8(buf.split().to_vec()).unwrap(),
                String::from(concat!(r#"{"method":"isPrime","prime":true}"#, "\n"))
            );
        }

        #[test]
        fn is_not_prime() {
            let mut buf = BytesMut::new();
            let mut codec = PrimeTimeCodec::new();

            assert!(
                codec
                    .encode(PrimeTimeResponse::IsPrime(false), &mut buf)
                    .is_ok()
            );
            assert_eq!(
                String::from_utf8(buf.split().to_vec()).unwrap(),
                String::from(concat!(r#"{"method":"isPrime","prime":false}"#, "\n"))
            );
        }

        #[test]
        fn invalid_response() {
            let mut buf = BytesMut::new();
            let mut codec = PrimeTimeCodec::new();

            assert!(codec.encode(PrimeTimeResponse::Invalid, &mut buf).is_ok());
            assert_eq!(
                String::from_utf8(buf.split().to_vec()).unwrap(),
                String::from(concat!(r#"{"method":"invalid","prime":false}"#, "\n"))
            );
        }
    }
}
