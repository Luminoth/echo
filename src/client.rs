use anyhow::bail;
use tokio::io::{stdin, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::info;

#[derive(Debug)]
enum Event {
    Input(String),
    Message(String),
}

async fn handle_event(event: Event, stream: &mut TcpStream) -> anyhow::Result<()> {
    match event {
        Event::Input(line) => stream.write_all(line.as_bytes()).await?,
        Event::Message(message) => {
            info!("Read: {}", message);
        }
    }

    Ok(())
}

pub async fn run(addr: impl AsRef<str>) -> anyhow::Result<()> {
    info!("Connecting to {} ...", addr.as_ref());
    let mut stream = TcpStream::connect(addr.as_ref()).await?;
    info!("Success!");

    let mut buf = [0; 1024];
    let mut stdin = BufReader::new(stdin()).lines();
    loop {
        let event = tokio::select! {
            line = stdin.next_line() => Event::Input(line?.unwrap()),
            n = stream.read(&mut buf) => {
                let n = n?;
                if n == 0 {
                    bail!("Server disconnected!");
                }
                Event::Message(std::str::from_utf8(&buf[0..n])?.to_string())
            },
            else => bail!("Unhandled event!"),
        };

        handle_event(event, &mut stream).await?;
    }
}
