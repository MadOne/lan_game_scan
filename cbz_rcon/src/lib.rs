mod error;
mod status;

pub mod goldsrc;
pub mod source;

use std::net::SocketAddr;

pub use error::RconError;
pub use source::SourceRconClient;
pub use status::RconStatus;

use crate::goldsrc::GoldSrcRconClient;

pub enum RconProtocol {
    GoldSrc,
    Source,
    Source2,
    Quake3,
    GameSpy,
    Unknown,
}

pub enum RconClient {
    Source(SourceRconClient),
    GoldSrc(GoldSrcRconClient),
    Source2(SourceRconClient),
}
impl RconClient {
    pub fn new(addr: SocketAddr, password: String, protocol: RconProtocol) -> Self {
        match protocol {
            RconProtocol::Source => RconClient::Source(SourceRconClient::new(addr, password)),
            RconProtocol::GoldSrc => RconClient::GoldSrc(GoldSrcRconClient::new(addr, password)),
            RconProtocol::Source2 => RconClient::Source(SourceRconClient::new(addr, password)), // ...
            RconProtocol::Quake3 => todo!(),
            RconProtocol::GameSpy => todo!(),
            RconProtocol::Unknown => todo!(),
        }
    }

    pub async fn connect(&mut self) -> Result<(), RconError> {
        match self {
            RconClient::Source(client) => client.connect().await,
            RconClient::GoldSrc(client) => client.connect().await,
            RconClient::Source2(client) => client.connect().await,
        }
    }

    pub async fn command(&mut self, command: &str) -> Result<String, RconError> {
        println!("[RconClient] command: {command}");
        match self {
            RconClient::Source(client) => client.command(command).await,
            RconClient::GoldSrc(client) => client.command(command).await,
            RconClient::Source2(client) => client.command(command).await,
        }
    }

    pub async fn command_no_response(&mut self, command: &str) -> Result<(), RconError> {
        match self {
            RconClient::Source(client) => client.command_no_response(command).await,
            RconClient::GoldSrc(client) => client.command_no_response(command).await,
            RconClient::Source2(client) => client.command_no_response(command).await,
        }
    }

    pub fn disconnect(&mut self) {
        match self {
            RconClient::Source(client) => client.disconnect(),
            RconClient::GoldSrc(client) => client.disconnect(),
            RconClient::Source2(client) => client.disconnect(),
        }
    }

    pub fn is_connected(&self) -> bool {
        match self {
            RconClient::Source(client) => client.is_connected(),
            RconClient::GoldSrc(client) => client.is_connected(),
            RconClient::Source2(client) => client.is_connected(),
        }
    }
}
