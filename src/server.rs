use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::info;

async fn handle_connection(
    mut socket: TcpStream,
    addr: SocketAddr,
    _silent: bool,
) -> anyhow::Result<()> {
    let mut buf = [0; 1024];
    loop {
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            info!("Connection from {} closed", addr);
            return Ok(());
        }

        //if !silent {
        info!("Read from {}: {}", addr, std::str::from_utf8(&buf[0..n])?);
        //}

        socket.write_all(&buf[0..n]).await?;
    }
}

pub async fn run(addr: impl AsRef<str>, silent: bool) -> anyhow::Result<()> {
    info!("Listening on {}", addr.as_ref());
    let listener = TcpListener::bind(addr.as_ref()).await?;

    loop {
        let (socket, addr) = listener.accept().await?;
        info!("New connection from {}", addr);

        tokio::spawn(handle_connection(socket, addr, silent));
    }
}
