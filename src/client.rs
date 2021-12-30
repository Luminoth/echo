use tokio::io::{stdin, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::info;

pub async fn run(addr: impl AsRef<str>) -> anyhow::Result<()> {
    info!("Connecting to {} ...", addr.as_ref());
    let mut stream = TcpStream::connect(addr.as_ref()).await?;
    info!("Success!");

    // TODO: need to read and print what's echo'd back

    let mut stdin = BufReader::new(stdin()).lines();
    loop {
        let line = stdin.next_line().await?.unwrap();
        stream.write_all(line.as_bytes()).await?;
    }
}
