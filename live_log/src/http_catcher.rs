// -----------------------------------------------------------------------------
// live_log.rs
// -----------------------------------------------------------------------------

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Router};
use std::io;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, Receiver};

use crate::parser::{LogParser, ParsedLine};

const MAX_JSON_LINES: usize = 80;
const MAX_CVAR_LINES: usize = 1028;

#[derive(Debug)]
pub struct LiveLog {
    port: u16,
    receiver: Option<Receiver<ParsedLine>>,
}

impl LiveLog {
    /// Creates a new LiveLog instance.
    ///
    /// Port 0 tells the operating system to select a free port.
    pub async fn new() -> io::Result<Self> {
        let parser = Arc::new(LogParser::new());

        // Let the OS select a free port.
        let listener = TcpListener::bind("0.0.0.0:0").await?;

        let port = listener.local_addr()?.port();

        println!(
            "\x1b[1;36m--- LiveLog listening on port {} ---\x1b[0m",
            port
        );

        let (tx, mut rx) = mpsc::channel::<String>(1000);
        let (parsed_sender, parsed_receiver) = mpsc::channel::<ParsedLine>(1000);

        let app = Router::new().route("/", post(handle_logs)).with_state(tx);

        // ---------------------------------------------------------------------
        // HTTP listener
        // ---------------------------------------------------------------------

        tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, app).await {
                eprintln!("[LIVE LOG] HTTP server stopped: {}", err);
            }
        });

        // ---------------------------------------------------------------------
        // Log processing
        // ---------------------------------------------------------------------

        tokio::spawn(async move {
            let mut assembler = LogAssembler::new();

            while let Some(body) = rx.recv().await {
                for line in body.lines() {
                    for message in assembler.process_line(line) {
                        let parsed = parser.parse(&message, "0.0.0.0:0".parse().unwrap());

                        if parsed_sender.send(parsed).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });

        Ok(Self {
            port,
            receiver: Some(parsed_receiver),
        })
    }

    /// Returns the TCP port this LiveLog instance is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn take_receiver(&mut self) -> Receiver<ParsedLine> {
        self.receiver
            .take()
            .expect("LiveLog receiver was already taken")
    }
}

// -----------------------------------------------------------------------------
// HTTP handler
// -----------------------------------------------------------------------------

async fn handle_logs(State(tx): State<mpsc::Sender<String>>, body: String) -> impl IntoResponse {
    if tx.send(body).await.is_err() {
        eprintln!("[LIVE LOG] Failed to send incoming log body to processor");
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

            // JSON completed normally
            if line.contains("JSON_END") {
                let buffer = self.json_buffer.take().unwrap();

                return vec![buffer.join("\n")];
            }

            // Safety limit reached
            if buffer.len() >= MAX_JSON_LINES {
                eprintln!("[JSON BUFFER LIMIT] -> {} lines", buffer.len());

                let buffer = self.json_buffer.take().unwrap();

                return buffer;
            }

            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // We are already collecting a CVar dump
        // ---------------------------------------------------------------------

        if let Some(buffer) = self.cvar_buffer.as_mut() {
            buffer.push(line.to_string());

            // CVar dump completed normally
            if line.contains("server cvars end") {
                let buffer = self.cvar_buffer.take().unwrap();

                return vec![buffer.join("\n")];
            }

            // Safety limit reached
            if buffer.len() >= MAX_CVAR_LINES {
                eprintln!("[CVAR BUFFER LIMIT] -> {} lines", buffer.len());

                let buffer = self.cvar_buffer.take().unwrap();

                return buffer;
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
