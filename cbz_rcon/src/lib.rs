mod error;
mod status;

pub mod goldsrc;
pub mod quake3;
pub mod source;

use std::net::SocketAddr;

pub use error::RconError;
pub use source::SourceRconClient;
pub use status::RconStatus;

use crate::{goldsrc::GoldSrcRconClient, quake3::Quake3RconClient};

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
    Quake3(Quake3RconClient),
}

impl RconClient {
    pub fn new(addr: SocketAddr, password: String, protocol: RconProtocol) -> Self {
        match protocol {
            RconProtocol::Source => RconClient::Source(SourceRconClient::new(addr, password)),
            RconProtocol::GoldSrc => RconClient::GoldSrc(GoldSrcRconClient::new(addr, password)),
            RconProtocol::Source2 => RconClient::Source2(SourceRconClient::new(addr, password)),
            RconProtocol::Quake3 => RconClient::Quake3(Quake3RconClient::new(addr, password)),
            RconProtocol::GameSpy => {
                unimplemented!("GameSpy protocol is not supported yet by LAN GAME SCAN")
            }
            RconProtocol::Unknown => unimplemented!("Unknown protocol cannot create an RconClient"),
        }
    }

    pub async fn connect(&mut self) -> Result<(), RconError> {
        match self {
            RconClient::Source(client) => client.connect().await,
            RconClient::GoldSrc(client) => client.connect().await,
            RconClient::Source2(client) => client.connect().await,
            RconClient::Quake3(client) => client.connect().await,
        }
    }

    pub async fn command(&mut self, command: &str) -> Result<String, RconError> {
        println!("[RconClient] command: {command}");
        match self {
            RconClient::Source(client) => client.command(command).await,
            RconClient::GoldSrc(client) => client.command(command).await,
            RconClient::Source2(client) => client.command(command).await,
            RconClient::Quake3(client) => client.command(command).await,
        }
    }

    pub async fn command_no_response(&mut self, command: &str) -> Result<(), RconError> {
        match self {
            RconClient::Source(client) => client.command_no_response(command).await,
            RconClient::GoldSrc(client) => client.command_no_response(command).await,
            RconClient::Source2(client) => client.command_no_response(command).await,
            RconClient::Quake3(client) => client.command_no_response(command).await,
        }
    }

    pub fn disconnect(&mut self) {
        match self {
            RconClient::Source(client) => client.disconnect(),
            RconClient::GoldSrc(client) => client.disconnect(),
            RconClient::Source2(client) => client.disconnect(),
            RconClient::Quake3(client) => client.disconnect(),
        }
    }

    pub fn is_connected(&self) -> bool {
        match self {
            RconClient::Source(client) => client.is_connected(),
            RconClient::GoldSrc(client) => client.is_connected(),
            RconClient::Source2(client) => client.is_connected(),
            RconClient::Quake3(client) => client.is_connected(),
        }
    }
}
