mod client;
mod gamelift;
mod options;
mod server;
mod util;

use tokio::sync::watch;
use tracing_subscriber::{filter, prelude::*};

fn init_logging() -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let file_appender = tracing_appender::rolling::daily("logs", "echo.log");
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .and_then(tracing_subscriber::fmt::layer().with_writer(non_blocking_appender))
                .with_filter(filter::LevelFilter::INFO),
        )
        .init();

    Ok(guard)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options: options::Options = argh::from_env();

    // TODO: make this not mutually exclusive
    let _guard = if options.tracing {
        console_subscriber::init();
        None
    } else {
        Some(init_logging()?)
    };

    let (ready_sender, ready_receiver) = watch::channel(false);
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);

    match options.mode {
        options::Mode::Connect(_) => {
            client::connect(options.connect_addr()).await?;
        }
        options::Mode::Find(_) => {
            client::find().await?;
        }
        options::Mode::Server(_) => {
            // spawn the server process
            let server_handle = tokio::spawn(server::run(
                options.server_addr(),
                true,
                ready_sender,
                shutdown_receiver,
                server::ServerCallbacks::default(),
            ));

            // wait for the server to be ready
            util::wait_for_signal(ready_receiver).await?;

            // run the client
            client::connect(options.connect_addr()).await?;

            shutdown_sender.send(true)?;

            server_handle.await??;
        }
        options::Mode::Dedicated(_) => {
            server::run(
                options.server_addr(),
                false,
                ready_sender,
                shutdown_receiver,
                server::ServerCallbacks::default(),
            )
            .await?;
        }
        options::Mode::GameLift(cmd) => {
            gamelift::run(cmd.port).await?;
        }
    };

    Ok(())
}
