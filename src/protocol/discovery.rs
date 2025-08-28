use serde::{Deserialize, Serialize};

pub const DISCOVERY_PORT: u16 = 6968;
pub const BROADCAST_ADDR: &str = "255.255.255.255";

#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum DiscoveryMessage {
    Announce {
        peer_name: String,
        peer_id: String,
        tcp_port: u16,
    },
    Request,
}

impl DiscoveryMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::encode_to_vec(self, bincode::config::standard())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let (decoded, _) = bincode::decode_from_slice(bytes, bincode::config::standard())?;
        Ok(decoded)
    }
}