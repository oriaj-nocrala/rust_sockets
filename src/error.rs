use thiserror::Error;

#[derive(Error, Debug)]
pub enum P2PError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    
    #[error("Serialization encode error: {0}")]
    SerializationEncode(#[from] bincode::error::EncodeError),
    
    #[error("Serialization decode error: {0}")]
    SerializationDecode(#[from] bincode::error::DecodeError),
    
    #[error("Peer not found: {peer_id}")]
    PeerNotFound { peer_id: String },
    
    #[error("Discovery timeout")]
    DiscoveryTimeout,
    
    #[error("Invalid message format")]
    InvalidMessage,
    
    #[error("Connection refused by peer")]
    ConnectionRefused,
}

pub type P2PResult<T> = Result<T, P2PError>;