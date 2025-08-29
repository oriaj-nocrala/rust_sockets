use crate::{P2PMessenger, P2PEvent, message_content};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: u64,
    pub message_type: MessageType,
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Text,
    File { filename: String, size: u64, saved_path: Option<String> },
    System,
}

#[derive(Debug, Clone)]
pub struct PeerStatus {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u32,
    pub last_seen: u64,
    pub is_connected: bool,
}

pub struct AppState {
    pub messenger: Arc<P2PMessenger>,
    pub messages: VecDeque<ChatMessage>,
    pub discovered_peers: Vec<PeerStatus>,
    pub connected_peers: Vec<PeerStatus>,
    pub selected_peer: Option<usize>,
    pub input_buffer: String,
    pub status_message: String,
    pub max_messages: usize,
}

impl AppState {
    pub fn new(messenger: P2PMessenger) -> Self {
        Self {
            messenger: Arc::new(messenger),
            messages: VecDeque::new(),
            discovered_peers: Vec::new(),
            connected_peers: Vec::new(),
            selected_peer: None,
            input_buffer: String::new(),
            status_message: "Ready".to_string(),
            max_messages: 100,
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
    }

    pub fn add_system_message(&mut self, content: String) {
        let message = ChatMessage {
            sender: "System".to_string(),
            content,
            timestamp: crate::get_current_timestamp(),
            message_type: MessageType::System,
        };
        self.add_message(message);
    }


    pub async fn refresh_peers(&mut self) {
        // Update discovered peers
        let discovered = self.messenger.get_discovered_peers();
        self.discovered_peers = discovered
            .into_iter()
            .map(|peer| PeerStatus {
                id: peer.id,
                name: peer.name,
                ip: peer.ip,
                port: peer.port,
                last_seen: peer.last_seen,
                is_connected: false,
            })
            .collect();

        // Update connected peers
        let connected = self.messenger.get_connected_peers().await;
        self.connected_peers = connected
            .into_iter()
            .map(|peer| PeerStatus {
                id: peer.id,
                name: peer.name,
                ip: peer.ip,
                port: peer.port,
                last_seen: peer.last_seen,
                is_connected: true,
            })
            .collect();

        // Mark discovered peers that are also connected
        for discovered in &mut self.discovered_peers {
            if self.connected_peers.iter().any(|c| c.id == discovered.id) {
                discovered.is_connected = true;
            }
        }
    }

    pub async fn connect_to_selected_peer(&mut self) -> Result<String, String> {
        if let Some(index) = self.selected_peer {
            if let Some(peer) = self.discovered_peers.get(index).cloned() {
                if peer.is_connected {
                    return Err("Already connected to this peer".to_string());
                }

                let peer_info = crate::PeerInfo {
                    id: peer.id.clone(),
                    name: peer.name.clone(),
                    ip: peer.ip.clone(),
                    port: peer.port,
                    last_seen: peer.last_seen,
                };

                let peer_name = peer.name.clone();
                match self.messenger.connect_to_peer(&peer_info).await {
                    Ok(()) => {
                        self.add_system_message(format!("Connecting to {}...", peer_name));
                        Ok(format!("Connecting to {}", peer_name))
                    }
                    Err(e) => Err(format!("Failed to connect: {}", e))
                }
            } else {
                Err("Invalid peer selection".to_string())
            }
        } else {
            Err("No peer selected".to_string())
        }
    }

    pub async fn disconnect_from_selected_peer(&mut self) -> Result<String, String> {
        if let Some(index) = self.selected_peer {
            if let Some(peer) = self.connected_peers.get(index).cloned() {
                let peer_name = peer.name.clone();
                match self.messenger.disconnect_peer(&peer.id).await {
                    Ok(()) => {
                        self.add_system_message(format!("Disconnected from {}", peer_name));
                        Ok(format!("Disconnected from {}", peer_name))
                    }
                    Err(e) => Err(format!("Failed to disconnect: {}", e))
                }
            } else {
                Err("Invalid peer selection".to_string())
            }
        } else {
            Err("No peer selected".to_string())
        }
    }

    pub async fn send_text_message(&mut self, text: String) -> Result<String, String> {
        if text.trim().is_empty() {
            return Err("Message cannot be empty".to_string());
        }

        if let Some(index) = self.selected_peer {
            if let Some(peer) = self.connected_peers.get(index).cloned() {
                let peer_name = peer.name.clone();
                match self.messenger.send_text_message(&peer.id, text.clone()).await {
                    Ok(()) => {
                        let message = ChatMessage {
                            sender: format!("{} (You)", self.messenger.peer_name()),
                            content: text.clone(),
                            timestamp: crate::get_current_timestamp(),
                            message_type: MessageType::Text,
                        };
                        self.add_message(message);
                        Ok(format!("Message sent to {}", peer_name))
                    }
                    Err(e) => Err(format!("Failed to send message: {}", e))
                }
            } else {
                Err("Invalid peer selection".to_string())
            }
        } else {
            Err("No peer selected".to_string())
        }
    }

    pub async fn send_file(&mut self, file_path: String) -> Result<String, String> {
        if file_path.trim().is_empty() {
            return Err("File path cannot be empty".to_string());
        }

        if let Some(index) = self.selected_peer {
            if let Some(peer) = self.connected_peers.get(index) {
                match self.messenger.send_file(&peer.id, &file_path).await {
                    Ok(()) => Ok(format!("File sent to {}", peer.name)),
                    Err(e) => Err(format!("Failed to send file: {}", e))
                }
            } else {
                Err("Invalid peer selection".to_string())
            }
        } else {
            Err("No peer selected".to_string())
        }
    }

    pub fn force_discovery(&self) -> Result<String, String> {
        match self.messenger.discover_peers() {
            Ok(_) => Ok("Discovery broadcast sent!".to_string()),
            Err(e) => Err(format!("Discovery failed: {}", e))
        }
    }

    pub fn get_status_info(&self) -> String {
        format!(
            "Name: {} | ID: {:.8}... | IP: {} | Discovered: {} | Connected: {}",
            self.messenger.peer_name(),
            self.messenger.peer_id(),
            self.messenger.get_local_ip(),
            self.discovered_peers.len(),
            self.connected_peers.len()
        )
    }
}

pub struct AppEventHandler;

impl AppEventHandler {
    pub async fn handle_p2p_event(event: P2PEvent, app_state: &mut AppState) {
        match event {
            P2PEvent::PeerDiscovered(peer) => {
                app_state.add_system_message(format!(
                    "🔍 Peer discovered: {} ({}:{}) ID:{:.8}...",
                    peer.name, peer.ip, peer.port, peer.id
                ));
                app_state.refresh_peers().await;
            }
            P2PEvent::PeerConnected(peer) => {
                app_state.add_system_message(format!(
                    "🔗 Peer connected: {} ({}:{}) ID:{:.8}...",
                    peer.name, peer.ip, peer.port, peer.id
                ));
                app_state.refresh_peers().await;
            }
            P2PEvent::PeerDisconnected(peer) => {
                app_state.add_system_message(format!(
                    "💔 Peer disconnected: {} ({}:{}) ID:{:.8}...",
                    peer.name, peer.ip, peer.port, peer.id
                ));
                app_state.refresh_peers().await;
            }
            P2PEvent::MessageReceived(message) => {
                if let Some(content) = &message.content {
                    match &content.content {
                        Some(message_content::Content::Text(text_msg)) => {
                            let chat_message = ChatMessage {
                                sender: message.sender_name.clone(),
                                content: text_msg.text.clone(),
                                timestamp: message.timestamp,
                                message_type: MessageType::Text,
                            };
                            app_state.add_message(chat_message);
                        }
                        Some(message_content::Content::File(file_msg)) => {
                            let size_kb = file_msg.data.len() as u64 / 1024;
                            match app_state.messenger.save_received_file(&message) {
                                Ok(path) => {
                                    let chat_message = ChatMessage {
                                        sender: message.sender_name.clone(),
                                        content: format!("📁 File received: {} ({} KB)", file_msg.filename, size_kb),
                                        timestamp: message.timestamp,
                                        message_type: MessageType::File {
                                            filename: file_msg.filename.clone(),
                                            size: file_msg.data.len() as u64,
                                            saved_path: Some(path),
                                        },
                                    };
                                    app_state.add_message(chat_message);
                                }
                                Err(e) => {
                                    app_state.add_system_message(format!(
                                        "❌ Failed to save file {}: {}",
                                        file_msg.filename, e
                                    ));
                                }
                            }
                        }
                        _ => {
                            app_state.add_system_message(format!(
                                "📨 Unknown message type from {}",
                                message.sender_name
                            ));
                        }
                    }
                }
            }
            P2PEvent::FileTransferStarted { filename, size, .. } => {
                let size_kb = size / 1024;
                app_state.add_system_message(format!(
                    "📤 Sending file {} ({} KB)...",
                    filename, size_kb
                ));
            }
            P2PEvent::FileTransferCompleted { filename, .. } => {
                app_state.add_system_message(format!("✅ File sent successfully: {}", filename));
            }
            P2PEvent::FileTransferFailed { filename, error, .. } => {
                app_state.add_system_message(format!(
                    "❌ File transfer failed for {}: {}",
                    filename, error
                ));
            }
            P2PEvent::Error(error) => {
                app_state.add_system_message(format!("❌ Library error: {}", error));
            }
            _ => {}
        }
    }
}