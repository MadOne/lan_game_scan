//broadcast.rs
use core::net::SocketAddr;

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc::*;
use tokio::time::Duration;

/// Struct for managing broadcast functionality.
pub struct Broadcast {
    tx: Sender<(Vec<u8>, SocketAddr)>,
}
impl Broadcast {
    /// Creates a new `Broadcast` instance.
    ///
    /// # Arguments
    ///
    /// * `tx` - A `Sender` for sending byte vectors and socket addresses.
    pub fn new(tx: Sender<(Vec<u8>, SocketAddr)>) -> Broadcast {
        Broadcast { tx: tx }
    }

    async fn broadcast(
        tx: Sender<(Vec<u8>, SocketAddr)>,
        ping: Arc<Mutex<HashMap<SocketAddr, Instant>>>,
        mut receive_query_command: Receiver<SocketAddr>,
    ) {
        // Wait for the socket to be writeable
        //println!("sending broadcast");

        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let broadcast_ip_addr: IpAddr = IpAddr::V4(Ipv4Addr::BROADCAST);
        let source_ports: Vec<u16> = vec![27015, 27016, 27017, 27018, 27019];
        let q3_ports: Vec<u16> = vec![
            27070, 27960, 27961, 27962, 27963, 27992, 28960, 28961, 28962, 28963,
        ];

        let utports: Vec<u16> = vec![7777, 7778, 7787, 7788, 23000, 12203, 12300];

        let source_query: &[u8; 25] = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";

        // getstatus
        let quake3_query: &[u8; 14] =
                //b"\xFF\xFF\xFF\xFF\x67\x65\x74\x73\x74\x61\x74\x75\x73\x0A";
                b"\xFF\xFF\xFF\xFF\x67\x65\x74\x73\x74\x61\x74\x75\x73\x0A"; //getstatus

        // let gs_info = b"\x5C\x69\x6E\x66\x6F\x5C"; //info
        let gs_status = b"\x5C\x73\x74\x61\x74\x75\x73\x5C"; //status

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    for port in &source_ports {
                        //println!("broadcasting source ports");
                        let socket_addr = SocketAddr::new(broadcast_ip_addr, *port);
                        let _ = tx.send((source_query.to_vec(), socket_addr)).await;
                        ping.lock().unwrap().insert(socket_addr, Instant::now());
                    }
                    for port in &q3_ports {
                        //println!("broadcasting q3 ports");
                        let socket_addr = SocketAddr::new(broadcast_ip_addr, *port);
                        let _ = tx.send((quake3_query.to_vec(), socket_addr)).await;
                        ping.lock().unwrap().insert(socket_addr, Instant::now());
                    }
                    for port in &utports {
                        let socket_addr = SocketAddr::new(broadcast_ip_addr, *port);
                        let _ = tx.send((gs_status.to_vec(), socket_addr)).await;
                        ping.lock().unwrap().insert(socket_addr, Instant::now());
                        //let _ = tx.send((gs_info.to_vec(), socket_addr)).await;
            }}
                Some(socket_addr) = receive_query_command.recv() => {
                    ping.lock().unwrap().insert(socket_addr, Instant::now());
                    let _ = tx.send((source_query.to_vec(), socket_addr)).await;
                    let _ = tx.send((quake3_query.to_vec(), socket_addr)).await;
                    let _ = tx.send((gs_status.to_vec(), socket_addr)).await;

                }
            }
        }
    }

    /// Starts the broadcast loop in a new Tokio task.
    pub fn start(&mut self, ping: Arc<Mutex<HashMap<SocketAddr, Instant>>>) -> Sender<SocketAddr> {
        let a = self.tx.clone();
        let (send_to_query, receive_query_command) = channel::<SocketAddr>(1_000);

        tokio::spawn(async move {
            Broadcast::broadcast(a, ping, receive_query_command).await;
        });
        send_to_query
    }
}
