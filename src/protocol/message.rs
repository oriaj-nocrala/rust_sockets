// Re-export from root where protobuf types are generated
pub use crate::{
    P2pMessage as Message, MessageContent, TextMessage, FileMessage, 
    FileRequest, FileResponse, PeerInfo,
};