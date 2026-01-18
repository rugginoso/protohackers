use anyhow::Context;
use futures::{SinkExt, StreamExt};
use protohackers::utils;
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use tower::{Service, ServiceExt};

use crate::{codec::MeansToAnEndCodec, service::MeansToAnEndService};

mod codec;
mod service;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    utils::setup_tracing();

    let listen_addr = utils::listen_address(0)?;
    let listener = TcpListener::bind(listen_addr).await.context("bind")?;
    tracing::info!("listening on {}", listen_addr.to_string());

    loop {
        let (stream, client_addr) = listener.accept().await.context("accept")?;
        tracing::info!("accepted connection from {}", client_addr.to_string());

        tokio::spawn(async move {
            let framed = Framed::new(stream, MeansToAnEndCodec);
            let (mut tx, mut rx) = framed.split();
            let mut svc = MeansToAnEndService::default();
            let svc = ServiceExt::ready(&mut svc)
                .await
                .expect("cannot get ready instance of service");

            tracing::info!("serving connection from {}", client_addr.to_string());
            while let Some(result) = rx.next().await {
                let req = match result {
                    Ok(req) => req,
                    Err(err) => {
                        tracing::error!("error reading request: {}", err);
                        break;
                    }
                };

                let resp = match svc.call(req).await {
                    Ok(resp) => resp,
                    Err(err) => {
                        tracing::error!("error processing request: {}", err);
                        break;
                    }
                };

                if let Err(err) = tx.send(resp).await {
                    tracing::error!("error writing response: {}", err);
                    break;
                }
            }
            tracing::info!("end serving {}", client_addr.to_string());
        });
    }
}
