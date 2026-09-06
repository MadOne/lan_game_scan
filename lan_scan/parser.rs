use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;

use crate::scanner::{PlayerInfo, ServerUpdate};
use crate::server::ScannedServer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerProtocol {
    GoldSrc,
    Source,
    Source2,
    Quake3,
    GameSpy,
    Unknown,
}

#[derive(Debug)]
pub enum ParseResult {
    /// Server payload parsed successfully
    Update(ServerUpdate),
    /// Challenge token received from server (4 bytes)
    Challenge([u8; 4]),
    /// Waiting for remaining split fragments to complete reassembly
    PartialSplit,
    /// Unrecognized packet format or corrupted data
    Ignored,
}

/// Buffer for reassembling multi-packet UDP responses
#[derive(Default, Debug)]
pub struct SplitBuffer {
    pub total: u8,
    pub packets: BTreeMap<u8, Vec<u8>>,
}

/// Main entry point called directly by scanner.rs upon receiving a UDP packet.
pub fn parse(
    data: &mut [u8],
    addr: SocketAddr,
    ping_ms: Option<u16>,
    split_cache: &mut HashMap<(SocketAddr, u32), SplitBuffer>,
) -> ParseResult {
    //println!("Parsing data: {:?} from addr: {:?}", data, addr);

    // 0. Handle Source/GoldSrc Multi-packet splits (0xFEFFFFFF / -2)
    let payload = if data.len() > 12 && data.starts_with(b"\xFE\xFF\xFF\xFF") {
        match handle_split_packet(data, addr, split_cache) {
            Some(reassembled) => reassembled,
            None => return ParseResult::PartialSplit,
        }
    } else {
        data.to_vec()
    };

    let len = payload.len();

    // 1. Quake3 / CoD status response
    if len >= 18 && payload.starts_with(b"\xFF\xFF\xFF\xFFstatusResponse") {
        //println!("Q3 Query catched");
        if let Some(update) = parse_quake3(&payload[20..], addr, ping_ms) {
            //println!("Successfully parsed Q3 Server: {:?}", update);
            return ParseResult::Update(update);
        }
        return ParseResult::Ignored;
    }

    // 2. Source / GoldSrc challenge response ('A' / 0x41)
    if len >= 9 && payload.starts_with(b"\xFF\xFF\xFF\xFF\x41") {
        if let Ok(challenge_bytes) = payload[5..9].try_into() {
            return ParseResult::Challenge(challenge_bytes);
        }
        return ParseResult::Ignored;
    }

    // 3. Source A2S_INFO response ('I' / 0x49)
    if len > 5 && payload.starts_with(b"\xFF\xFF\xFF\xFF\x49") {
        if let Some(server) = parse_a2s_info(&payload[5..], addr, ping_ms) {
            return ParseResult::Update(ServerUpdate::FullServer(server));
        }
        return ParseResult::Ignored;
    }

    // 4. GoldSrc Legacy INFO response ('m' / 0x6D)
    if len > 5 && payload.starts_with(b"\xFF\xFF\xFF\xFF\x6D") {
        if let Some(server) = parse_goldsrc_info(&payload[5..], addr, ping_ms) {
            return ParseResult::Update(ServerUpdate::FullServer(server));
        }
        return ParseResult::Ignored;
    }

    // 5. Source A2S_PLAYER response ('D' / 0x44)
    if len > 5 && payload.starts_with(b"\xFF\xFF\xFF\xFF\x44") {
        if let Some(players) = parse_a2s_player(&payload[5..]) {
            return ParseResult::Update(ServerUpdate::PlayerList { addr, players });
        }
        return ParseResult::Ignored;
    }

    // 6. GameSpy protocol (1, 2 & 3)
    if len > 4
        && (payload.starts_with(b"\\gamename")
            || payload.starts_with(b"\\hostname")
            || payload[0] == 0x00)
    {
        let gs_payload = if payload.starts_with(b"\\") {
            &payload[1..]
        } else {
            &payload[4..]
        };
        if let Some(server) = parse_gamespy(gs_payload, addr, ping_ms) {
            return ParseResult::Update(ServerUpdate::FullServer(server));
        }
        return ParseResult::Ignored;
    }

    ParseResult::Ignored
}

// --- PRIVATE PROTOCOL PARSERS ---

