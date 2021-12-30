use argh::FromArgs;
use derive_more::Display;

#[derive(FromArgs, PartialEq, Debug, Display)]
#[argh(subcommand)]
pub enum Mode {
    #[display(fmt = "Client")]
    Client(ClientCommand),

    #[display(fmt = "Server")]
    Server(ServerCommand),

    #[display(fmt = "Dedicated")]
    Dedicated(DedicatedCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Run as client
#[argh(subcommand, name = "client")]
pub struct ClientCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Run as combined client and server
#[argh(subcommand, name = "server")]
pub struct ServerCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Run as dedicated server
#[argh(subcommand, name = "dedicated")]
pub struct DedicatedCommand {}

/// Echo client / server
#[derive(FromArgs, Debug)]
pub struct Options {
    /// the mode to run as
    #[argh(subcommand)]
    pub mode: Mode,
}

impl Options {
    pub fn is_client(&self) -> bool {
        matches!(self.mode, Mode::Client(_)) || matches!(self.mode, Mode::Server(_))
    }

    pub fn is_server(&self) -> bool {
        matches!(self.mode, Mode::Server(_)) || matches!(self.mode, Mode::Dedicated(_))
    }
}
