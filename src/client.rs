use anyhow::bail;
use aws_sdk_gamelift::model::{
    GameSession, MatchmakingConfigurationStatus, MatchmakingTicket, Player, PlayerSession,
};
use tokio::{
    io::{stdin, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use tracing::info;
use uuid::Uuid;

use crate::gamelift::new_client;

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

async fn connect_server(
    addr: impl AsRef<str>,
    player_id: impl AsRef<str>,
    player_session_id: impl AsRef<str>,
) -> anyhow::Result<()> {
    let player_id = player_id.as_ref();
    let player_session_id = player_session_id.as_ref();

    info!(
        "{} connecting to {} ({}) ...",
        player_id,
        addr.as_ref(),
        player_session_id
    );
    let mut stream = TcpStream::connect(addr.as_ref()).await?;
    info!("Success!");

    // TODO: send player id

    // first thing we send is our player session id
    stream.write_u8(player_session_id.len() as u8).await?;
    stream.write_all(player_session_id.as_bytes()).await?;

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

pub async fn connect(addr: impl AsRef<str>, player_id: impl AsRef<str>) -> anyhow::Result<()> {
    connect_server(addr, &player_id, &player_id).await
}

fn print_game_session(game_session: &GameSession) {
    info!("Game Session: {:?}", game_session.game_session_id);
}

pub async fn create_gamelift(
    fleet_id: impl AsRef<str>,
    player_id: impl AsRef<str>,
    local: bool,
) -> anyhow::Result<()> {
    info!("Creating GameLift server ...");

    let client = new_client(local).await;

    let output = client
        .create_game_session()
        .fleet_id(fleet_id.as_ref().to_owned())
        .maximum_player_session_count(10)
        .send()
        .await?;

    let game_session = output.game_session.unwrap();
    print_game_session(&game_session);

    let game_session_id = game_session.game_session_id.unwrap();

    connect_gamelift(player_id, game_session_id, local).await
}

fn print_player_session(player_session: &PlayerSession) {
    info!("Player Session: {:?}", player_session.player_session_id);
}

pub async fn connect_gamelift(
    player_id: impl AsRef<str>,
    session_id: impl AsRef<str>,
    local: bool,
) -> anyhow::Result<()> {
    info!("Joining GameLift server ...");

    let client = new_client(local).await;

    let output = client
        .create_player_session()
        .game_session_id(session_id.as_ref().to_owned())
        .player_id(player_id.as_ref().to_owned())
        .send()
        .await?;

    let player_session = output.player_session.unwrap();
    print_player_session(&player_session);

    let player_session_id = player_session.player_session_id.unwrap();

    let connect_addr = format!(
        "{}:{}",
        player_session.ip_address.unwrap(),
        player_session.port.unwrap()
    );

    connect_server(connect_addr, player_id, player_session_id).await
}

fn print_ticket(ticket: &MatchmakingTicket) {
    info!("Ticket ID: {:?}", ticket.ticket_id);
    info!(
        "Status: {:?} (reason: {:?}) - {:?}",
        ticket.status, ticket.status_reason, ticket.status_message
    );
    info!("Estimated wait: {:?}", ticket.estimated_wait_time);
}

pub async fn find() -> anyhow::Result<()> {
    info!("Searching for server ...");

    let player_id = Uuid::new_v4().to_string();

    let client = new_client(false).await;

    let output = client
        .start_matchmaking()
        .configuration_name("echo")
        .players(Player::builder().player_id(&player_id).build())
        .send()
        .await?;

    let mut ticket = output.matchmaking_ticket.unwrap();
    print_ticket(&ticket);

    let connection_info;

    // poll until we find a match or timeout
    let ticket_id = ticket.ticket_id.unwrap();
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let output = client
            .describe_matchmaking()
            .ticket_ids(ticket_id.clone())
            .send()
            .await?;

        ticket = output.ticket_list.unwrap().first().unwrap().clone();

        let status = ticket.status.as_ref().unwrap();
        match status {
            MatchmakingConfigurationStatus::Cancelled
            | MatchmakingConfigurationStatus::Failed
            | MatchmakingConfigurationStatus::TimedOut
            | MatchmakingConfigurationStatus::Unknown(_) => {
                bail!("Find failed: {:?}", status);
            }
            MatchmakingConfigurationStatus::Queued
            | MatchmakingConfigurationStatus::Searching
            | MatchmakingConfigurationStatus::Placing => print_ticket(&ticket),
            MatchmakingConfigurationStatus::Completed => {
                connection_info = ticket.game_session_connection_info.clone();

                break;
            }
            _ => unreachable!(),
        }
    }

    info!("Found a match: {:?}", connection_info);

    let connection_info = connection_info.as_ref().unwrap();

    let connect_addr = Some(format!(
        "{}:{}",
        connection_info.ip_address.as_ref().unwrap(),
        connection_info.port.unwrap()
    ));

    let player_session_id = connection_info.matched_player_sessions().as_ref().unwrap()[0]
        .player_session_id
        .clone()
        .unwrap();

    connect_server(connect_addr.unwrap(), player_id, player_session_id).await?;

    Ok(())
}
