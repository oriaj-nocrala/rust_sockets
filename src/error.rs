use thiserror::Error;

#[derive(Error, Debug)]
pub enum P2PError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    
    #[error("Protobuf encode error: {0}")]
    ProtobufEncode(#[from] prost::EncodeError),
    
    #[error("Protobuf decode error: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),
    
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