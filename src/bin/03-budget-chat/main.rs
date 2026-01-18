use std::sync::{Arc, Mutex};

use anyhow::Context;
use futures::{SinkExt, StreamExt};

use protohackers::utils;
use tokio::{net::TcpListener, sync::broadcast};
use tokio_util::codec::Framed;

use tower::{Service, ServiceExt};

mod codec;
use codec::BudgetChatCodec;

mod handshake;
use handshake::handshake;

mod service;
use service::{BudgetChatEvent, BudgetChatServiceFactory};

use crate::handshake::{Error as HandshakeError, HandshakeResponse};

mod state;
mod username;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    utils::setup_tracing();

    let listen_addr = utils::listen_address(0)?;
    let listener = TcpListener::bind(listen_addr).await.context("bind")?;
    tracing::info!("listening on {}", listen_addr.to_string());

    let state = Arc::new(Mutex::new(state::State::default()));
    let make_service = BudgetChatServiceFactory;
    let (publisher, _) = broadcast::channel::<BudgetChatEvent>(100);

    loop {
        let (stream, client_addr) = listener.accept().await?;
        let publisher = publisher.clone();
        let state = state.clone();
        let mut make_service = make_service.clone();

        tokio::spawn(async move {
            tracing::info!("accepted connection from {}", &client_addr);

            let mut framed = Framed::new(stream, BudgetChatCodec::new());

            let handshake_result = handshake(state.clone(), &mut framed, publisher.clone()).await;
            let (username, subscriber) = match handshake_result {
                Ok(user_and_subscriber) => user_and_subscriber,
                Err(HandshakeError::Username(err)) => {
                    let _ = framed
                        .send(HandshakeResponse::InvalidUsername(err.to_string()))
                        .await;
                    return;
                }
                Err(HandshakeError::State(err)) => {
                    let _ = framed
                        .send(HandshakeResponse::InvalidUsername(err.to_string()))
                        .await;
                    return;
                }
                Err(err) => {
                    tracing::error!("handshake error: {err}");
                    return;
                }
            };

            tokio::pin!(subscriber);

            let mut svc = ServiceExt::ready_oneshot(&mut make_service)
                .await
                .expect("failed get factory ready")
                .call(username.clone())
                .await
                .expect("failed to create service");

            loop {
                tokio::select! {
                    line = framed.next() => {
                        match line {
                            Some(Ok(line)) => {
                                let svc = ServiceExt::ready(&mut svc).await.expect("failed to get service ready");
                                match svc.call(line).await {
                                    Ok(event) => {
                                        if let Err(err) = publisher.send(event.clone()) {
                                            tracing::debug!("error publishing event: {err}");
                                            continue;
                                        }
                                    },
                                    Err(e) => {
                                        tracing::error!("error processing message from {}: {}", &client_addr, e);
                                        break;
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                tracing::error!("error reading from {}: {}", &client_addr, e);
                                break;
                            }
                            None => {
                                tracing::info!("client closed connection");
                                break;
                            }
                        }
                    },
                    Some(msg) = subscriber.next() => {
                        if let Err(err) = framed.send(msg).await {
                            tracing::error!("error sending to {}: {}", &client_addr, err);
                            break;
                        }
                    }
                }
            }
            state.lock().expect("cannot unlock").remove_user(&username);
            if let Err(err) = publisher.send(BudgetChatEvent::Leave { username }) {
                tracing::debug!("error publishing event: {err}");
            }

            tracing::info!("connection from {} closed", &client_addr);
        });
    }
}
