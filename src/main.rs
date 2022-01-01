mod client;
mod options;
mod server;

use tokio::sync::mpsc;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn init_logging() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options: options::Options = argh::from_env();

    // TODO: make this not mutually exclusive
    if options.tracing {
        console_subscriber::init();
    } else {
        init_logging()?;
    }

    let (shutdown_sender, shutdown_receiver) = mpsc::unbounded_channel();

    // start server
    let server_handle = if options.is_gamelift() {
        todo!();
    } else if options.is_server() {
        Some(tokio::spawn(server::run(
            options.server_addr(),
            options.is_client(),
            shutdown_receiver,
        )))

        // TODO: server mode needs to wait for the listener to start before trying to connect
    } else {
        None
    };

    // start client
    if options.is_connect() {
        client::connect(options.connect_addr()).await?;
        shutdown_sender.send(true)?;
    } else if options.is_find() {
        client::find().await?;
        shutdown_sender.send(true)?;
    }

    if let Some(server_handle) = server_handle {
        server_handle.await??;
    }

    Ok(())
}
