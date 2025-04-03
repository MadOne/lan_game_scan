use core::net::SocketAddr;
use std::sync::Arc;
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr},
};
use tokio::sync::mpsc::*;
use tokio::{
    io,
    time::{sleep, Duration},
};
use tokio::{net::UdpSocket, sync::mpsc};

use std::collections::VecDeque;

#[derive(Debug)]
pub enum Game {
    CS,
    Cod4,
    CS2,
    Q3,
}

pub struct ServerScaner {
    servers: BTreeMap<String, Server>,
    scan_intervall: i32,
    broadcast: bool,
}

#[derive(Debug)]
pub struct Server {
    pub addr: SocketAddr,
    pub hostname: String,
    pub game: String,
    pub map: String,
    pub players: String,
    pub players_max: String,
}

/*
struct Q3Info {
    punkbuster: Option<i32>,
    version: Option<String>,
    platform: Option<String>,
    date: Option<String>,
    dmflags: Option<String>,
    fraglimit: Option<i32>,
    timelimit: Option<i32>,
    gametype: Option<i32>,
    protocol: Option<i32>,
    mapname: Option<String>,
    private_clients: Option<i32>,
    hostname: Option<String>,
    max_clients: Option<i32>,
    max_rate: Option<i32>,
    min_ping: Option<i32>,
    max_ping: Option<i32>,
    flood_protection: Option<i32>,
    allow_download: Option<i32>,
    bot_minplayers: Option<i32>,
    game_name: Option<i32>,
    max_game_clients: Option<i32>,
    capture_limit: Option<i32>,
    need_pass: Option<i32>,
}
*/
impl ServerScaner {
    pub fn new() -> ServerScaner {
        let map: BTreeMap<String, Server> = BTreeMap::new();
        ServerScaner {
            servers: map,
            scan_intervall: 10,
            broadcast: true,
        }
    }

    pub fn print_servers(&self) {
        print!("{:?}", self.servers)
    }
    

    pub async fn udp_send(socket: &tokio::net::UdpSocket, mut rx: Receiver<(Vec<u8>, SocketAddr)>) {
        while let Some((bytes, addr)) = rx.recv().await {
            socket.writable().await.unwrap();
            socket.set_broadcast(true).unwrap();
            socket.send_to(&bytes, addr).await.unwrap();
        }
    }

    pub async fn udp_listen(socket: &tokio::net::UdpSocket, tx: Sender<(Vec<u8>, SocketAddr)>) {
        loop {
            socket.readable().await.unwrap();
            let mut buf = [0; 1024];
            match socket.try_recv_from(&mut buf) {
                Ok((n, addr)) => {
                    let a = buf[..n].to_vec();
                    let _ = tx.send((a.clone(), addr)).await;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    println!("An error occoured while listening to UDP: {:?}", e);
                }
            }
        }
    }

    pub async fn process_response(
        mut listener_receiver: Receiver<(Vec<u8>, SocketAddr)>,
        sender_processed: Sender<Server>,
        sender_udp: Sender<(Vec<u8>, SocketAddr)>,
    ) {
        while let Some((response, addr)) = listener_receiver.recv().await {
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
                        addr,
                        hostname: newmap.get("sv_hostname").unwrap().to_string(),
                        game: newmap.get("gamename").unwrap().to_string(),
                        map: newmap.get("mapname").unwrap().to_string(),
                        players: "".to_string(),
                        players_max: newmap.get("sv_maxclients").unwrap().to_string(),
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
                    let val = Self::pop_bytes(&mut payload, 1);
                    let server_protocol = val[0];
                    let val = Self::pop_bytes(&mut payload, 0);
                    let server_name = String::from_utf8(val).unwrap();
                    let val = Self::pop_bytes(&mut payload, 0);
                    let server_map = String::from_utf8(val).unwrap();
                    let val = Self::pop_bytes(&mut payload, 0);
                    let server_folder = String::from_utf8(val).unwrap();
                    let val = Self::pop_bytes(&mut payload, 0);
                    let server_game = String::from_utf8(val).unwrap();
                    let val = Self::pop_bytes(&mut payload, 2);
                    let server_id = u16::from_ne_bytes([val[0], val[1]]);
                    let val = Self::pop_bytes(&mut payload, 1);
                    let server_players = val[0];
                    let val = Self::pop_bytes(&mut payload, 1);
                    let server_players_max = val[0];
                    let val = Self::pop_bytes(&mut payload, 1);
                    let server_bots = val[0];
                    let val = Self::pop_bytes(&mut payload, 1);
                    let server_type = val[0];
                    let val = Self::pop_bytes(&mut payload, 1);
                    let server_environment = val[0];
                    let val = Self::pop_bytes(&mut payload, 1);
                    let server_visibility = val[0];
                    let val = Self::pop_bytes(&mut payload, 1);
                    let server_vac = val[0];
                    let val = Self::pop_bytes(&mut payload, 0);
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
                        addr,
                        hostname: server_name,
                        game: server_game,
                        map: server_map,
                        players: server_players.to_string(),
                        players_max: server_players_max.to_string(),
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
            //sender_processed.send(resp);
        }
    }

