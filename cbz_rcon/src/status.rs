#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RconStatus {
    Disconnected,
    Connecting,
    Authenticated,
    Error,
}
