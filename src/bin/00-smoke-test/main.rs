use anyhow::Context;
use protohackers::utils;
use tokio::{io, net::TcpListener};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    utils::setup_tracing();

    let listen_addr = utils::listen_address(0)?;
    let listener = TcpListener::bind(listen_addr).await.context("bind")?;
    tracing::info!("listening on {}", listen_addr.to_string());

    loop {
        let (mut stream, client_addr) = listener.accept().await.context("accept")?;
        tracing::info!("accepted connection from {}", client_addr.to_string());

        tokio::spawn(async move {
            let (mut rx, mut tx) = stream.split();

            tracing::info!("serving connection from {}", client_addr.to_string());
            loop {
                match io::copy(&mut rx, &mut tx).await {
                    Ok(0) => {
                        tracing::debug!("{} closed connection", client_addr.to_string());
                        break;
                    }
                    Ok(len) => {
                        tracing::debug!("copied {len} bytes");
                    }
                    Err(err) => {
                        tracing::error!("error coping bytes {err}");
                        break;
                    }
                }
            }
            tracing::info!("end serving {}", client_addr.to_string());
        });
    }
}
