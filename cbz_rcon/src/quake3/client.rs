use std::net::SocketAddr;
use std::time::Duration;
use tokio::{net::UdpSocket, time::timeout};

use crate::RconError;

pub const QUAKE3_HEADER: [u8; 4] = [0xFF; 4];

pub struct Quake3RconClient {
    addr: SocketAddr,
    password: String,
    socket: Option<UdpSocket>,
    authenticated: bool,
}

impl Quake3RconClient {
    pub fn new(addr: SocketAddr, password: impl Into<String>) -> Self {
        println!("Quake3 Rcon Client created");
        Self {
            addr,
            password: password.into(),
            socket: None,
            authenticated: false,
        }
    }

    pub async fn connect(&mut self) -> Result<(), RconError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        self.socket = Some(socket);
        println!("Quake3 socket bound successfully");

        // Führt das Status-Kommando zur Validierung aus
        let response = self.command("status").await?;
        self.authenticated = self.is_valid_status_response(&response);

        if !self.authenticated {
            return Err(RconError::AuthenticationFailed);
        }

        Ok(())
    }

    /// Sendet ein RCON-Kommando ab, ohne auf eine Antwort zu warten.
    /// Perfekt für Befehle, die keine Ausgabe erzeugen (z.B. map_rotate oder kick),
    /// da hierbei kein 100ms Timeout-Fenster abgewartet werden muss.
    pub async fn command_no_response(&self, command: &str) -> Result<(), RconError> {
        let socket = self.socket.as_ref().ok_or(RconError::NotConnected)?;

        // 1. Paket zusammenbauen aus den struct-eigenen Variablen
        let payload = format!("rcon \"{}\" {}\n", self.password, command);
        let mut packet = QUAKE3_HEADER.to_vec();
        packet.extend_from_slice(payload.as_bytes());

        // 2. Paket an die gespeicherte Serveradresse senden
        socket.send_to(&packet, self.addr).await?;

        Ok(())
    }

    pub async fn command(&self, command: &str) -> Result<String, RconError> {
        let socket = self.socket.as_ref().ok_or(RconError::NotConnected)?;

        // 1. Nutzt command_no_response, um das Paket verlustfrei abzusetzen
        self.command_no_response(command).await?;

        println!("[QUAKE3 RCON] Command sent, waiting for response packets");

        let mut full_response = String::new();
        let mut buf = [0u8; 4096];

        // Das erste Paket darf bis zu 3 Sekunden brauchen
        let mut current_timeout = Duration::from_secs(3);

        loop {
            // 2. Asynchrones Lesen mit dynamischem Tokio-Timeout
            let read_result = timeout(current_timeout, socket.recv_from(&mut buf)).await;

            match read_result {
                Ok(Ok((amt, _))) => {
                    let response_bytes = &buf[..amt];

                    if response_bytes.starts_with(&QUAKE3_HEADER) {
                        // Überspringe Header [0xFF,0xFF,0xFF,0xFF] + "print\n" (10 Bytes)
                        let text_bytes =
                            if response_bytes.len() >= 10 && &response_bytes[4..10] == b"print\n" {
                                &response_bytes[10..]
                            } else {
                                &response_bytes[4..]
                            };

                        let text = String::from_utf8_lossy(text_bytes);
                        full_response.push_str(&text);
                    }

                    // Sobald Daten fließen, senken wir das Timeout für Folgepakete auf 100ms
                    current_timeout = Duration::from_millis(100);
                }
                Ok(Err(e)) => {
                    return Err(RconError::from(e));
                }
                Err(_) => {
                    // Timeout-Logik für das Ende des UDP-Streams
                    if full_response.is_empty() {
                        return Err(RconError::Timeout);
                    }
                    break;
                }
            }
        }

        Ok(full_response)
    }

    pub fn disconnect(&mut self) {
        self.socket = None;
        self.authenticated = false;
    }

    pub fn is_connected(&self) -> bool {
        self.authenticated && self.socket.is_some()
    }

    fn is_valid_status_response(&self, response: &str) -> bool {
        let response = response.trim();
        !response.is_empty() && !response.contains("Bad rconpassword")
    }
}
