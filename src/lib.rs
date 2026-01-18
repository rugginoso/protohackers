/// utils module contains common utilities used across problems
pub mod utils {
    use std::net::SocketAddr;
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    /// Parse the nth_arg as SocketAddr
    ///
    /// the program name is ignored, so 0 is the first argument
    pub fn listen_address(nth_arg: usize) -> anyhow::Result<SocketAddr> {
        let arg = std::env::args().nth(nth_arg + 1);
        anyhow::ensure!(arg.is_some(), "listen address not provided");

        Ok(arg.unwrap().parse::<SocketAddr>()?)
    }

    /// Setup tracing_subscriber to format messages in human-readable format and filter by
    /// environment variables
    pub fn setup_tracing() {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env())
            .init();
    }
}
