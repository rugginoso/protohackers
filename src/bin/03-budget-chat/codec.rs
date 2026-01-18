use tokio_util::{
    bytes::BytesMut,
    codec::{Decoder, Encoder, LinesCodec},
};

use crate::{handshake::HandshakeResponse, service::BudgetChatEvent, username::Username};

pub struct BudgetChatCodec {
    inner: LinesCodec,
}

impl BudgetChatCodec {
    pub fn new() -> Self {
        Self {
            inner: LinesCodec::new(),
        }
    }
}

impl Decoder for BudgetChatCodec {
    type Item = String;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode(src).map_err(Into::into)
    }
}

impl Encoder<BudgetChatEvent> for BudgetChatCodec {
    type Error = Error;

    fn encode(&mut self, item: BudgetChatEvent, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let line = match item {
            BudgetChatEvent::Join { username } => format!("* {} has entered the room", username),
            BudgetChatEvent::Leave { username } => format!("* {} has left the room", username),
            BudgetChatEvent::Message { username, content } => {
                format!("[{}] {}", username, content)
            }
        };
        self.inner.encode(line, dst).map_err(Into::into)
    }
}

impl Encoder<HandshakeResponse> for BudgetChatCodec {
    type Error = Error;

    fn encode(&mut self, item: HandshakeResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let line = match item {
            HandshakeResponse::Welcome => {
                "Welcome to budgetchat! What shall I call you?".to_string()
            }
            HandshakeResponse::InvalidUsername(cause) => {
                format!("Invalid username: {cause}")
            }
            HandshakeResponse::UsersList { users } => {
                format!(
                    "* The room contains: {}",
                    users
                        .iter()
                        .map(Username::inner)
                        .collect::<Vec<&str>>()
                        .join(", ")
                )
            }
        };
        self.inner.encode(line, dst).map_err(Into::into)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lines codec error: {0}")]
    LinesCodec(#[from] tokio_util::codec::LinesCodecError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
