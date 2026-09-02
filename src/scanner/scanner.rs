// scanner.rs

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::{mpsc::*, Mutex};

use std::time::Instant;

use crate::broadcast::Broadcast;
use crate::parser::Parser;
use crate::server::ScannedServer;
use crate::udp_listener::UdpListener;
use crate::udp_sender::UdpSender;

pub async fn create_scaner() -> (Arc<Mutex<Receiver<ScannedServer>>>, Sender<SocketAddr>) {
    let ping: Arc<std::sync::Mutex<HashMap<SocketAddr, Instant>>> =
        Arc::new(std::sync::Mutex::new(HashMap::new()));

    let socket = UdpSocket::bind("0.0.0.0:34153").await.unwrap();
    let udp_sender_socket = Arc::new(socket);
    let udp_reciever_socket = udp_sender_socket.clone();

    let udp_sender = UdpSender::new(udp_sender_socket);
    let send_to_udp_sender = udp_sender.start().await;
    let send_to_udp_sender2 = send_to_udp_sender.clone();

    let mut broadcast = Broadcast::new(send_to_udp_sender2);
    let send_to_query = broadcast.start(ping.clone());

    let udp_listener = UdpListener::new(udp_reciever_socket);
    let udp_listener_receiver = udp_listener.start().await;

    let mut parser = Parser::new(udp_listener_receiver, send_to_udp_sender, ping);
    let parsed = parser.receiver_parsed.clone();
    //let mut processed = processed.lock().await;
    parser.start().await;
    (parsed, send_to_query)
    /*loop {
        if let Some(server) = processed.recv().await {
            println!("{server:?}");
            let a = server_sender.send(server).await;
            println!("{a:?}");
        }
    }
    */
}
