use std::sync::Arc;
use tokio::sync::{mpsc::*, Mutex};

use tokio::net::UdpSocket;

mod broadcast;
mod db;
mod misc;
mod parser;
pub mod server;
mod udp_listener;
mod udp_sender;

use broadcast::Broadcast;
use parser::Parser;
use server::Server;
use udp_listener::UdpListener;
use udp_sender::UdpSender;

use crate::db::Db;

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

    let mut parser = Parser::new(udp_listener_receiver, send_to_udp_sender);
    let parsed = parser.receiver_parsed.clone();
    //let mut processed = processed.lock().await;
    parser.start().await;
    parsed
    /*loop {
        if let Some(server) = processed.recv().await {
            println!("{server:?}");
            let a = server_sender.send(server).await;
            println!("{a:?}");
        }
    }
    */
}

pub async fn start() {
    let db = Db {};
    tokio::spawn(async move {
        db.add_or_update_server().await;
    });
}
