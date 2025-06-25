use core::net::SocketAddr;

use std::sync::Arc;
use tokio::sync::{mpsc::*, Mutex};

use tokio::sync::mpsc;

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
