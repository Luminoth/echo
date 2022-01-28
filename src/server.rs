use std::future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::bail;
use aws_gamelift_server_sdk_rs::{
    log_parameters::LogParameters, process_parameters::ProcessParameters,
};
use chrono::Utc;
use futures_util::FutureExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, watch, RwLock},
    time,
};
use tracing::{debug, error, info, warn};

type BeginSessionOutput = Pin<Box<dyn future::Future<Output = ()> + Send>>;
type BeginSession = Box<dyn Fn() -> BeginSessionOutput + Send + Sync>;

type EndSessionOutput = Pin<Box<dyn future::Future<Output = ()> + Send>>;
type EndSession = Box<dyn Fn() -> EndSessionOutput + Send + Sync>;

type AcceptPlayerSessionOutput = Pin<Box<dyn future::Future<Output = ()> + Send>>;
type AcceptPlayerSession = Box<dyn Fn(String) -> AcceptPlayerSessionOutput + Send + Sync>;

type RemovePlayerSessionOutput = Pin<Box<dyn future::Future<Output = ()> + Send>>;
type RemovePlayerSession = Box<dyn Fn(String) -> RemovePlayerSessionOutput + Send + Sync>;

pub struct ServerCallbacks {
    pub begin_session: BeginSession,
    pub end_session: EndSession,

    pub accept_player_session: AcceptPlayerSession,
    pub remove_player_session: RemovePlayerSession,
}

impl Default for ServerCallbacks {
    fn default() -> Self {
        Self {
            begin_session: Box::new(|| future::ready(()).boxed()),
            end_session: Box::new(|| future::ready(()).boxed()),
            accept_player_session: Box::new(|_| future::ready(()).boxed()),
            remove_player_session: Box::new(|_| future::ready(()).boxed()),
        }
    }
}

#[derive(Default)]
struct ServerState {
    callbacks: ServerCallbacks,
    timeout: Option<u64>,

    player_count: usize,
    last_update_time: i64,
}

impl ServerState {
    fn timed_out(&self) -> bool {
        if let Some(timeout) = self.timeout {
            return self.player_count == 0
                && Utc::now().timestamp() >= self.last_update_time + timeout as i64;
        }
        false
    }
}

async fn read_player_session_id(stream: &mut TcpStream) -> anyhow::Result<String> {
    let len = stream.read_u8().await? as usize;

    let mut buf = vec![0; len];

    let mut t = 0;
    while t < len {
        let n = stream.read(&mut buf[t..]).await?;
        if n == 0 {
            bail!("Connection closed!");
        }

        t += n;
    }

    Ok(std::str::from_utf8(&buf)?.to_string())
}

async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    silent: bool,
    state: Arc<RwLock<ServerState>>,
) -> anyhow::Result<()> {
    // TODO: read player id from stream
    let player_id = "N/A";

    let player_session_id = match read_player_session_id(&mut stream).await {
        Ok(player_session_id) => player_session_id,
        Err(err) => {
            info!("Connection from {} error: {}", addr, err);
            return Ok(());
        }
    };

    info!("Accepted player {} ({})", player_id, player_session_id);
    {
        let mut state = state.write().await;
        (state.callbacks.accept_player_session)(player_session_id.clone()).await;

        state.player_count += 1;
        state.last_update_time = Utc::now().timestamp();
    }

    let mut buf = [0; 1024];
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            info!("Connection from {} closed", addr);

            {
                let mut state = state.write().await;
                (state.callbacks.remove_player_session)(player_session_id.clone()).await;

                state.player_count -= 1;
                state.last_update_time = Utc::now().timestamp();
            }
            info!("Removed player {} ({})", player_id, player_session_id);

            return Ok(());
        }

        if !silent {
            info!("Read from {}: {}", addr, std::str::from_utf8(&buf[0..n])?);
        }

        stream.write_all(&buf[0..n]).await?;
    }
}

pub async fn run(
    addr: impl AsRef<str>,
    silent: bool,
    mut shutdown: watch::Receiver<bool>,
    callbacks: ServerCallbacks,
    timeout: Option<u64>,
) -> anyhow::Result<()> {
    let state = Arc::new(RwLock::new(ServerState {
        callbacks,
        timeout,
        last_update_time: Utc::now().timestamp(),
        ..Default::default()
    }));

    info!("Listening on {}", addr.as_ref());
    let listener = TcpListener::bind(addr.as_ref()).await?;

    info!("Starting session ...");
    (state.read().await.callbacks.begin_session)().await;

    loop {
        let mut timer = time::interval(time::Duration::from_secs(timeout.unwrap_or(300)));

        tokio::select! {
            res = listener.accept() => {
                let (stream, addr) = res?;

                info!("New connection from {}", addr);
                tokio::spawn(handle_connection(stream, addr, silent, state.clone()));

            },
            _ = timer.tick() => {
                let state = state.read().await;
                if state.timed_out() {
                    info!("Session timed out, exiting ...");
                    (state.callbacks.end_session)().await;
                    return Ok(());
                }
            }
            _ = shutdown.changed() => {
                let shutdown = shutdown.borrow();
                if *shutdown {
                    info!("Received shutdown, exiting ...");
                    return Ok(());
                }
            }
        }
    }
}

pub async fn run_gamelift(port: u16) -> anyhow::Result<()> {
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

                    let callbacks = ServerCallbacks {
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
                                    if let Err(err) = api
                                        .write()
                                        .await
                                        .accept_player_session(player_session_id)
                                        .await
                                    {
                                        error!("Player session accept error: {}", err);
                                    }
                                }
                                .boxed()
                            }
                        }),
                        remove_player_session: Box::new({
                            let api = api.clone();
                            move |player_session_id| {
                                let api = api.clone();
                                async move {
                                    if let Err(err) = api
                                        .write()
                                        .await
                                        .remove_player_session(player_session_id)
                                        .await
                                    {
                                        error!("Player session remove error: {}", err);
                                    }
                                }
                                .boxed()
                            }
                        }),
                    };

                    // spawn the server process
                    tokio::spawn(run(
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
                    debug!("health check");
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
