use std::future;
use std::sync::Arc;

use aws_gamelift_server_sdk_rs::{
    log_parameters::LogParameters, process_parameters::ProcessParameters,
};
use futures_util::FutureExt;
use tokio::sync::{mpsc, watch, RwLock};
use tracing::{info, warn};

use crate::server;
use crate::util;

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
                        accept_player_session: Box::new({
                            let api = api.clone();
                            move |player_session_id| {
                                let api = api.clone();
                                async move {
                                    api.write()
                                        .await
                                        .accept_player_session(player_session_id)
                                        .await
                                        .expect("Invalid player session for accept!");
                                }
                                .boxed()
                            }
                        }),
                        remove_player_session: Box::new({
                            let api = api.clone();
                            move |player_session_id| {
                                let api = api.clone();
                                async move {
                                    api.write()
                                        .await
                                        .remove_player_session(player_session_id)
                                        .await
                                        .expect("Invalid player session for remove!");
                                }
                                .boxed()
                            }
                        }),
                    };

                    // spawn the server process
                    let (ready_sender, ready_receiver) = watch::channel(false);
                    tokio::spawn(server::run(
                        format!("0.0.0.0:{}", port),
                        false,
                        ready_sender,
                        shutdown_receiver.clone(),
                        callbacks,
                    ));

                    let api = api.clone();
                    async move {
                        // wait for the server to be ready
                        util::wait_for_signal(ready_receiver).await.unwrap();

                        // update gamelift
                        api.write().await.activate_game_session().await.unwrap();
                    }
                    .boxed()
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
            on_health_check: Box::new(|| async move { true }.boxed()),
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
