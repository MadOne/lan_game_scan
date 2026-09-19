// -----------------------------------------------------------------------------
// live_log.rs
// -----------------------------------------------------------------------------

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Router};
use std::io;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, Receiver};

use crate::log_receiver::log_assembler::LogAssembler;

#[derive(Debug)]
pub struct LogReceiverTcp {
    port: u16,
    receiver: Option<Receiver<String>>,
    http_task: tokio::task::JoinHandle<()>,
    assembler_task: tokio::task::JoinHandle<()>,
}

impl LogReceiverTcp {
    pub async fn new() -> io::Result<Self> {
        // Let the OS select a free port.
        let listener = TcpListener::bind("0.0.0.0:0").await?;
        let port = listener.local_addr()?.port();

        log::debug!(
            target: "live_log",
            "LogReceiverTcp listening on port {}",
            port
        );

        let (tx, mut rx) = mpsc::channel::<String>(1000);
        let (message_sender, message_receiver) = mpsc::channel::<String>(1000);

        let app = Router::new().route("/", post(handle_logs)).with_state(tx);

        // ---------------------------------------------------------------------
        // HTTP listener
        // ---------------------------------------------------------------------

        let http_task = tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, app).await {
                log::error!(
                    target: "live_log",
                    "HTTP server stopped: {}",
                    err
                );
            }
        });

        // ---------------------------------------------------------------------
        // Log processing
        // ---------------------------------------------------------------------

        let assembler_task = tokio::spawn(async move {
            let mut assembler = LogAssembler::new();

            while let Some(body) = rx.recv().await {
                for line in body.lines() {
                    for message in assembler.process_line(line) {
                        if message_sender.send(message).await.is_err() {
                            log::debug!(
                                target: "live_log",
                                "Parsed message receiver was dropped; stopping log assembler"
                            );

                            return;
                        }
                    }
                }
            }

            log::debug!(
                target: "live_log",
                "Log input channel closed; stopping log assembler"
            );
        });

        Ok(Self {
            port,
            receiver: Some(message_receiver),
            http_task,
            assembler_task,
        })
    }

    /// Returns the TCP port this LiveLog instance is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn take_receiver(&mut self) -> Option<Receiver<String>> {
        self.receiver.take()
    }

    pub async fn stop(self) {
        log::debug!(
            target: "live_log",
            "Stopping LogReceiverTcp on port {}",
            self.port
        );

        self.http_task.abort();
        self.assembler_task.abort();

        let _ = self.http_task.await;
        let _ = self.assembler_task.await;
    }
}

// -----------------------------------------------------------------------------
// HTTP handler
// -----------------------------------------------------------------------------

async fn handle_logs(State(tx): State<mpsc::Sender<String>>, body: String) -> impl IntoResponse {
    if tx.send(body).await.is_err() {
        log::error!(
            target: "live_log",
            "Failed to forward incoming log body to assembler"
        );

        return StatusCode::SERVICE_UNAVAILABLE;
    }

    StatusCode::OK
}

// -----------------------------------------------------------------------------
// Log assembler
// -----------------------------------------------------------------------------
