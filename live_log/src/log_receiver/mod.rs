pub mod log_assembler;
pub mod tcp;
pub mod udp;

use tokio::sync::mpsc::Receiver;

use tcp::LogReceiverTcp;
use udp::LogReceiverUdp;

#[derive(Debug)]
pub enum LogReceiver {
    Tcp(LogReceiverTcp),
    Udp(LogReceiverUdp),
}

impl LogReceiver {
    pub fn port(&self) -> u16 {
        match self {
            Self::Tcp(receiver) => receiver.port(),
            Self::Udp(receiver) => receiver.port(),
        }
    }

    pub fn take_receiver(&mut self) -> Option<Receiver<String>> {
        match self {
            Self::Tcp(receiver) => receiver.take_receiver(),
            Self::Udp(receiver) => receiver.take_receiver(),
        }
    }

    pub async fn stop(self) {
        match self {
            Self::Tcp(receiver) => receiver.stop().await,
            Self::Udp(receiver) => receiver.stop().await,
        }
    }
}
