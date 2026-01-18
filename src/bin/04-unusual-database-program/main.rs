use anyhow::Context;
use futures::{SinkExt, StreamExt};
use protohackers::utils;
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;
use tower::{Service, ServiceExt};

use crate::{codec::UDPCodec, service::UDPService};

mod codec;
mod service;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    utils::setup_tracing();

    let listen_addr = utils::listen_address(0)?;
    let socket = UdpSocket::bind(listen_addr)
        .await
        .context("bind udp socket")?;
    tracing::info!("listening on {}", listen_addr.to_string());

    let framed = UdpFramed::new(socket, UDPCodec::default());
    let (mut tx, mut rx) = framed.split();
    let mut svc = UDPService::default();

    while let Some(req) = rx.next().await {
        let (req, client_addr) = match req {
            Ok(req) => req,
            Err(err) => {
                tracing::error!("error recv request: {err}");
                continue;
            }
        };
        tracing::debug!(client_addr = ?client_addr, "req" = ?req, "recv request");

        let svc = match ServiceExt::ready(&mut svc).await {
            Ok(svc) => svc,
            Err(err) => {
                tracing::error!("error getting service ready: {err}");
                continue;
            }
        };

        let resp = match svc.call(req).await {
            Ok(resp) => resp,
            Err(err) => {
                tracing::error!("error calling service: {err}");
                continue;
            }
        };

        if let Some(resp) = resp {
            tracing::debug!(client_addr = ?client_addr, "resp" = ?resp, "send response");

            if let Err(err) = tx.send((resp, client_addr)).await {
                tracing::error!("error sending response: {err}");
            }
        }
    }

    Ok(())
}
