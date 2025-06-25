use core::net::SocketAddr;

use std::net::{IpAddr, Ipv4Addr};
use tokio::sync::mpsc::*;
use tokio::time::{sleep, Duration};

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

            let utports: Vec<u16> = vec![7777, 7778, 7787, 7788, 23000];

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
