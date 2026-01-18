use std::sync::{Arc, Mutex};

use futures::{SinkExt, Stream, StreamExt};
use tokio::{net::TcpStream, sync::broadcast};
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::codec::Framed;

use crate::{
    codec::{BudgetChatCodec, Error as CodecError},
    service::BudgetChatEvent,
    state::{Error as StateError, State},
    username::{Error as UsernameError, Username},
};

#[derive(Debug, Clone)]
pub enum HandshakeResponse {
    Welcome,
    InvalidUsername(String),
    UsersList { users: Vec<Username> },
}

pub async fn handshake(
    state: Arc<Mutex<State>>,
    framed: &mut Framed<TcpStream, BudgetChatCodec>,
    publisher: broadcast::Sender<BudgetChatEvent>,
) -> Result<(Username, impl Stream<Item = BudgetChatEvent> + use<>), Error> {
    framed.send(HandshakeResponse::Welcome).await?;

    let username = match framed.next().await.transpose() {
        Ok(Some(line)) => line.parse::<Username>().map_err(Error::from),
        Ok(None) => Err(Error::from(UsernameError::EmptyUsername)),
        Err(err) => Err(Error::from(err)),
    }?;

    let already_connected_users = state
        .lock()
        .expect("failed to get state lock")
        .add_user(username.clone())?;

    framed
        .send(HandshakeResponse::UsersList {
            users: already_connected_users,
        })
        .await?;

    let _ = publisher.send(BudgetChatEvent::Join {
        username: username.clone(),
    });

    let subscriber = BroadcastStream::new(publisher.subscribe());
    let filtered_subscriber = {
        let username = username.clone();
        subscriber.filter_map(move |event| {
            let username = username.clone();
            async move {
                match event {
                    Ok(event) if !event.is_from(&username) => Some(event),
                    _ => None,
                }
            }
        })
    };

    Ok((username, filtered_subscriber))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("codec error: {0}")]
    CodecError(#[from] CodecError),
    #[error("username error: {0}")]
    Username(#[from] UsernameError),
    #[error("state error: {0}")]
    State(#[from] StateError),
}
