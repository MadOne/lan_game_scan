// -----------------------------------------------------------------------------
// live_log.rs
// -----------------------------------------------------------------------------

use std::io;
use std::sync::Arc;

use tokio::sync::mpsc::{self, Receiver};

use crate::_parser::types::ParsedLine;
use crate::game::Game;
use crate::log_receiver::{tcp::LogReceiverTcp, udp::LogReceiverUdp, LogReceiver};
use crate::parser::LogParser;

#[derive(Debug)]
pub struct LiveLog {
    port: u16,
    receiver: Option<Receiver<ParsedLine>>,
    log_receiver: LogReceiver,
    processor_task: tokio::task::JoinHandle<()>,
}

impl LiveLog {
    pub async fn new(game: Game) -> io::Result<Self> {
        // ---------------------------------------------------------------------
        // Create the appropriate log receiver for the game.
        // ---------------------------------------------------------------------

        let mut log_receiver = match &game {
            Game::Cs2 => LogReceiver::Tcp(LogReceiverTcp::new().await?),
            Game::Css | Game::Cs16 => LogReceiver::Udp(LogReceiverUdp::new().await?),
        };

        let port = log_receiver.port();

        let input_receiver = match log_receiver.take_receiver() {
            Some(receiver) => receiver,
            None => {
                log::error!(
                    target: "live_log",
                    "Failed to take log receiver channel"
                );

                log_receiver.stop().await;

                return Err(io::Error::other("LiveLog receiver channel already taken"));
            }
        };

        // ---------------------------------------------------------------------
        // Processor
        // ---------------------------------------------------------------------

        let (sender, receiver) = mpsc::channel::<ParsedLine>(1000);

        let parser = Arc::new(LogParser::new(game));

        let processor_task = tokio::spawn({
            let parser = Arc::clone(&parser);

            async move {
                process_logs(input_receiver, sender, parser).await;
            }
        });

        log::debug!(
            target: "live_log",
            "LiveLog started on port {}",
            port
        );

        Ok(Self {
            port,
            receiver: Some(receiver),
            log_receiver,
            processor_task,
        })
    }

    /// Returns the port the LiveLog receiver is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Takes the processed log receiver.
    ///
    /// This can only be called once.
    pub fn take_receiver(&mut self) -> Option<Receiver<ParsedLine>> {
        self.receiver.take()
    }

    pub async fn stop(self) {
        log::debug!(
            target: "live_log",
            "Stopping LiveLog on port {}",
            self.port
        );

        self.processor_task.abort();

        let _ = self.processor_task.await;

        self.log_receiver.stop().await;
    }
}

// -----------------------------------------------------------------------------
// Processor
// -----------------------------------------------------------------------------

async fn process_logs(
    mut receiver: Receiver<String>,
    sender: mpsc::Sender<ParsedLine>,
    parser: Arc<LogParser>,
) {
    while let Some(line) = receiver.recv().await {
        log::debug!(
            target: "live_log",
            "Parsing log line: {}",
            line
        );
        let parsed = parser.parse(&line);
        log::debug!(
            target: "live_log",
            "Parsed result: {:?}",
            parsed
        );
        if sender.send(parsed).await.is_err() {
            log::debug!(
                target: "live_log",
                "Parsed log receiver was dropped; stopping LiveLog processor"
            );

            return;
        }
    }

    log::debug!(
        target: "live_log",
        "Log receiver channel closed; stopping LiveLog processor"
    );
}
