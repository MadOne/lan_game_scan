use core::net::SocketAddr;

use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};
use tokio::sync::{mpsc::*, Mutex};
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

pub struct UdpSender {
    socket: Arc<tokio::net::UdpSocket>,
}

impl UdpSender {
    pub fn new(socket: Arc<tokio::net::UdpSocket>) -> UdpSender {
        UdpSender { socket }
    }

    async fn udp_send(
        &self,
        socket: Arc<tokio::net::UdpSocket>,
        rx: Arc<Mutex<Receiver<(Vec<u8>, SocketAddr)>>>,
    ) {
        while let Some((bytes, addr)) = rx.lock().await.recv().await {
            //unwrap().recv().await {
            socket.writable().await.unwrap();
            socket.set_broadcast(true).unwrap();
            socket.send_to(&bytes, addr).await.unwrap();
        }
    }

    pub async fn start(self: Self) -> Sender<(Vec<u8>, SocketAddr)> {
        let (listener_tx, listener_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
        let a = self.socket.clone();
        let b = Arc::new(Mutex::new(listener_rx));

        tokio::spawn(async move {
            self.udp_send(a, b).await;
        });
        listener_tx
    }
}

pub struct UdpListener {
    socket: Arc<tokio::net::UdpSocket>,
}

impl UdpListener {
    pub fn new(socket: Arc<tokio::net::UdpSocket>) -> UdpListener {
        UdpListener { socket }
    }

    pub async fn udp_listen(
        &self,
        socket: Arc<tokio::net::UdpSocket>,
        tx: Sender<(Vec<u8>, SocketAddr)>,
    ) {
        loop {
            socket.readable().await.unwrap();
            let mut buf = [0; 1024];
            match socket.try_recv_from(&mut buf) {
                Ok((n, addr)) => {
                    let a = buf[..n].to_vec();
                    let _ = tx.send((a.clone(), addr)).await;
                    //println!("got response in scanner");
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

    pub async fn start(self) -> Receiver<(Vec<u8>, SocketAddr)> {
        let a = self.socket.clone();
        let (listener_tx, listener_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);

        tokio::spawn(async move {
            self.udp_listen(a, listener_tx).await;
        });
        listener_rx
    }
}

pub struct Broadcast {
    tx: Sender<(Vec<u8>, SocketAddr)>,
}
impl Broadcast {
    pub fn new(tx: Sender<(Vec<u8>, SocketAddr)>) -> Broadcast {
        Broadcast { tx: tx }
    }

    pub async fn broadcast(tx: Sender<(Vec<u8>, SocketAddr)>) {
        loop {
            // Wait for the socket to be writeable
            //println!("sending broadcast");
            let broadcast_ip_addr: IpAddr = IpAddr::V4(Ipv4Addr::BROADCAST);
            let source_ports: Vec<u16> = vec![27015, 27016, 27017, 27018, 27019];
            let q3_ports: Vec<u16> = vec![
                27070, 27960, 27961, 27962, 27963, 27992, 28960, 28961, 28962, 28963,
            ];

            let utports: Vec<u16> = vec![7787, 7778];

            let source_query: &[u8; 25] = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";

            // getstatus
            let quake3_query: &[u8; 14] =
                b"\xFF\xFF\xFF\xFF\x67\x65\x74\x73\x74\x61\x74\x75\x73\x0A";

            let utquery = b"\x5C\x69\x6E\x66\x6F\x5C";

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

            for port in utports {
                let socket_addr = SocketAddr::new(broadcast_ip_addr, port);
                let _ = tx.send((utquery.to_vec(), socket_addr)).await;
            }
            sleep(Duration::from_secs(10)).await;
        }
    }

    pub fn start(&mut self) {
        let a = self.tx.clone();
        tokio::spawn(async move {
            Broadcast::broadcast(a).await;
        });
    }
}

pub async fn create_scaner() -> Arc<Mutex<Receiver<Server>>> {
    let socket = UdpSocket::bind("0.0.0.0:34153").await.unwrap();
    let udp_sender_socket = Arc::new(socket);
    let udp_reciever_socket = udp_sender_socket.clone();

    let udp_sender = UdpSender::new(udp_sender_socket);
    let send_to_udp_sender = udp_sender.start().await;
    let send_to_udp_sender2 = send_to_udp_sender.clone();

    let mut broadcast = Broadcast::new(send_to_udp_sender2);
    broadcast.start();

    let udp_listener = UdpListener::new(udp_reciever_socket);
    let udp_listener_receiver = udp_listener.start().await;

    let mut processor = Processor::new(udp_listener_receiver, send_to_udp_sender);
    let processed = processor.receiver_processed.clone();
    //let mut processed = processed.lock().await;
    processor.start().await;
    processed
    /*loop {
        if let Some(server) = processed.recv().await {
            println!("{server:?}");
            let a = server_sender.send(server).await;
            println!("{a:?}");
        }
    }
    */
}

pub struct Processor {
    udp_listener_receiver: Arc<Mutex<Receiver<(Vec<u8>, SocketAddr)>>>,
    sender_processed: Arc<Sender<Server>>,
    udp_sender_sender: Arc<Sender<(Vec<u8>, SocketAddr)>>,
    pub receiver_processed: Arc<Mutex<Receiver<Server>>>,
}

impl Processor {
    pub fn new(
        udp_listener_receiver: Receiver<(Vec<u8>, SocketAddr)>,
        sender_udp: Sender<(Vec<u8>, SocketAddr)>,
    ) -> Processor {
        let (processor_sender, processor_receiver) = mpsc::channel::<Server>(1_000);
        Processor {
            udp_listener_receiver: Arc::new(Mutex::new(udp_listener_receiver)),
            sender_processed: Arc::new(processor_sender),
            udp_sender_sender: Arc::new(sender_udp),
            receiver_processed: Arc::new(Mutex::new(processor_receiver)),
        }
    }

    pub async fn start(&mut self) {
        let a = self.udp_listener_receiver.clone();
        let b = self.sender_processed.clone();
        let c = self.udp_sender_sender.clone();

        tokio::spawn(async move {
            Processor::process_response(a, b, c).await;
        });
    }

    pub async fn process_response(
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
