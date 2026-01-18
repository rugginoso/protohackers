use protohackers::utils;

use std::sync::{Arc, Mutex};

use tokio::net::TcpListener;

mod client;
mod codec;
mod state;

use state::State;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    utils::setup_tracing();

    let listen_addr = utils::listen_address(0)?;
    let listener = TcpListener::bind(listen_addr).await?;
    tracing::info!("server listening on {}", listen_addr);

    let state = Arc::new(Mutex::new(State::default()));

    loop {
        let (stream, client_addr) = listener.accept().await?;
        let state = state.clone();

        tokio::spawn(async move {
            tracing::info!("{} connected", &client_addr);
            client::handle_client(stream, state).await;
            tracing::info!("{} disconnected", &client_addr);
        });
    }
}