    pub async fn broadcast(tx: Sender<(Vec<u8>, SocketAddr)>) {
        loop {
            // Wait for the socket to be writeable
            let broadcast_ip_addr: IpAddr = IpAddr::V4(Ipv4Addr::BROADCAST);
            let source_ports: Vec<u16> = vec![27015, 27016, 27017, 27018, 27019];
            let q3_ports: Vec<u16> = vec![
                27070, 27960, 27961, 27962, 27963, 27992, 28960, 28961, 28962, 28963,
            ];
            let source_query: &[u8; 25] = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";
            let quake3_query: &[u8; 14] =
                b"\xFF\xFF\xFF\xFF\x67\x65\x74\x73\x74\x61\x74\x75\x73\x0A";

            for port in source_ports {
                //println!("broadcasting source ports");
                let socket_addr = SocketAddr::new(broadcast_ip_addr, port);
                let _ = tx.send((source_query.to_vec(), socket_addr)).await;
            }

            for port in q3_ports {
                //println!("broadcasting q3 ports");
                let socket_addr = SocketAddr::new(broadcast_ip_addr, port);
                let _ = tx.send((quake3_query.to_vec(), socket_addr)).await;
            }
            sleep(Duration::from_secs(10)).await;
        }
    }

    pub async fn start_daemon(&mut self) -> () {
        let socket = UdpSocket::bind("0.0.0.0:34153").await.unwrap();
        let udp_sender = Arc::new(socket);
        let udp_reciever = udp_sender.clone();
        let (listener_tx, listener_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
        let (sender_tx, sender_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
        //let (_processor_tx, _processor_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
        let (processed_tx, mut processed_rx) = mpsc::channel::<Server>(1_000);

        //broadcast every X seconds
        tokio::spawn(async move {
            Self::udp_send(&udp_sender, sender_rx).await;
        });

        //listen for incoming udp responses to broadcast and send them through chanel
        tokio::spawn(async move {
            Self::udp_listen(&udp_reciever, listener_tx).await;
        });

        //start broadcasting (every 10 seconds for now)
        let sender_tx2 = sender_tx.clone();
        //let sender_tx3 = sender_tx.clone();
        tokio::spawn(async move {
            Self::broadcast( sender_tx.clone()).await;
        });

        tokio::spawn(async move {
            Self::process_response(listener_rx, processed_tx, sender_tx2).await;
        });
        while let Some(server) = processed_rx.recv().await {
            {
                self.servers.insert(server.addr.to_string(), server);
                println!("{:?}", self.servers);
            }
            //return (sender_tx3, processed_rx);
        }
    }
    // Pop $number bytes from vector.
    // When number == 0 pop a 0 terminated string
    pub fn pop_bytes(byte_vec: &mut VecDeque<u8>, number: i32) -> Vec<u8> {
        let mut myreturn: Vec<u8> = vec![];

        // Pop a 0 terminated string
        if number == 0 {
            loop {
                let mybyte = byte_vec.pop_front().unwrap();
                if mybyte.to_owned() == 0x00 {
                    return myreturn;
                } else {
                    myreturn.push(mybyte.to_owned());
                }
            }
        } else
        // Pop $number bytes
        {
            for _n in 1..number + 1 {
                let mybyte = byte_vec.pop_front().unwrap();
                myreturn.push(mybyte.to_owned());
            }
            return myreturn;
        }
    }
}
