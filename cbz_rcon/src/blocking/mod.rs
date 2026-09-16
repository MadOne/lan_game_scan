use std::net::SocketAddr;
use tokio::runtime::{Builder, Runtime};

use crate::{RconClient, RconError, RconProtocol};

/// A synchronous, blocking RCON client wrapper.
///
/// Under the hood, this uses a lightweight, single-threaded Tokio runtime to
/// execute the underlying asynchronous RCON protocol operations synchronously.
pub struct BlockingRconClient {
    inner: RconClient,
    rt: Runtime,
}

impl BlockingRconClient {
    /// Creates a new blocking RCON client for the specified protocol.
    pub fn new(
        addr: SocketAddr,
        password: impl Into<String>,
        protocol: RconProtocol,
    ) -> Result<Self, RconError> {
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| RconError::Connection(error.to_string()))?;

        let inner = RconClient::new(addr, password.into(), protocol);

        Ok(Self { inner, rt })
    }

    /// Connects and authenticates with the remote server synchronously.
    pub fn connect(&mut self) -> Result<(), RconError> {
        self.rt.block_on(self.inner.connect())
    }

    /// Sends an RCON command and waits for the complete response string.
    pub fn command(&mut self, command: &str) -> Result<String, RconError> {
        self.rt.block_on(self.inner.command(command))
    }

    /// Disconnects from the server and closes the underlying stream/socket.
    pub fn disconnect(&mut self) {
        self.inner.disconnect();
    }

    /// Returns whether the client is currently connected.
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }
}
