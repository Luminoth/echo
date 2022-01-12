use std::future;
use std::sync::Arc;

use aws_gamelift_server_sdk_rs::{
    log_parameters::LogParameters, process_parameters::ProcessParameters,
};
use futures_util::FutureExt;
use tokio::sync::{mpsc, watch, RwLock};
use tracing::{error, info, warn};

use crate::server;

pub async fn run(port: u16) -> anyhow::Result<()> {
    let mut api = aws_gamelift_server_sdk_rs::api::Api::default();
    api.init_sdk().await?;

    let (terminate_sender, mut terminate_receiver) = mpsc::unbounded_channel();
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);

    let api = Arc::new(RwLock::new(api));

    api.write()
        .await
        .process_ready(ProcessParameters {
            on_start_game_session: Box::new({
                let api = api.clone();
                move |game_session| {
                    info!("Starting game session: {:?}", game_session);

                    let callbacks = server::ServerCallbacks {
                        begin_session: Box::new({
                            let api = api.clone();
                            move || {
                                let api = api.clone();
                                async move {
                                    if let Err(err) =
                                        api.write().await.activate_game_session().await
                                    {
                                        error!("Failed to begin session: {}", err);
                                    }
                                }
                                .boxed()
                            }
                        }),
                        end_session: Box::new({
                            let api = api.clone();
                            move || {
                                let api = api.clone();
                                async move {
                                    if let Err(err) = api.write().await.process_ending().await {
                                        error!("Failed to end session: {}", err);
                                    }
                                }
                                .boxed()
                            }
                        }),
                        accept_player_session: Box::new({
                            let api = api.clone();
                            move |player_session_id| {
                                let api = api.clone();
                                async move {
                                    // TODO: have to spawn to get around the internal SDK lock being held
                                    tokio::spawn(async move {
                                        if let Err(err) = api
                                            .write()
                                            .await
                                            .accept_player_session(player_session_id)
                                            .await
                                        {
                                            error!("Player session accept error: {}", err);
                                        }
                                    });
                                }
                                .boxed()
                            }
                        }),
                        remove_player_session: Box::new({
                            let api = api.clone();
                            move |player_session_id| {
                                let api = api.clone();
                                async move {
                                    // TODO: have to spawn to get around the internal SDK lock being held
                                    tokio::spawn(async move {
                                        if let Err(err) = api
                                            .write()
                                            .await
                                            .remove_player_session(player_session_id)
                                            .await
                                        {
                                            error!("Player session remove error: {}", err);
                                        }
                                    });
                                }
                                .boxed()
                            }
                        }),
                    };

                    // spawn the server process
                    tokio::spawn(server::run(
                        format!("0.0.0.0:{}", port),
                        false,
                        shutdown_receiver.clone(),
                        callbacks,
                        Some(60),
                    ));

                    info!("Waiting for session ...");

                    future::ready(()).boxed()
                }
            }),
            on_update_game_session: Box::new(|update_game_session| {
                warn!("Update game session: {:?}", update_game_session);

                future::ready(()).boxed()
            }),
            on_process_terminate: Box::new(move || {
                info!("Process terminating ...");

                shutdown_sender.send(true).unwrap();
                terminate_sender.send(true).unwrap();

                future::ready(()).boxed()
            }),
            on_health_check: Box::new(|| {
                async move {
                    info!("health check");
                    true
                }
                .boxed()
            }),
            port: port as i32,
            log_parameters: LogParameters {
                log_paths: vec!["logs".to_string()],
            },
        })
        .await?;

    info!("Waiting for game session ...");

    terminate_receiver.recv().await;

    info!("Process terminated!");

    Ok(())
}
