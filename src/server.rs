#[derive(Debug)]
pub struct Server {
    pub ip_port: String,
    pub hostname: String,
    pub game: String,
    pub map: String,
    pub players: String,
    pub players_max: String,
    pub query_port: u16,
    pub rcon: Option<String>,
}
