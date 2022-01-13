use std::future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::bail;
use chrono::Utc;
use futures_util::FutureExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{watch, RwLock},
    time,
};
use tracing::info;

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
    let mut buf = [0; 36];

    let mut t = 0;
    while t < 36 {
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
    let player_session_id = match read_player_session_id(&mut stream).await {
        Ok(player_session_id) => player_session_id,
        Err(_) => {
            info!("Connection from {} closed", addr);
            return Ok(());
        }
    };

    info!("Accepted player {}", player_session_id);
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
            info!("Removed player {}", player_session_id);

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

    info!("Starting session");
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
