pub mod discovery;
pub mod message;

// Re-export PeerInfo from message module for compatibility
pub use message::PeerInfo;