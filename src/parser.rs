use core::net::SocketAddr;

use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{mpsc::*, Mutex};

use tokio::sync::mpsc;

use std::collections::VecDeque;

use crate::misc::pop_bytes;
use crate::server::Server;

pub struct Parser {
    udp_listener_receiver: Arc<Mutex<Receiver<(Vec<u8>, SocketAddr)>>>,
    sender_parsed: Arc<Sender<Server>>,
    udp_sender_sender: Arc<Sender<(Vec<u8>, SocketAddr)>>,
    pub receiver_parsed: Arc<Mutex<Receiver<Server>>>,
}

impl Parser {
    pub fn new(
        udp_listener_receiver: Receiver<(Vec<u8>, SocketAddr)>,
        sender_udp: Sender<(Vec<u8>, SocketAddr)>,
    ) -> Parser {
        let (parser_sender, parser_receiver) = mpsc::channel::<Server>(1_000);
        Parser {
            udp_listener_receiver: Arc::new(Mutex::new(udp_listener_receiver)),
            sender_parsed: Arc::new(parser_sender),
            udp_sender_sender: Arc::new(sender_udp),
            receiver_parsed: Arc::new(Mutex::new(parser_receiver)),
        }
    }

    pub async fn start(&mut self) {
        let a = self.udp_listener_receiver.clone();
        let b = self.sender_parsed.clone();
        let c = self.udp_sender_sender.clone();

        tokio::spawn(async move {
            Parser::parse_response(a, b, c).await;
        });
    }

    pub async fn parse_response(
        listener_receiver: Arc<Mutex<Receiver<(Vec<u8>, SocketAddr)>>>,
        sender_processed: Arc<Sender<Server>>,
        sender_udp: Arc<Sender<(Vec<u8>, SocketAddr)>>,
    ) {
        while let Some((response, addr)) = listener_receiver.lock().await.recv().await {
            let response_length = response.len();

            // Quake protocol response
            if response_length >= 18 {
                if response[0..18] == *b"\xFF\xFF\xFF\xFFstatusResponse" {
                    let b = String::from_utf8(response[20..response_length].to_vec()).unwrap();
                    let d: Vec<&str> = b.split("\\").collect();
                    //println!("{:?}", &b);
                    let mut newmap: BTreeMap<&str, &str> = BTreeMap::new();
                    let mut i = 0;
                    while i < d.len() {
                        newmap.insert(d[i], d[i + 1]);
                        i += 2;
                    }
                    if false {
                        println!("{}: {:?}", addr, newmap);
                    }
                    let resp = Server {
                        ip_port: addr.ip().to_string() + ":" + addr.port().to_string().as_str(),
                        hostname: newmap.get("sv_hostname").unwrap().to_string(),
                        game: newmap.get("gamename").unwrap().to_string(),
                        map: newmap.get("mapname").unwrap().to_string(),
                        players: "".to_string(),
                        players_max: newmap.get("sv_maxclients").unwrap().to_string(),
                        query_port: addr.port(),
                        rcon: None,
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
                {server_id},s
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
                    let resp = Server {
                        ip_port: addr.ip().to_string() + ":" + addr.port().to_string().as_str(),
                        hostname: server_name,
                        game: server_game,
                        map: server_map,
                        players: server_players.to_string(),
                        players_max: server_players_max.to_string(),
                        query_port: addr.port(),
                        rcon: None,
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
                if response[0..9] == *b"\x5C\x68\x6F\x73\x74\x6E\x61\x6D\x65" {
                    let b = String::from_utf8(response[1..response_length].to_vec()).unwrap();
                    let d: Vec<&str> = b.split("\\").collect();
                    //println!("{:?}", &b);
                    let mut newmap: BTreeMap<&str, &str> = BTreeMap::new();
                    let mut i = 0;
                    while i < d.len() {
                        newmap.insert(d[i], d[i + 1]);
                        i += 2;
                    }
                    if false {
                        println!("{}: {:?}", addr, newmap);
                    }
                    let resp = Server {
                        ip_port: addr.ip().to_string() + ":" + addr.port().to_string().as_str(),
                        hostname: newmap.get("hostname").unwrap().to_string(),
                        game: "".to_string(), //newmap.get("gamename").unwrap().to_string(),
                        map: newmap.get("mapname").unwrap().to_string(),
                        players: newmap.get("numplayers").unwrap().to_string(),
                        players_max: newmap.get("maxplayers").unwrap().to_string(),
                        query_port: addr.port(),
                        rcon: None,
                    };

                    let _ = sender_processed.send(resp).await;
                }
            }

            //sender_processed.send(resp);
        }
    }
}
