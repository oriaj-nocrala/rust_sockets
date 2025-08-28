use crate::{P2pMessage as Message, PeerInfo};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum P2PEvent {
    PeerDiscovered(PeerInfo),
    PeerConnected(PeerInfo),
    PeerDisconnected(PeerInfo),
    MessageReceived(Message),
    MessageSent(Message),
    FileTransferStarted { 
        peer_id: String, 
        filename: String,
        size: u64,
    },
    FileTransferProgress { 
        peer_id: String, 
        filename: String,
        bytes_transferred: u64,
        total_bytes: u64,
    },
    FileTransferCompleted { 
        peer_id: String, 
        filename: String,
    },
    FileTransferFailed { 
        peer_id: String, 
        filename: String,
        error: String,
    },
    Error(String),
}

pub struct EventManager {
    event_sender: mpsc::UnboundedSender<P2PEvent>,
    event_receiver: Option<mpsc::UnboundedReceiver<P2PEvent>>,
}

impl EventManager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            event_sender: sender,
            event_receiver: Some(receiver),
        }
    }

    pub fn get_sender(&self) -> mpsc::UnboundedSender<P2PEvent> {
        self.event_sender.clone()
    }

    pub fn take_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<P2PEvent>> {
        self.event_receiver.take()
    }

    pub fn emit_event(&self, event: P2PEvent) {
        let _ = self.event_sender.send(event);
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}