use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct Message {
    pub id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub timestamp: u64,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum MessageContent {
    Text { text: String },
    File { filename: String, data: Vec<u8> },
    FileRequest { filename: String, size: u64 },
    FileResponse { filename: String, accepted: bool },
}

impl Message {
    pub fn new_text(sender_id: String, sender_name: String, text: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender_id,
            sender_name,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            content: MessageContent::Text { text },
        }
    }

    pub fn new_file(sender_id: String, sender_name: String, filename: String, data: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender_id,
            sender_name,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            content: MessageContent::File { filename, data },
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::encode_to_vec(self, bincode::config::standard())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let (decoded, _) = bincode::decode_from_slice(bytes, bincode::config::standard())?;
        Ok(decoded)
    }

    pub fn size(&self) -> usize {
        match &self.content {
            MessageContent::Text { text } => text.len(),
            MessageContent::File { data, .. } => data.len(),
            MessageContent::FileRequest { .. } => 0,
            MessageContent::FileResponse { .. } => 0,
        }
    }
}