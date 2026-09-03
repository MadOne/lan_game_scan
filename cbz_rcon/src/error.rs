#[derive(Debug)]
pub enum RconError {
    Connection(String),
    Timeout,
    AuthenticationFailed,
    NotConnected,
    InvalidPacket,
    InvalidUtf8,
    Io(String),
}

impl std::fmt::Display for RconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(e) => write!(f, "Connection error: {}", e),
            Self::Timeout => write!(f, "Connection timed out"),
            Self::AuthenticationFailed => write!(f, "Authentication failed"),
            Self::NotConnected => write!(f, "Not connected"),
            Self::InvalidPacket => write!(f, "Invalid RCON packet"),
            Self::InvalidUtf8 => write!(f, "Invalid UTF-8 in RCON response"),
            Self::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for RconError {}

impl From<std::io::Error> for RconError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
