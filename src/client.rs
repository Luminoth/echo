use anyhow::bail;
use aws_sdk_gamelift::model::{AttributeValue, Player};
use aws_sdk_gamelift::Client;
use tokio::io::{stdin, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::info;
use uuid::Uuid;

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

pub async fn connect(addr: impl AsRef<str>) -> anyhow::Result<()> {
    info!("Connecting to {} ...", addr.as_ref());
    let mut stream = TcpStream::connect(addr.as_ref()).await?;
    info!("Success!");

    let mut buf = [0; 1024];
    let mut stdin = BufReader::new(stdin()).lines();
    loop {
        let event = tokio::select! {
            line = stdin.next_line() => {
                match line? {
                    Some(line) => Event::Input(line),
                    None => return Ok(()),
                }
            },
            n = stream.read(&mut buf) => {
                match n? {
                    0 => bail!("Server disconnected!"),
                    n => Event::Message(std::str::from_utf8(&buf[0..n])?.to_string()),
                }
            },
            else => bail!("Unhandled event!"),
        };

        handle_event(event, &mut stream).await?;
    }
}

pub async fn find() -> anyhow::Result<()> {
    info!("Searching for server ...");

    let shared_config = aws_config::from_env().load().await;

    let player_id = Uuid::new_v4();

    let client = Client::new(&shared_config);
    let output = client
        .start_matchmaking()
        .configuration_name("echo")
        .players(
            Player::builder()
                .player_id(player_id.to_string())
                .player_attributes("skill", AttributeValue::builder().n(0.0).build())
                .build(),
        )
        .send()
        .await?;

    let ticket = output.matchmaking_ticket.unwrap();
    info!("Ticket ID: {:?}", ticket.ticket_id);
    info!(
        "Status: {:?} (reason: {:?}) - {:?}",
        ticket.status, ticket.status_reason, ticket.status_message
    );
    info!("Estimated wait: {:?}", ticket.estimated_wait_time);

    Ok(())
}