fn handle_split_packet(
    payload: &[u8],
    addr: SocketAddr,
    split_cache: &mut HashMap<(SocketAddr, u32), SplitBuffer>,
) -> Option<Vec<u8>> {
    if payload.len() < 12 {
        return None;
    }

    let request_id = u32::from_le_bytes(payload[4..8].try_into().ok()?);
    let total = payload[8];
    let number = payload[9];

    let entry = split_cache.entry((addr, request_id)).or_default();
    entry.total = total;
    entry.packets.insert(number, payload[12..].to_vec());

    if entry.packets.len() == total as usize {
        let mut full_payload = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Standard uncompressed header
        for (_, pkt_data) in entry.packets.iter() {
            full_payload.extend_from_slice(pkt_data);
        }
        split_cache.remove(&(addr, request_id));
        Some(full_payload)
    } else {
        None
    }
}

fn parse_a2s_info(
    mut payload: &[u8],
    addr: SocketAddr,
    ping: Option<u16>,
) -> Option<ScannedServer> {
    if payload.is_empty() {
        return None;
    }
    let _protocol = payload[0];
    payload = &payload[1..];

    let name = read_cstring(&mut payload)?;
    let map = read_cstring(&mut payload)?;
    let _folder = read_cstring(&mut payload)?;
    let game = read_cstring(&mut payload)?;

    if payload.len() < 2 {
        return None;
    }
    let server_id = u16::from_le_bytes([payload[0], payload[1]]);
    payload = &payload[2..];

    if payload.len() < 3 {
        return None;
    }
    let players = payload[0];
    let max_players = payload[1];
    let bots = payload[2];
    payload = &payload[3..];

    // Skip environment + server_type
    if payload.len() < 2 {
        return None;
    }
    payload = &payload[2..];

    let visibility = if !payload.is_empty() { payload[0] } else { 0 };

    let game_name = match server_id {
        10 => "CS".to_string(),
        20 => "TFC".to_string(),
        30 => "DoD".to_string(),
        240 => "CSS".to_string(),
        300 => "DoD:S".to_string(),
        440 => "TF2".to_string(),
        730 => "CS2".to_string(),
        _ => game,
    };

    let protocol = match server_id {
        730 => ServerProtocol::Source2,
        id if id < 200 => ServerProtocol::GoldSrc,
        _ => ServerProtocol::Source,
    };

    Some(ScannedServer {
        socket_addr: addr,
        hostname: Some(name),
        game: Some(game_name),
        map: Some(map),
        players: Some(players),
        players_max: Some(max_players),
        players_list: vec![],
        query_port: Some(addr.port()),
        ping,
        bots: Some(bots),
        has_password: visibility == 1,
        password: None,
        protocol,
    })
}

fn parse_goldsrc_info(
    mut payload: &[u8],
    addr: SocketAddr,
    ping: Option<u16>,
) -> Option<ScannedServer> {
    let _ip_addr = read_cstring(&mut payload)?;
    let name = read_cstring(&mut payload)?;
    let map = read_cstring(&mut payload)?;
    let _folder = read_cstring(&mut payload)?;
    let game = read_cstring(&mut payload)?;

    if payload.len() < 2 {
        return None;
    }
    let players = payload[0];
    let max_players = payload[1];

    Some(ScannedServer {
        socket_addr: addr,
        hostname: Some(name),
        game: Some(game),
        map: Some(map),
        players: Some(players),
        players_max: Some(max_players),
        players_list: vec![],
        query_port: Some(addr.port()),
        ping,
        bots: None,
        has_password: false,
        password: None,
        protocol: ServerProtocol::GoldSrc,
    })
}

fn parse_a2s_player(mut payload: &[u8]) -> Option<Vec<PlayerInfo>> {
    if payload.is_empty() {
        return None;
    }

    let player_count = payload[0] as usize;
    payload = &payload[1..];

    let mut players = Vec::with_capacity(player_count);

    for _ in 0..player_count {
        if payload.is_empty() {
            break;
        }

        let index = payload[0];
        payload = &payload[1..];

        let name = read_cstring(&mut payload)?;

        if payload.len() < 8 {
            break;
        }
        let score = i32::from_le_bytes(payload[..4].try_into().ok()?);
        let duration_secs = f32::from_le_bytes(payload[4..8].try_into().ok()?);
        payload = &payload[8..];

        players.push(PlayerInfo {
            name,
            score,
            ping: None,
            duration_secs: Some(duration_secs),
            index: Some(index),
            team: None,
            skin: None,
            is_bot: false,
        });
    }

    Some(players)
}

