use std::time::Duration;

use tokio_util::{
    bytes::{Buf, BytesMut},
    codec::{Decoder, Encoder},
};

use crate::state::{self, MileMarker, RoadID, Ticket, Timestamp};

mod decode {
    use std::time::Duration;

    use tokio_util::bytes::BytesMut;

    use crate::state::{self, MileMarker, RoadID, Timestamp};

    pub struct FrameCursor<'a> {
        buf: &'a BytesMut,
        pos: usize,
    }

    impl<'a> FrameCursor<'a> {
        pub fn new(buf: &'a BytesMut) -> Self {
            Self { buf, pos: 0 }
        }

        fn remaining(&self) -> usize {
            self.buf.len() - self.pos
        }

        pub fn peek_u8(&mut self) -> Option<u8> {
            if self.remaining() < 1 {
                None
            } else {
                let v = self.buf[self.pos];
                self.pos += 1;
                Some(v)
            }
        }

        fn peek_bytes(&mut self, len: usize) -> Option<&'a [u8]> {
            if self.remaining() < len {
                None
            } else {
                let start = self.pos;
                self.pos += len;
                Some(&self.buf[start..start + len])
            }
        }

        pub fn peek_u16(&mut self) -> Option<u16> {
            let bytes = self.peek_bytes(2)?;
            Some(u16::from_be_bytes(bytes.try_into().unwrap()))
        }

        pub fn peek_u32(&mut self) -> Option<u32> {
            let bytes = self.peek_bytes(4)?;
            Some(u32::from_be_bytes(bytes.try_into().unwrap()))
        }

        pub fn peek_string(&mut self) -> Result<Option<String>, super::Error> {
            let len = match self.peek_u8() {
                None => return Ok(None),
                Some(value) => value,
            };
            let bytes = match self.peek_bytes(len as usize) {
                None => return Ok(None),
                Some(value) => value,
            };
            Ok(Some(String::from_utf8(bytes.to_vec())?))
        }

        pub fn pos(&self) -> usize {
            self.pos
        }
    }

    macro_rules! ensure_some {
        ($val:expr) => {{
            match $val {
                None => return Ok(None),
                Some(v) => v,
            }
        }};
    }
    pub(super) use ensure_some;

    macro_rules! ensure_some_result {
        ($val:expr) => {{
            match $val? {
                None => return Ok(None),
                Some(v) => v,
            }
        }};
    }

    pub fn plate(cursor: &mut FrameCursor) -> Result<Option<super::IncomingMessage>, super::Error> {
        let plate = ensure_some_result!(cursor.peek_string());
        let timestamp = Timestamp::new(ensure_some!(cursor.peek_u32()));
        Ok(Some(super::IncomingMessage::Plate { plate, timestamp }))
    }

    pub fn want_heartbeat(
        cursor: &mut FrameCursor,
    ) -> Result<Option<super::IncomingMessage>, super::Error> {
        let interval = Duration::from_millis(u64::from(ensure_some!(cursor.peek_u32())) * 100);
        Ok(Some(super::IncomingMessage::WantHeartBeat { interval }))
    }

    pub fn i_am_camera(
        cursor: &mut FrameCursor,
    ) -> Result<Option<super::IncomingMessage>, super::Error> {
        let road_id: RoadID = ensure_some!(cursor.peek_u16()).into();
        let mile: MileMarker = ensure_some!(cursor.peek_u16()).into();
        let limit = state::MphX100::try_from(u32::from(ensure_some!(cursor.peek_u16())) * 100)?;
        Ok(Some(super::IncomingMessage::IAmCamera {
            road_id,
            mile,
            limit,
        }))
    }

    pub fn i_am_dispatcher(
        cursor: &mut FrameCursor,
    ) -> Result<Option<super::IncomingMessage>, super::Error> {
        let num_roads = ensure_some!(cursor.peek_u8()) as usize;

        let mut roads_ids = Vec::with_capacity(num_roads);
        for _ in 0..num_roads {
            let road_id = ensure_some!(cursor.peek_u16()).into();
            roads_ids.push(road_id);
        }

        Ok(Some(super::IncomingMessage::IAmDispatcher { roads_ids }))
    }
}

mod encode {
    use tokio_util::bytes::{BufMut, BytesMut};

    use crate::state::Ticket;

    fn string(buf: &mut BytesMut, value: &str) -> Result<(), super::Error> {
        let len: u8 = value.len().try_into()?;
        buf.put_u8(len);
        buf.put_slice(value.as_bytes());
        Ok(())
    }

    pub fn error(buf: &mut BytesMut, msg: String) -> Result<(), super::Error> {
        buf.put_u8(0x10);
        string(buf, &msg)
    }

