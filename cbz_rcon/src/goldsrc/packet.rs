use crate::RconError;

pub const GOLD_SRC_HEADER: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
pub const MULTIPACKET_HEADER: [u8; 4] = [0xFE, 0xFF, 0xFF, 0xFF];

pub enum GoldSrcPacketResponse {
    Single(Vec<u8>),

    Multi {
        id: u32,
        payload: Vec<u8>,
        is_last: bool,
        packet_number: u8,
    },
}

pub fn challenge_request() -> Vec<u8> {
    let mut packet = Vec::from(GOLD_SRC_HEADER);
    packet.extend_from_slice(b"challenge rcon\n\0");
    packet
}

pub fn rcon_request(challenge: &str, password: &str, command: &str) -> Vec<u8> {
    let mut packet = Vec::from(GOLD_SRC_HEADER);

    let command = format!("rcon {} {} {}", challenge, password, command);
    packet.extend_from_slice(command.as_bytes());

    packet
}

pub fn response(packet: &[u8]) -> Option<&[u8]> {
    packet.strip_prefix(&GOLD_SRC_HEADER)
}

pub fn parse_response(response: &[u8]) -> Result<GoldSrcPacketResponse, RconError> {
    if response.starts_with(&GOLD_SRC_HEADER) {
        return Ok(GoldSrcPacketResponse::Single(response.to_vec()));
    }

    if response.starts_with(&MULTIPACKET_HEADER) {
        if response.len() < 9 {
            return Err(RconError::InvalidPacket);
        }

        let id = u32::from_le_bytes([response[4], response[5], response[6], response[7]]);

        let raw_byte_8 = response[8];
        let is_last = (raw_byte_8 & 0x80) != 0;
        let packet_number = raw_byte_8 & 0x7F;

        println!(
            "[GoldSrc RCON] raw byte 8: {:#04x} (index={}, is_last={})",
            raw_byte_8, packet_number, is_last
        );

        let payload = response[9..].to_vec();

        return Ok(GoldSrcPacketResponse::Multi {
            id,
            packet_number,
            is_last,
            payload,
        });
    }

    Err(RconError::InvalidPacket)
}
