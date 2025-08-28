pub mod discovery;
pub mod events;
pub mod peer;
pub mod protocol;
pub mod error;

use crate::discovery::DiscoveryService;
use crate::events::EventManager;
use crate::peer::PeerManager;
// Include generated protobuf code
include!(concat!(env!("OUT_DIR"), "/archsockrust.rs"));
use crate::error::{P2PError, P2PResult};

use std::fs;
use tokio::sync::mpsc;
use local_ip_address;

pub struct P2PMessenger {
    peer_name: String,
    peer_id: String,
    discovery: DiscoveryService,
    peer_manager: PeerManager,
    event_manager: EventManager,
}

impl P2PMessenger {
    pub fn new(peer_name: String) -> P2PResult<Self> {
        Self::with_ports(peer_name, 6969, 6968)
    }

    pub fn with_ports(peer_name: String, tcp_port: u16, discovery_port: u16) -> P2PResult<Self> {
        let discovery = DiscoveryService::new(peer_name.clone(), tcp_port, discovery_port)?;
        
        let event_manager = EventManager::new();
        let event_sender = event_manager.get_sender();
        
        let peer_manager = PeerManager::new(tcp_port, event_sender);
        
        Ok(Self {
            peer_id: discovery.peer_id.clone(),
            peer_name,
            discovery,
            peer_manager,
            event_manager,
        })
    }

    pub async fn start(&self) -> P2PResult<()> {
        self.discovery.start().await?;
        self.peer_manager.start_listening().await?;
        Ok(())
    }

    pub async fn stop(&self) {
        self.discovery.stop();
        self.peer_manager.stop_listening().await;
    }

    pub fn get_event_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<crate::events::P2PEvent>> {
        self.event_manager.take_receiver()
    }

    pub fn discover_peers(&self) -> P2PResult<Vec<PeerInfo>> {
        self.discovery.request_peers()?;
        Ok(self.discovery.get_peers())
    }

    pub fn get_discovered_peers(&self) -> Vec<PeerInfo> {
        self.discovery.get_peers()
    }

    pub async fn get_connected_peers(&self) -> Vec<PeerInfo> {
        self.peer_manager.get_connected_peers().await
    }

    pub async fn connect_to_peer(&self, peer_info: &PeerInfo) -> P2PResult<()> {
        self.peer_manager.connect_to_peer(peer_info).await
    }

    pub async fn disconnect_peer(&self, peer_id: &str) -> P2PResult<()> {
        self.peer_manager.disconnect_peer(peer_id).await
    }

    pub async fn send_text_message(&self, peer_id: &str, text: String) -> P2PResult<()> {
        let message = P2pMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender_id: self.peer_id.clone(),
            sender_name: self.peer_name.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            content: Some(MessageContent {
                content: Some(message_content::Content::Text(TextMessage { text })),
            }),
        };

        self.peer_manager
            .send_message_to_peer(peer_id, &message)
            .await?;

        self.event_manager
            .emit_event(crate::events::P2PEvent::MessageSent(message));

        Ok(())
    }

    pub async fn send_file(&self, peer_id: &str, file_path: &str) -> P2PResult<()> {
        let file_data = fs::read(file_path).map_err(P2PError::Network)?;
        let filename = std::path::Path::new(file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let message = P2pMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender_id: self.peer_id.clone(),
            sender_name: self.peer_name.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            content: Some(MessageContent {
                content: Some(message_content::Content::File(FileMessage { 
                    filename: filename.clone(), 
                    data: file_data 
                })),
            }),
        };

        self.event_manager.emit_event(crate::events::P2PEvent::FileTransferStarted {
            peer_id: peer_id.to_string(),
            filename: filename.clone(),
            size: match &message.content {
                Some(content) => match &content.content {
                    Some(message_content::Content::File(file)) => file.data.len() as u64,
                    _ => 0,
                },
                None => 0,
            },
        });

        match self.peer_manager.send_message_to_peer(peer_id, &message).await {
            Ok(()) => {
                self.event_manager.emit_event(crate::events::P2PEvent::FileTransferCompleted {
                    peer_id: peer_id.to_string(),
                    filename,
                });
                self.event_manager.emit_event(crate::events::P2PEvent::MessageSent(message));
                Ok(())
            }
            Err(e) => {
                self.event_manager.emit_event(crate::events::P2PEvent::FileTransferFailed {
                    peer_id: peer_id.to_string(),
                    filename,
                    error: e.to_string(),
                });
                Err(e)
            }
        }
    }

    pub fn save_received_file(&self, message: &P2pMessage) -> P2PResult<String> {
        if let Some(content) = &message.content {
            if let Some(message_content::Content::File(file_msg)) = &content.content {
                let save_dir = "recibidos";
                
                if !std::path::Path::new(save_dir).exists() {
                    fs::create_dir_all(save_dir).map_err(P2PError::Network)?;
                }

                let file_path = format!("{}/{}", save_dir, file_msg.filename);
                fs::write(&file_path, &file_msg.data).map_err(P2PError::Network)?;
                
                Ok(file_path)
            } else {
                Err(P2PError::InvalidMessage)
            }
        } else {
            Err(P2PError::InvalidMessage)
        }
    }

    pub fn get_local_ip(&self) -> String {
        local_ip_address::local_ip()
            .unwrap_or_else(|_| "127.0.0.1".parse().unwrap())
            .to_string()
    }

    pub fn peer_id(&self) -> &str {
        &self.peer_id
    }

    pub fn peer_name(&self) -> &str {
        &self.peer_name
    }

    pub fn cleanup_stale_peers(&self) {
        self.discovery.cleanup_stale_peers(60);
    }
}

pub use crate::events::P2PEvent;