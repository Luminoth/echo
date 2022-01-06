use std::future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::bail;
use futures_util::FutureExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{watch, RwLock},
};
use tracing::info;

type AcceptPlayerSessionOutput = Pin<Box<dyn future::Future<Output = ()> + Send>>;
type AcceptPlayerSession = Box<dyn Fn(String) -> AcceptPlayerSessionOutput + Send + Sync>;

type RemovePlayerSessionOutput = Pin<Box<dyn future::Future<Output = ()> + Send>>;
type RemovePlayerSession = Box<dyn Fn(String) -> RemovePlayerSessionOutput + Send + Sync>;

pub struct ServerCallbacks {
    pub accept_player_session: AcceptPlayerSession,
    pub remove_player_session: RemovePlayerSession,
}

impl Default for ServerCallbacks {
    fn default() -> Self {
        Self {
            accept_player_session: Box::new(|_| future::ready(()).boxed()),
            remove_player_session: Box::new(|_| future::ready(()).boxed()),
        }
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
    callbacks: Arc<RwLock<ServerCallbacks>>,
) -> anyhow::Result<()> {
    let player_session_id = match read_player_session_id(&mut stream).await {
        Ok(player_session_id) => player_session_id,
        Err(_) => {
            info!("Connection from {} closed", addr);
            return Ok(());
        }
    };

    info!("Accepted player {}", player_session_id);
    (callbacks.write().await.accept_player_session)(player_session_id).await;

    let mut buf = [0; 1024];
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            info!("Connection from {} closed", addr);
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
    ready: watch::Sender<bool>,
    mut shutdown: watch::Receiver<bool>,
    callbacks: ServerCallbacks,
) -> anyhow::Result<()> {
    let callbacks = Arc::new(RwLock::new(callbacks));

    info!("Listening on {}", addr.as_ref());
    let listener = TcpListener::bind(addr.as_ref()).await?;

    ready.send(true)?;

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (stream, addr) = res?;

                info!("New connection from {}", addr);
                tokio::spawn(handle_connection(stream, addr, silent, callbacks.clone()));

            },
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
