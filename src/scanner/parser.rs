//parser.rs

use core::net::SocketAddr;

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Mutex as SyncMutex;
use std::time::{Instant, SystemTime};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{mpsc::*, Mutex};

use tokio::sync::mpsc;

use std::collections::{HashMap, VecDeque};

use crate::helper::pop_bytes;
use crate::server::GameServer;

pub struct Parser {
    udp_listener_receiver: Arc<Mutex<Receiver<(Vec<u8>, SocketAddr)>>>,
    sender_parsed: Arc<Sender<GameServer>>,
    udp_sender_sender: Arc<Sender<(Vec<u8>, SocketAddr)>>,
    pub receiver_parsed: Arc<Mutex<Receiver<GameServer>>>,
    ping: Arc<SyncMutex<HashMap<SocketAddr, Instant>>>,
}

impl Parser {
    pub fn new(
        udp_listener_receiver: Receiver<(Vec<u8>, SocketAddr)>,
        sender_udp: Sender<(Vec<u8>, SocketAddr)>,
        ping: Arc<SyncMutex<HashMap<SocketAddr, Instant>>>,
    ) -> Parser {
        let (parser_sender, parser_receiver) = mpsc::channel::<GameServer>(1_000);
        Parser {
            udp_listener_receiver: Arc::new(Mutex::new(udp_listener_receiver)),
            sender_parsed: Arc::new(parser_sender),
            udp_sender_sender: Arc::new(sender_udp),
            receiver_parsed: Arc::new(Mutex::new(parser_receiver)),
            ping: ping,
        }
    }

    pub async fn start(&mut self) {
        let a = self.udp_listener_receiver.clone();
        let b = self.sender_parsed.clone();
        let c = self.udp_sender_sender.clone();
        let d = self.ping.clone();

        tokio::spawn(async move {
            Parser::parse_response(a, b, c, d).await;
        });
    }