fn parse_quake3(payload: &[u8], addr: SocketAddr, ping: Option<u16>) -> Option<ServerUpdate> {
    let resp = String::from_utf8_lossy(payload);
    let lines: Vec<&str> = resp.split('\n').collect();
    if lines.is_empty() {
        return None;
    }

    let info = lines[0];
    let d: Vec<&str> = info.split('\\').collect();

    let mut newmap: BTreeMap<&str, &str> = BTreeMap::new();
    let mut i = 0;
    while i + 1 < d.len() {
        newmap.insert(d[i], d[i + 1]);
        i += 2;
    }

    let mut players_list = Vec::new();
    for line in lines[1..]
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
    {
        if let Some(player) = parse_quake3_player_line(line) {
            players_list.push(player);
        }
    }

    let players_count = players_list.len() as u8;

    Some(ServerUpdate::FullServer(ScannedServer {
        socket_addr: addr,
        hostname: newmap.get("sv_hostname").map(|s| s.to_string()),
        game: newmap.get("gamename").map(|s| s.to_string()),
        map: newmap.get("mapname").map(|s| s.to_string()),
        players: Some(players_count),
        players_max: newmap.get("sv_maxclients").and_then(|s| s.parse().ok()),
        players_list,
        query_port: Some(addr.port()),
        ping,
        bots: None,
        has_password: newmap.get("g_needpass").map_or(false, |v| *v == "1"),
        password: None,
        protocol: ServerProtocol::Quake3,
    }))
}

fn parse_quake3_player_line(line: &str) -> Option<PlayerInfo> {
    let mut parts = line.splitn(3, ' ');

    let score = parts.next()?.parse::<i32>().ok()?;
    let ping_val = parts.next()?.parse::<u16>().ok()?;
    let raw_name = parts.next()?.trim();

    let name = raw_name
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw_name)
        .to_string();

    let is_bot = ping_val == 0 || ping_val == 999;

    Some(PlayerInfo {
        name,
        score,
        ping: Some(ping_val),
        duration_secs: None,
        index: None,
        team: None,
        skin: None,
        is_bot,
    })
}

fn parse_gamespy(payload: &[u8], addr: SocketAddr, ping: Option<u16>) -> Option<ScannedServer> {
    let text = String::from_utf8_lossy(payload);

    // Split on backslash and discard empty trailing/leading tokens
    let tokens: Vec<&str> = text
        .split('\\')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut map: BTreeMap<&str, &str> = BTreeMap::new();
    let mut i = 0;

    // Safely insert key-value pairs
    while i + 1 < tokens.len() {
        let key = tokens[i];
        let val = tokens[i + 1];

        // Ignore the GameSpy trailing delimiter "final"
        if key != "final" {
            map.insert(key, val);
        }
        i += 2;
    }

    Some(ScannedServer {
        socket_addr: addr,
        hostname: map
            .get("hostname")
            .or_else(|| map.get("servername"))
            .map(|s| s.to_string()),
        game: map
            .get("gamename")
            .or_else(|| map.get("game"))
            .map(|s| s.to_string()),
        map: map
            .get("mapname")
            .or_else(|| map.get("map"))
            .map(|s| s.to_string()),
        players: map.get("numplayers").and_then(|s| s.parse().ok()),
        players_max: map.get("maxplayers").and_then(|s| s.parse().ok()),
        players_list: vec![],
        query_port: Some(addr.port()),
        ping,
        bots: map.get("numbots").and_then(|s| s.parse().ok()),
        has_password: map
            .get("password")
            .map_or(false, |&v| v == "1" || v == "true"),
        password: None,
        protocol: ServerProtocol::GameSpy,
    })
}

fn read_cstring(cursor: &mut &[u8]) -> Option<String> {
    let null_pos = cursor.iter().position(|&b| b == 0)?;
    let s = String::from_utf8_lossy(&cursor[..null_pos]).into_owned();
    *cursor = &cursor[null_pos + 1..];
    Some(s)
}

use std::collections::VecDeque;

/// Extracts `count` bytes from the front of a `VecDeque<u8>`.
/// If `count` is 0, extracts until the first null byte (`0x00`) or end of deque.
pub fn pop_bytes(payload: &mut VecDeque<u8>, count: usize) -> Vec<u8> {
    let mut result = Vec::new();

    if count > 0 {
        for _ in 0..count {
            if let Some(b) = payload.pop_front() {
                result.push(b);
            } else {
                break;
            }
        }
    } else {
        while let Some(b) = payload.pop_front() {
            if b == 0 {
                break;
            }
            result.push(b);
        }
    }

    result
}
