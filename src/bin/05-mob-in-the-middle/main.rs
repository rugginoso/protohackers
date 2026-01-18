use futures::StreamExt;
use protohackers::utils;
use tokio::net::TcpListener;
use tokio_util::codec::Framed;

mod substitute;

mod codec;
use codec::MITMCodec;

mod pipeline;

const TONY_BOGUSCOIN_ADDRESS: &str = "7YWHMfk9JZe0LM0g1ZauHuiSxhI";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    utils::setup_tracing();

    let listen_addr = utils::listen_address(0)?;
    let listener = TcpListener::bind(listen_addr).await?;
    tracing::info!("server listening on {}", listen_addr);

    loop {
        let (client_stream, client_addr) = listener.accept().await?;
        let server_stream = tokio::net::TcpStream::connect("chat.protohackers.com:16963").await?;
        tracing::info!("connected to chat.protohackers.com:16963");

        tokio::spawn(async move {
            tracing::info!("accepted connection from {}", &client_addr);

            let client_framed = Framed::new(client_stream, MITMCodec::new());
            let (client_tx, client_rx) = client_framed.split();

            let server_framed = Framed::new(server_stream, MITMCodec::new());
            let (server_tx, server_rx) = server_framed.split();

            let events = futures::stream::select(
                pipeline::client_stream(client_rx),
                pipeline::server_stream(server_rx),
            );

            tokio::pin!(events);

            if let Err(err) =
                pipeline::run(events, client_tx, server_tx, TONY_BOGUSCOIN_ADDRESS).await
            {
                tracing::error!("pipeline error for {}: {}", client_addr, err);
            }

            tracing::info!("connection {} closed", client_addr);
        });
    }
}