    pub fn ticket(buf: &mut BytesMut, ticket: Ticket) -> Result<(), super::Error> {
        buf.put_u8(0x21);
        string(buf, &ticket.plate)?;
        buf.put_u16(ticket.road_id.inner());
        buf.put_u16(ticket.observation1.mile.inner());
        buf.put_u32(ticket.observation1.timestamp.inner());
        buf.put_u16(ticket.observation2.mile.inner());
        buf.put_u32(ticket.observation2.timestamp.inner());
        buf.put_u16(ticket.speed.inner());
        Ok(())
    }

    pub fn heartbeat(buf: &mut BytesMut) -> Result<(), super::Error> {
        buf.put_u8(0x41);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum IncomingMessage {
    Plate {
        plate: String,
        timestamp: Timestamp,
    },
    WantHeartBeat {
        interval: Duration,
    },
    IAmCamera {
        road_id: RoadID,
        mile: MileMarker,
        limit: state::MphX100,
    },
    IAmDispatcher {
        roads_ids: Vec<RoadID>,
    },
}

#[derive(Debug)]
pub enum OutgoingMessage {
    Error(String),
    Ticket(Ticket),
    HeartBeat,
}

pub struct SpeedDaemonCodec;

impl Decoder for SpeedDaemonCodec {
    type Item = IncomingMessage;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut cursor = decode::FrameCursor::new(src);

        let message_type = decode::ensure_some!(cursor.peek_u8());
        let result = match message_type {
            0x20 => decode::plate(&mut cursor),
            0x40 => decode::want_heartbeat(&mut cursor),
            0x80 => decode::i_am_camera(&mut cursor),
            0x81 => decode::i_am_dispatcher(&mut cursor),
            _ => Err(Error::UnknownMessageType(message_type)),
        };

        if result.as_ref().is_ok() && result.as_ref().unwrap().is_some() {
            src.advance(cursor.pos());
        }

        result
    }
}

impl Encoder<OutgoingMessage> for SpeedDaemonCodec {
    type Error = Error;

    fn encode(&mut self, item: OutgoingMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            OutgoingMessage::Error(msg) => encode::error(dst, msg),
            OutgoingMessage::Ticket(ticket) => encode::ticket(dst, ticket),
            OutgoingMessage::HeartBeat => encode::heartbeat(dst),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("int conversion error: {0}")]
    IntConversion(#[from] std::num::TryFromIntError),
    #[error("unknown message type: {0}")]
    UnknownMessageType(u8),
}

#[cfg(test)]
mod tests {
    mod decoder {
        use tokio_util::{bytes::BufMut, codec::Decoder};

        use crate::state::Timestamp;

        #[test]
        fn test_decode_want_heartbeat() {
            let mut buf = tokio_util::bytes::BytesMut::new();
            buf.put_u8(0x40);
            buf.put_u32(10); // ten "deciseconds" = 1 second

            let mut codec = super::super::SpeedDaemonCodec;
            let msg = codec.decode(&mut buf).unwrap().unwrap();
            match msg {
                super::super::IncomingMessage::WantHeartBeat { interval } => {
                    assert_eq!(interval.as_millis(), 1000);
                }
                _ => panic!("unexpected message: {:?}", msg),
            }
        }

        #[test]
        fn test_decode_plate() {
            let mut buf = tokio_util::bytes::BytesMut::with_capacity(1024);
            buf.put_u8(0x20);
            buf.put_u8(5);
            buf.put_slice("a1234".as_bytes());
            buf.put_u32(u32::MAX);

            let mut codec = super::super::SpeedDaemonCodec;
            let msg = codec.decode(&mut buf).unwrap().unwrap();
            match msg {
                super::super::IncomingMessage::Plate { plate, timestamp } => {
                    assert_eq!(plate, String::from("a1234"));
                    assert_eq!(timestamp, Timestamp::new(u32::MAX));
                }
                _ => panic!("unexpected message: {:?}", msg),
            }
        }

        #[test]
        fn test_decode_i_am_camera_limit_overflow() {
            let mut buf = tokio_util::bytes::BytesMut::new();
            buf.put_u8(0x80);
            buf.put_u16(1); // road id
            buf.put_u16(1); // mile marker
            buf.put_u16(1000); // speed limit in mph, 1000 * 100 does not fit in u16

            let mut codec = super::super::SpeedDaemonCodec;
            let err = codec.decode(&mut buf).expect_err("decode should fail");
            assert!(matches!(err, super::super::Error::IntConversion(_)));
            assert_eq!(buf.len(), 7, "buffer should not be advanced on decode error");
        }
    }

    mod encoder {
        use tokio_util::codec::Encoder;

        #[test]
        fn test_encode_heartbeat() {
            let mut buf = tokio_util::bytes::BytesMut::new();
            let mut codec = super::super::SpeedDaemonCodec;
            codec
                .encode(super::super::OutgoingMessage::HeartBeat, &mut buf)
                .unwrap();

            let expected: Vec<u8> = vec![0x41];
            assert_eq!(&buf[..], &expected[..]);
        }
    }
}
