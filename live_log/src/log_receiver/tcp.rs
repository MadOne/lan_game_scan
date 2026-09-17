// -----------------------------------------------------------------------------
// live_log.rs
// -----------------------------------------------------------------------------

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Router};
use std::io;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, Receiver};

const MAX_JSON_LINES: usize = 80;
const MAX_CVAR_LINES: usize = 1028;

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

pub struct LogAssembler {
    json_buffer: Option<Vec<String>>,
    cvar_buffer: Option<Vec<String>>,
}

impl LogAssembler {
    pub fn new() -> Self {
        Self {
            json_buffer: None,
            cvar_buffer: None,
        }
    }

    pub fn process_line(&mut self, line: &str) -> Vec<String> {
        let line = line.trim();

        if line.is_empty() {
            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // We are already collecting JSON
        // ---------------------------------------------------------------------

        if let Some(buffer) = self.json_buffer.as_mut() {
            buffer.push(line.to_string());

            if line.contains("JSON_END") {
                let buffer = self.json_buffer.take();

                return match buffer {
                    Some(buffer) => vec![buffer.join("\n")],
                    None => Vec::new(),
                };
            }

            if buffer.len() >= MAX_JSON_LINES {
                let line_count = buffer.len();

                self.json_buffer = None;

                log::warn!(
                    target: "live_log",
                    "JSON buffer exceeded {} lines; discarding incomplete block ({} lines)",
                    MAX_JSON_LINES,
                    line_count
                );

                return Vec::new();
            }

            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // We are already collecting a CVar dump
        // ---------------------------------------------------------------------

        if let Some(buffer) = self.cvar_buffer.as_mut() {
            buffer.push(line.to_string());

            if line.contains("server cvars end") {
                let buffer = self.cvar_buffer.take();

                return match buffer {
                    Some(buffer) => vec![buffer.join("\n")],
                    None => Vec::new(),
                };
            }

            if buffer.len() >= MAX_CVAR_LINES {
                let line_count = buffer.len();

                self.cvar_buffer = None;

                log::warn!(
                    target: "live_log",
                    "CVar buffer exceeded {} lines; discarding incomplete block ({} lines)",
                    MAX_CVAR_LINES,
                    line_count
                );

                return Vec::new();
            }

            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // Start of a new JSON block
        // ---------------------------------------------------------------------

        if line.contains("JSON_BEGIN") {
            let mut buffer = Vec::with_capacity(MAX_JSON_LINES);
            buffer.push(line.to_string());

            self.json_buffer = Some(buffer);

            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // Start of a new CVar dump
        // ---------------------------------------------------------------------

        if line.contains("server cvars start") {
            let mut buffer = Vec::with_capacity(MAX_CVAR_LINES);
            buffer.push(line.to_string());

            self.cvar_buffer = Some(buffer);

            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // Normal log line
        // ---------------------------------------------------------------------

        vec![line.to_string()]
    }
}
