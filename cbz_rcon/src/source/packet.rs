use crate::RconError;

#[derive(Debug)]
pub struct SourceRconPacket {
    pub id: i32,
    pub packet_type: i32,
    pub body: String,
}

impl SourceRconPacket {
    pub fn new(id: i32, packet_type: i32, body: impl Into<String>) -> Self {
        Self {
            id,
            packet_type,
            body: body.into(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let body = self.body.as_bytes();

        let size = (body.len() + 10) as i32;

        let mut bytes = Vec::with_capacity((size + 4) as usize);

        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(&self.id.to_le_bytes());
        bytes.extend_from_slice(&self.packet_type.to_le_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(&[0, 0]);

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RconError> {
        if bytes.len() < 12 {
            return Err(RconError::InvalidPacket);
        }

        let size = i32::from_le_bytes(
            bytes[0..4]
                .try_into()
                .map_err(|_| RconError::InvalidPacket)?,
        );

        if size < 10 {
            return Err(RconError::InvalidPacket);
        }

        let id = i32::from_le_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|_| RconError::InvalidPacket)?,
        );

        let packet_type = i32::from_le_bytes(
            bytes[8..12]
                .try_into()
                .map_err(|_| RconError::InvalidPacket)?,
        );

        let body_end = bytes.len().saturating_sub(2);

        let body =
            String::from_utf8(bytes[12..body_end].to_vec()).map_err(|_| RconError::InvalidUtf8)?;

        Ok(Self {
            id,
            packet_type,
            body,
        })
    }
}
