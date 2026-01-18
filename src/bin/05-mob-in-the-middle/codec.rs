use tokio_util::codec::{Decoder, Encoder, LinesCodec};

pub struct MITMCodec {
    inner: LinesCodec,
}

impl MITMCodec {
    pub fn new() -> Self {
        Self {
            inner: LinesCodec::new(),
        }
    }
}

impl Decoder for MITMCodec {
    type Item = <LinesCodec as Decoder>::Item;
    type Error = <LinesCodec as Decoder>::Error;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode(src)
    }

    fn decode_eof(
        &mut self,
        buf: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        // discard data without newline
        buf.clear();
        Ok(None)
    }
}

impl Encoder<String> for MITMCodec {
    type Error = <LinesCodec as Encoder<String>>::Error;

    fn encode(
        &mut self,
        item: String,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        self.inner.encode(item, dst)
    }
}

#[cfg(test)]
mod tests {
    use tokio_util::{bytes::BytesMut, codec::Decoder};

    use crate::codec::MITMCodec;

    #[test]
    fn test_decode_without_newline_return_none() {
        let mut codec = MITMCodec::new();

        let mut buf = BytesMut::from("foo");
        assert!(codec.decode(&mut buf).unwrap().is_none());
    }

    #[test]
    fn test_decode_eof_without_newline_return_none() {
        let mut codec = MITMCodec::new();

        let mut buf = BytesMut::from("foo");
        assert!(codec.decode_eof(&mut buf).unwrap().is_none());
    }
}
