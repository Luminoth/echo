mod client;
mod gamelift;
mod options;
mod server;

use tokio::sync::broadcast;
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

    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    match options.mode {
        options::Mode::Connect(_) => {
            client::connect(options.connect_addr()).await?;
        }
        options::Mode::Find(_) => {
            client::find().await?;
        }
        options::Mode::Server(_) => {
            let server_handle =
                tokio::spawn(server::run(options.server_addr(), true, shutdown_receiver));

            // TODO: need to wait for the server listener before starting the client

            client::connect(options.connect_addr()).await?;

            shutdown_sender.send(true)?;

            server_handle.await??;
        }
        options::Mode::Dedicated(_) => {
            server::run(options.server_addr(), false, shutdown_receiver).await?;
        }
        options::Mode::GameLift(cmd) => {
            gamelift::run(cmd.port).await?;
        }
    };

    Ok(())
}