    pub async fn parse_response(
        listener_receiver: Arc<Mutex<Receiver<(Vec<u8>, SocketAddr)>>>,
        sender_processed: Arc<Sender<GameServer>>,
        sender_udp: Arc<Sender<(Vec<u8>, SocketAddr)>>,
        ping: Arc<SyncMutex<HashMap<SocketAddr, Instant>>>,
    ) {
        while let Some((response, addr)) = listener_receiver.lock().await.recv().await {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let response_length = response.len();

            // Quake protocol response
            if response_length >= 18 {
                if response[0..18] == *b"\xFF\xFF\xFF\xFFstatusResponse" {
                    let resp = String::from_utf8(response[20..response_length].to_vec()).unwrap();
                    let resp_split_newline: Vec<&str> = resp.split("\n").collect();
                    let players = resp_split_newline.len() - 2;
                    let info = resp_split_newline[0];
                    let d: Vec<&str> = info.split("\\").collect();
                    //println!("{:?}", &info);
                    println!("players: {players}");
                    let mut newmap: BTreeMap<&str, &str> = BTreeMap::new();
                    let mut i = 0;
                    while i < d.len() {
                        newmap.insert(d[i], d[i + 1]);
                        i += 2;
                    }
                    if false {
                        println!("{}: {:?}", addr, newmap);
                    }
                    let resp = GameServer {
                        socket_addr: addr,
                        hostname: Some(newmap.get("sv_hostname").unwrap().to_string()),
                        game: Some(newmap.get("gamename").unwrap().to_string()),
                        map: Some(newmap.get("mapname").unwrap().to_string()),
                        players: Some(players as u8),
                        players_max: Some(newmap.get("sv_maxclients").unwrap().parse().unwrap()),
                        query_port: Some(addr.port()),
                        rcon: None,
                        ping: Parser::calc_ping(&ping, addr),
                        last_update: Some(now as i64),
                        is_favorite: false,
                        bots: None,
                        has_password: false,
                        password: None,
                    };
                    let _ = sender_processed.send(resp).await;
                }
            }
            // Source challenge. Send back request with token
            if response_length == 9 {
                if response[0..5] == *b"\xFF\xFF\xFF\xFF\x41" {
                    let challenge = &response[5..];
                    let source_query: &[u8; 25] = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";
                    let myresp = [source_query.to_vec(), challenge.to_vec()].concat();
                    let _res = sender_udp.send((myresp, addr)).await;
                    ping.lock().unwrap().insert(addr, Instant::now());
                }
            }

            // Proper Source response
            if response_length > 5 {
                if response[0..5] == *b"\xFF\xFF\xFF\xFF\x49" {
                    //println!("Source response");

                    let resp_vec = response[5..].to_vec();
                    let mut payload: VecDeque<u8> = VecDeque::from(resp_vec.clone());
                    let val = pop_bytes(&mut payload, 1);
                    let server_protocol = val[0];
                    let val = pop_bytes(&mut payload, 0);
                    let server_name = String::from_utf8(val).unwrap();
                    let val = pop_bytes(&mut payload, 0);
                    let server_map = String::from_utf8(val).unwrap();
                    let val = pop_bytes(&mut payload, 0);
                    let server_folder = String::from_utf8(val).unwrap();
                    let val = pop_bytes(&mut payload, 0);
                    let server_game = String::from_utf8(val).unwrap();
                    let val = pop_bytes(&mut payload, 2);
                    let server_id = u16::from_ne_bytes([val[0], val[1]]);
                    let val = pop_bytes(&mut payload, 1);
                    let server_players = val[0];
                    let val = pop_bytes(&mut payload, 1);
                    let server_players_max = val[0];
                    let val = pop_bytes(&mut payload, 1);
                    let server_bots = val[0];
                    let val = pop_bytes(&mut payload, 1);
                    let server_type = val[0];
                    let val = pop_bytes(&mut payload, 1);
                    let server_environment = val[0];
                    let val = pop_bytes(&mut payload, 1);
                    let server_visibility = val[0];
                    let val = pop_bytes(&mut payload, 1);
                    let server_vac = val[0];
                    let val = pop_bytes(&mut payload, 0);
                    let server_version = String::from_utf8(val).unwrap();

                    // do not print
                    if false {
                        println!(
                            "
                            protocol: {server_protocol},
                            {server_name}, 
                            {server_map},
                            {server_folder},
                            {server_game},
                            {server_id},
                            players: {server_players},
                            max_players: {server_players_max},
                            bots: {server_bots},
                            type: {server_type},
                            environment: {server_environment},
                            visibility: {server_visibility},
                            vac: {server_vac},
                            version: {server_version}
                            "
                        );
                    }
                    let has_password = match server_visibility {
                        0 => false,
                        1 => true,
                        _ => false,
                    };

                    /*println!(
                        "{} visibility={} password={}",
                        addr, server_visibility, has_password
                    );*/

                    let game_name = match server_id {
                        10 => "CS".to_string(),
                        20 => "TFC".to_string(),
                        30 => "DoD".to_string(),
                        240 => "CSS".to_string(),
                        300 => "DoD:S".to_string(),
                        440 => "TF2".to_string(),
                        730 => "CS2".to_string(),
                        _ => server_game.clone(),
                    };
                    let resp = GameServer {
                        socket_addr: addr,
                        hostname: Some(server_name),
                        game: Some(game_name),
                        map: Some(server_map),
                        players: Some(server_players),
                        players_max: Some(server_players_max),
                        query_port: Some(addr.port()),
                        rcon: None,
                        ping: Parser::calc_ping(&ping, addr),
                        last_update: Some(now as i64),
                        is_favorite: false,
                        bots: Some(server_bots),
                        has_password: has_password,
                        password: None,
                    };
                    let _ = sender_processed.send(resp).await;
                }
            }
            // Obsolete GoldSource Response
            if response.len() > 5 {
                if response[0..5] == *b"\xFF\xFF\xFF\xFF\x6D" {
                    println!("GoldSource response");
                }
            }

            //gamespy
            if response.len() > 9 {
                //if response[0..9] == *b"\x5C\x68\x6F\x73\x74\x6E\x61\x6D\x65" { // \hostname
                if response[0..9] == *b"\x5C\x67\x61\x6D\x65\x6E\x61\x6D\x65"
                    || response[0..9] == *b"\x5C\x68\x6F\x73\x74\x6E\x61\x6D\x65"
                {
                    let b = String::from_utf8(response[1..response_length].to_vec()).unwrap();
                    let d: Vec<&str> = b.split("\\").collect();
                    println!("{:?}", &b);
                    let mut newmap: BTreeMap<&str, &str> = BTreeMap::new();
                    let mut i = 0;
                    while i < d.len() {
                        newmap.insert(d[i], d[i + 1]);
                        i += 2;
                    }
                    if false {
                        println!("{}: {:?}", addr, newmap);
                    }
                    let resp = GameServer {
                        socket_addr: addr,
                        hostname: Some(newmap.get("hostname").unwrap().to_string()),
                        game: Some(newmap.get("gamename").unwrap_or(&"").to_string()),
                        map: Some(newmap.get("mapname").unwrap().to_string()),
                        players: Some(newmap.get("numplayers").unwrap().parse().unwrap()),
                        players_max: Some(newmap.get("maxplayers").unwrap().parse().unwrap()),
                        query_port: Some(addr.port()),
                        rcon: None,
                        ping: Parser::calc_ping(&ping, addr),
                        last_update: Some(now as i64),
                        is_favorite: false,
                        bots: None,
                        has_password: false,
                        password: None,
                    };

                    let _ = sender_processed.send(resp).await;
                }
            }

            //sender_processed.send(resp);
        }
    }

    fn calc_ping(
        ping: &Arc<SyncMutex<HashMap<SocketAddr, Instant>>>,
        addr: SocketAddr,
    ) -> Option<u16> {
        let now = Instant::now();
        let mut ping_hashmap = ping.lock().unwrap();

        // 1. Check for a direct entry (Handshake/Challenge response)
        // We REMOVE it so we don't use a stale timestamp next time
        if let Some(start_time) = ping_hashmap.remove(&addr) {
            return Some(now.duration_since(start_time).as_millis().max(1) as u16);
        }

        // 2. Check for the Broadcast entry
        let bc_with_port =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), addr.port());
        if let Some(start_time) = ping_hashmap.get(&bc_with_port) {
            return Some(now.duration_since(*start_time).as_millis().max(1) as u16);
        }

        // 3. If neither found, return None (This helps the LAN filter know it's not ready)
        None
    }
}
