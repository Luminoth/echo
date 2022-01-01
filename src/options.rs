use argh::FromArgs;
use derive_more::Display;

#[derive(FromArgs, PartialEq, Debug, Display)]
#[argh(subcommand)]
pub enum Mode {
    #[display(fmt = "Connect")]
    Connect(ConnectCommand),

    #[display(fmt = "Find")]
    Find(FindCommand),

    #[display(fmt = "Server")]
    Server(ServerCommand),

    #[display(fmt = "Dedicated")]
    Dedicated(DedicatedCommand),

    #[display(fmt = "GameLift")]
    GameLift(GameLiftCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Connect client to a dedicated server
#[argh(subcommand, name = "connect")]
pub struct ConnectCommand {
    /// host to connect to
    #[argh(option, default = "default_host()")]
    pub host: String,

    /// port to connect to
    #[argh(option, default = "default_port()")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

#[derive(FromArgs, PartialEq, Debug)]
/// Search for a server to connect to
#[argh(subcommand, name = "find")]
pub struct FindCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Run as combined client and server
#[argh(subcommand, name = "server")]
pub struct ServerCommand {
    /// port to connect to
    #[argh(option, default = "default_port()")]
    pub port: u16,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Run as dedicated server
#[argh(subcommand, name = "dedicated")]
pub struct DedicatedCommand {
    /// port to connect to
    #[argh(option, default = "default_port()")]
    pub port: u16,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Run as dedicated server on AWS GameLift
#[argh(subcommand, name = "gamelift")]
pub struct GameLiftCommand {
    /// port to connect to
    #[argh(option, default = "default_port()")]
    pub port: u16,
}

fn default_port() -> u16 {
    8065
}

/// Echo client / server
#[derive(FromArgs, Debug)]
pub struct Options {
    /// the mode to run as
    #[argh(subcommand)]
    pub mode: Mode,

    /// enable tokio tracing
    #[argh(switch)]
    pub tracing: bool,
}

impl Options {
    pub fn connect_addr(&self) -> String {
        match &self.mode {
            Mode::Connect(cmd) => format!("{}:{}", cmd.host, cmd.port),
            Mode::Server(cmd) => format!("127.0.0.1:{}", cmd.port),
            _ => unreachable!(),
        }
    }

    pub fn server_addr(&self) -> String {
        match &self.mode {
            Mode::Server(cmd) => format!("127.0.0.1:{}", cmd.port),
            Mode::Dedicated(cmd) => format!("0.0.0.0:{}", cmd.port),
            _ => unreachable!(),
        }
    }
}
