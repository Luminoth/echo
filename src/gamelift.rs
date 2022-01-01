use aws_gamelift_server_sdk_rs::{
    log_parameters::LogParameters, process_parameters::ProcessParameters,
};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::info;

use crate::server;

pub async fn run(port: u16) -> anyhow::Result<()> {
    let mut api = aws_gamelift_server_sdk_rs::api::Api::default();
    api.init_sdk().await?;

    let (terminate_sender, mut terminate_receiver) = mpsc::unbounded_channel();
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    api.process_ready(ProcessParameters {
        on_start_game_session: Box::new(|_game_session| {
            info!("start game session");

            /*tokio::spawn(server::run(
                format!("0.0.0.0:{}", port),
                false,
                shutdown_receiver,
            ));*/
        }),
        on_update_game_session: Box::new(|_update_game_session| info!("update game session")),
        on_process_terminate: Box::new(move || {
            shutdown_sender.send(true).unwrap();
            terminate_sender.send(true).unwrap();
        }),
        on_health_check: Box::new(|| true),
        port: port as i32,
        log_parameters: LogParameters {
            log_paths: vec!["logs".to_string()],
        },
    })
    .await?;

    terminate_receiver.recv().await;

    Ok(())
}
