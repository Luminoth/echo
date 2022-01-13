mod client;
mod gamelift;
mod options;
mod server;
mod util;

use std::sync::Arc;

use futures_util::FutureExt;
use tokio::sync::{watch, Mutex};
use tracing::info;
use tracing_subscriber::{filter, prelude::*};
use uuid::Uuid;

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

    let (shutdown_sender, shutdown_receiver) = watch::channel(false);

    match options.mode {
        options::Mode::Connect(_) => {
            // TODO: allow optional CLI arg for the player id and player session id
            // so that GameLift local testing can function correctly
            let player_id = Uuid::new_v4().to_string();
            client::connect(options.connect_addr(), &player_id, &player_id).await?;
        }
        options::Mode::Find(_) => {
            client::find().await?;
        }
        options::Mode::Server(_) => {
            let (ready_sender, ready_receiver) = watch::channel(false);
            let ready_sender = Arc::new(Mutex::new(ready_sender));

            // spawn the server process
            let server_handle = tokio::spawn(server::run(
                options.server_addr(),
                true,
                shutdown_receiver,
                server::ServerCallbacks {
                    begin_session: Box::new({
                        let ready_sender = ready_sender.clone();
                        move || {
                            let ready_sender = ready_sender.clone();
                            async move {
                                ready_sender.lock().await.send(true).unwrap();
                            }
                            .boxed()
                        }
                    }),
                    ..Default::default()
                },
                None,
            ));

            info!("Waiting for ready ...");

            // wait for the server to be ready
            util::wait_for_signal(ready_receiver).await?;

            // run the client
            // TODO: allow optional CLI arg for the player id
            let player_id = Uuid::new_v4().to_string();
            client::connect(options.connect_addr(), &player_id, &player_id).await?;

            shutdown_sender.send(true)?;

            server_handle.await??;
        }
        options::Mode::Dedicated(_) => {
            server::run(
                options.server_addr(),
                false,
                shutdown_receiver,
                server::ServerCallbacks::default(),
                None,
            )
            .await?;
        }
        options::Mode::GameLift(cmd) => {
            gamelift::run(cmd.port).await?;
        }
    };

    Ok(())
}
