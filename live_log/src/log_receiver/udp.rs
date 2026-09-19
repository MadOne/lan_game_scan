use std::io;

use tokio::net::UdpSocket;
use tokio::sync::mpsc::{self, Receiver};

use crate::log_receiver::log_assembler::LogAssembler;

const UDP_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug)]
pub struct LogReceiverUdp {
    port: u16,
    receiver: Option<Receiver<String>>,
    task: tokio::task::JoinHandle<()>,
}

impl LogReceiverUdp {
    pub async fn new() -> io::Result<Self> {
        // Let the OS select a free port.
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let port = socket.local_addr()?.port();

        log::debug!(
            target: "live_log",
            "LogReceiverUdp listening on port {}",
            port
        );

        let (sender, receiver) = mpsc::channel::<String>(1000);

        // ---------------------------------------------------------------------
        // UDP listener
        // ---------------------------------------------------------------------

        let task = tokio::spawn(async move {
            let mut buffer = vec![0u8; UDP_BUFFER_SIZE];
            let mut assembler = LogAssembler::new();

            loop {
                let (len, addr) = match socket.recv_from(&mut buffer).await {
                    Ok(result) => result,
                    Err(err) => {
                        log::error!(
                            target: "live_log",
                            "UDP log receiver failed: {}",
                            err
                        );

                        return;
                    }
                };

                log::trace!(
                    target: "live_log",
                    "Received {} bytes from {}",
                    len,
                    addr
                );

                let mut data = &buffer[..len];

                // Source UDP log packet header.
                if data.starts_with(&[0xFF, 0xFF, 0xFF, 0xFF]) {
                    data = &data[4..];
                }

                // Source log payload header.
                if data.starts_with(b"log ") {
                    data = &data[4..];
                }

                // Source log packets are NUL terminated.
                data = data.strip_suffix(&[0]).unwrap_or(data);

                let body = String::from_utf8_lossy(data);

                for line in body.lines() {
                    let line = line.trim();

                    if line.is_empty() {
                        continue;
                    }

                    for message in assembler.process_line(line) {
                        if sender.send(message).await.is_err() {
                            log::debug!(
                                target: "live_log",
                                "Log receiver channel was dropped; stopping UDP receiver"
                            );

                            return;
                        }
                    }
                }
            }
        });

        Ok(Self {
            port,
            receiver: Some(receiver),
            task,
        })
    }

    /// Returns the UDP port this LogLive instance is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn take_receiver(&mut self) -> Option<Receiver<String>> {
        self.receiver.take()
    }

    pub async fn stop(self) {
        log::debug!(
            target: "live_log",
            "Stopping LogReceiverUdp on port {}",
            self.port
        );

        self.task.abort();

        let _ = self.task.await;
    }
}
