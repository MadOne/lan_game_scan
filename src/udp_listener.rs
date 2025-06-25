use core::net::SocketAddr;

use std::sync::Arc;
use tokio::io;
use tokio::sync::mpsc;
use tokio::sync::mpsc::*;

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
