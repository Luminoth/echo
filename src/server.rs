use std::net::SocketAddr;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tracing::info;

async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    silent: bool,
) -> anyhow::Result<()> {
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
    mut shutdown: mpsc::UnboundedReceiver<bool>,
) -> anyhow::Result<()> {
    info!("Listening on {}", addr.as_ref());
    let listener = TcpListener::bind(addr.as_ref()).await?;

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (stream, addr) = res?;

                info!("New connection from {}", addr);
                tokio::spawn(handle_connection(stream, addr, silent));
            },
            _ = shutdown.recv() => {
                info!("Received shutdown, exiting ...");
                return Ok(());
            }
        }
    }
}
