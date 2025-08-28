use crate::error::{P2PError, P2PResult};
use crate::events::P2PEvent;
use crate::{P2pMessage as Message, PeerInfo, MessageContent, message_content, HandshakeMessage};
use prost::Message as ProstMessage;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

// Commands that can be sent to the PeerManager actor
#[derive(Debug)]
pub enum PeerCommand {
    Connect {
        peer_info: PeerInfo,
        respond_to: oneshot::Sender<P2PResult<()>>,
    },
    Disconnect {
        peer_id: String,
        respond_to: oneshot::Sender<P2PResult<()>>,
    },
    SendMessage {
        peer_id: String,
        message: Message,
        respond_to: oneshot::Sender<P2PResult<()>>,
    },
    GetConnectedPeers {
        respond_to: oneshot::Sender<Vec<PeerInfo>>,
    },
    StartListening {
        port: u16,
        respond_to: oneshot::Sender<P2PResult<()>>,
    },
    RegisterIncomingConnection {
        peer_info: PeerInfo,
        stream: TcpStream,
        respond_to: oneshot::Sender<P2PResult<()>>,
    },
    UpdatePeerInfo {
        old_peer_id: String,
        new_peer_info: PeerInfo,
        respond_to: oneshot::Sender<P2PResult<()>>,
    },
    Stop,
}

// Individual peer connection handler
pub struct PeerConnection {
    peer_info: PeerInfo,
    stream: TcpStream,
    event_sender: mpsc::UnboundedSender<P2PEvent>,
}

impl PeerConnection {
    pub fn new(peer_info: PeerInfo, stream: TcpStream, event_sender: mpsc::UnboundedSender<P2PEvent>) -> Self {
        Self {
            peer_info,
            stream,
            event_sender,
        }
    }

    pub async fn handle_messages(mut self) {
        loop {
            match self.receive_message().await {
                Ok(message) => {
                    let _ = self.event_sender.send(P2PEvent::MessageReceived(message));
                }
                Err(_) => {
                    let _ = self.event_sender.send(P2PEvent::PeerDisconnected(self.peer_info.clone()));
                    break;
                }
            }
        }
    }

    pub async fn send_message(&mut self, message: &Message) -> P2PResult<()> {
        let mut data = Vec::new();
        message.encode(&mut data)?;
        let size = data.len() as u64;

        self.stream.write_all(&size.to_be_bytes()).await?;
        self.stream.write_all(&data).await?;
        self.stream.flush().await?;

        Ok(())
    }

    async fn receive_message(&mut self) -> P2PResult<Message> {
        let mut size_bytes = [0u8; 8];
        self.stream.read_exact(&mut size_bytes).await?;
        let size = u64::from_be_bytes(size_bytes) as usize;

        let mut buffer = vec![0u8; size];
        self.stream.read_exact(&mut buffer).await?;

        let message = Message::decode(&buffer[..])?;
        Ok(message)
    }
}

// Main PeerManager actor - no more shared mutexes!
pub struct PeerManager {
    command_sender: mpsc::UnboundedSender<PeerCommand>,
}

impl PeerManager {
    pub fn new(
        event_sender: mpsc::UnboundedSender<P2PEvent>,
        our_peer_id: String,
        our_peer_name: String,
        our_tcp_port: u16,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        
        // Spawn the actor
        tokio::spawn(PeerManagerActor::new(
            event_sender, 
            cmd_rx, 
            cmd_tx.clone(),
            our_peer_id,
            our_peer_name,
            our_tcp_port,
        ).run());
        
        Self {
            command_sender: cmd_tx,
        }
    }

    pub async fn connect_to_peer(&self, peer_info: &PeerInfo) -> P2PResult<()> {
        let (tx, rx) = oneshot::channel();
        let cmd = PeerCommand::Connect {
            peer_info: peer_info.clone(),
            respond_to: tx,
        };
        
        self.command_sender.send(cmd).map_err(|_| P2PError::InvalidMessage)?;
        rx.await.map_err(|_| P2PError::InvalidMessage)?
    }

    pub async fn disconnect_peer(&self, peer_id: &str) -> P2PResult<()> {
        let (tx, rx) = oneshot::channel();
        let cmd = PeerCommand::Disconnect {
            peer_id: peer_id.to_string(),
            respond_to: tx,
        };
        
        self.command_sender.send(cmd).map_err(|_| P2PError::InvalidMessage)?;
        rx.await.map_err(|_| P2PError::InvalidMessage)?
    }

    pub async fn send_message_to_peer(&self, peer_id: &str, message: &Message) -> P2PResult<()> {
        let (tx, rx) = oneshot::channel();
        let cmd = PeerCommand::SendMessage {
            peer_id: peer_id.to_string(),
            message: message.clone(),
            respond_to: tx,
        };
        
        self.command_sender.send(cmd).map_err(|_| P2PError::InvalidMessage)?;
        rx.await.map_err(|_| P2PError::InvalidMessage)?
    }

    pub async fn get_connected_peers(&self) -> Vec<PeerInfo> {
        let (tx, rx) = oneshot::channel();
        let cmd = PeerCommand::GetConnectedPeers {
            respond_to: tx,
        };
        
        if self.command_sender.send(cmd).is_err() {
            return Vec::new();
        }
        
        rx.await.unwrap_or_else(|_| Vec::new())
    }

    pub async fn start_listening(&self, port: u16) -> P2PResult<()> {
        let (tx, rx) = oneshot::channel();
        let cmd = PeerCommand::StartListening {
            port,
            respond_to: tx,
        };
        
        self.command_sender.send(cmd).map_err(|_| P2PError::InvalidMessage)?;
        rx.await.map_err(|_| P2PError::InvalidMessage)?
    }

    pub async fn stop_listening(&self) {
        let _ = self.command_sender.send(PeerCommand::Stop);
    }
}

// The actor that actually manages connections
struct PeerManagerActor {
    event_sender: mpsc::UnboundedSender<P2PEvent>,
    command_receiver: mpsc::UnboundedReceiver<PeerCommand>,
    command_sender: mpsc::UnboundedSender<PeerCommand>,
    connections: HashMap<String, mpsc::UnboundedSender<Message>>,
    peer_info_map: HashMap<String, PeerInfo>,
    // Local peer info for handshakes
    our_peer_id: String,
    our_peer_name: String,
    our_tcp_port: u16,
}

impl PeerManagerActor {
    fn new(
        event_sender: mpsc::UnboundedSender<P2PEvent>,
        command_receiver: mpsc::UnboundedReceiver<PeerCommand>,
        command_sender: mpsc::UnboundedSender<PeerCommand>,
        our_peer_id: String,
        our_peer_name: String,
        our_tcp_port: u16,
    ) -> Self {
        Self {
            event_sender,
            command_receiver,
            command_sender,
            connections: HashMap::new(),
            peer_info_map: HashMap::new(),
            our_peer_id,
            our_peer_name,
            our_tcp_port,
        }
    }

    async fn run(mut self) {
        while let Some(command) = self.command_receiver.recv().await {
            match command {
                PeerCommand::Connect { peer_info, respond_to } => {
                    let result = self.handle_connect(peer_info).await;
                    let _ = respond_to.send(result);
                }
                PeerCommand::Disconnect { peer_id, respond_to } => {
                    let result = self.handle_disconnect(&peer_id).await;
                    let _ = respond_to.send(result);
                }
                PeerCommand::SendMessage { peer_id, message, respond_to } => {
                    let result = self.handle_send_message(&peer_id, &message).await;
                    let _ = respond_to.send(result);
                }
                PeerCommand::GetConnectedPeers { respond_to } => {
                    let peers = self.peer_info_map.values().cloned().collect();
                    let _ = respond_to.send(peers);
                }
                PeerCommand::StartListening { port, respond_to } => {
                    let result = self.handle_start_listening(port).await;
                    let _ = respond_to.send(result);
                }
                PeerCommand::RegisterIncomingConnection { peer_info, stream, respond_to } => {
                    let result = self.handle_register_incoming(peer_info, stream).await;
                    let _ = respond_to.send(result);
                }
                PeerCommand::UpdatePeerInfo { old_peer_id, new_peer_info, respond_to } => {
                    let result = self.handle_update_peer_info(old_peer_id, new_peer_info).await;
                    let _ = respond_to.send(result);
                }
                PeerCommand::Stop => break,
            }
        }
    }

    async fn handle_connect(&mut self, peer_info: PeerInfo) -> P2PResult<()> {
        let addr = format!("{}:{}", peer_info.ip, peer_info.port);
        let stream = TcpStream::connect(&addr).await?;
        
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let peer_id = peer_info.id.clone();
        
        // Store connection
        self.connections.insert(peer_id.clone(), msg_tx);
        self.peer_info_map.insert(peer_id.clone(), peer_info.clone());
        
        // Emit event
        let _ = self.event_sender.send(P2PEvent::PeerConnected(peer_info.clone()));
        
        // Send handshake immediately after connecting
        let handshake = Message {
            id: uuid::Uuid::new_v4().to_string(),
            sender_id: self.our_peer_id.clone(),
            sender_name: self.our_peer_name.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            content: Some(MessageContent {
                content: Some(message_content::Content::Handshake(HandshakeMessage {
                    peer_id: self.our_peer_id.clone(),
                    peer_name: self.our_peer_name.clone(),
                    tcp_port: self.our_tcp_port as u32,
                })),
            }),
        };
        
        if let Some(sender) = self.connections.get(&peer_id) {
            let _ = sender.send(handshake);
        }

        // Split connection for bidirectional handling
        let (stream_read, stream_write) = stream.into_split();
        
        let event_sender = self.event_sender.clone();
        let peer_info_for_incoming = peer_info.clone();
        
        // Spawn outgoing message handler
        tokio::spawn(async move {
            let mut stream = stream_write;
            while let Some(message) = msg_rx.recv().await {
                let mut data = Vec::new();
                if message.encode(&mut data).is_ok() {
                    let size = data.len() as u64;
                    if stream.write_all(&size.to_be_bytes()).await.is_err() {
                        break;
                    }
                    if stream.write_all(&data).await.is_err() {
                        break;
                    }
                    if stream.flush().await.is_err() {
                        break;
                    }
                }
            }
        });
        
        // Spawn incoming message handler for outgoing connection
        let event_sender_clone = event_sender.clone();
        let command_sender_clone = self.command_sender.clone();
        let peer_id_for_handler = peer_id.clone();
        tokio::spawn(async move {
            let mut stream = stream_read;
            loop {
                let mut size_bytes = [0u8; 8];
                if stream.read_exact(&mut size_bytes).await.is_err() {
                    break;
                }
                let size = u64::from_be_bytes(size_bytes) as usize;

                let mut buffer = vec![0u8; size];
                if stream.read_exact(&mut buffer).await.is_err() {
                    break;
                }

                if let Ok(message) = Message::decode(&buffer[..]) {
                    // Check if this is a handshake message
                    if let Some(content) = &message.content {
                        if let Some(message_content::Content::Handshake(handshake)) = &content.content {
                            // Update peer info with real details from handshake
                            let updated_peer_info = PeerInfo {
                                id: handshake.peer_id.clone(),
                                name: handshake.peer_name.clone(),
                                ip: peer_info_for_incoming.ip.clone(),
                                port: handshake.tcp_port,
                                last_seen: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            };
                            
                            // Send update command to actor
                            let (tx, _) = tokio::sync::oneshot::channel();
                            let _ = command_sender_clone.send(PeerCommand::UpdatePeerInfo {
                                old_peer_id: peer_id_for_handler.clone(),
                                new_peer_info: updated_peer_info,
                                respond_to: tx,
                            });
                            
                            // Don't forward handshake messages as regular messages
                            continue;
                        }
                    }
                    
                    let _ = event_sender_clone.send(P2PEvent::MessageReceived(message));
                } else {
                    break;
                }
            }
            
            // Connection closed
            let _ = event_sender_clone.send(P2PEvent::PeerDisconnected(peer_info_for_incoming));
        });
        
        Ok(())
    }

    async fn handle_disconnect(&mut self, peer_id: &str) -> P2PResult<()> {
        if let Some(info) = self.peer_info_map.remove(peer_id) {
            self.connections.remove(peer_id);
            let _ = self.event_sender.send(P2PEvent::PeerDisconnected(info));
        }
        Ok(())
    }

    async fn handle_send_message(&self, peer_id: &str, message: &Message) -> P2PResult<()> {
        if let Some(sender) = self.connections.get(peer_id) {
            sender.send(message.clone()).map_err(|_| P2PError::PeerNotFound {
                peer_id: peer_id.to_string(),
            })?;
            Ok(())
        } else {
            Err(P2PError::PeerNotFound {
                peer_id: peer_id.to_string(),
            })
        }
    }

    async fn handle_start_listening(&mut self, port: u16) -> P2PResult<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        let command_sender = self.command_sender.clone();
        
        tokio::spawn(async move {
            while let Ok((stream, addr)) = listener.accept().await {
                let peer_info = PeerInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "Unknown".to_string(),
                    ip: addr.ip().to_string(),
                    port: addr.port() as u32,
                    last_seen: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };

                // Register incoming connection in the actor
                let (tx, _) = tokio::sync::oneshot::channel();
                let _ = command_sender.send(PeerCommand::RegisterIncomingConnection {
                    peer_info: peer_info.clone(),
                    stream,
                    respond_to: tx,
                });
            }
        });
        
        Ok(())
    }

    async fn handle_register_incoming(&mut self, peer_info: PeerInfo, stream: TcpStream) -> P2PResult<()> {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let peer_id = peer_info.id.clone();
        
        // Store connection but DON'T emit event yet - wait for handshake
        self.connections.insert(peer_id.clone(), msg_tx);
        self.peer_info_map.insert(peer_id.clone(), peer_info.clone());
        
        // Split connection for bidirectional handling
        let (stream_read, stream_write) = stream.into_split();
        
        let event_sender = self.event_sender.clone();
        let peer_info_for_incoming = peer_info.clone();
        
        // Spawn outgoing message handler
        tokio::spawn(async move {
            let mut stream = stream_write;
            while let Some(message) = msg_rx.recv().await {
                let mut data = Vec::new();
                if message.encode(&mut data).is_ok() {
                    let size = data.len() as u64;
                    if stream.write_all(&size.to_be_bytes()).await.is_err() {
                        break;
                    }
                    if stream.write_all(&data).await.is_err() {
                        break;
                    }
                    if stream.flush().await.is_err() {
                        break;
                    }
                }
            }
        });
        
        // Spawn incoming message handler for incoming connection
        let event_sender_clone = event_sender.clone();
        let command_sender_clone = self.command_sender.clone();
        let temp_peer_id = peer_id.clone();
        tokio::spawn(async move {
            let mut stream = stream_read;
            loop {
                let mut size_bytes = [0u8; 8];
                if stream.read_exact(&mut size_bytes).await.is_err() {
                    break;
                }
                let size = u64::from_be_bytes(size_bytes) as usize;

                let mut buffer = vec![0u8; size];
                if stream.read_exact(&mut buffer).await.is_err() {
                    break;
                }

                if let Ok(message) = Message::decode(&buffer[..]) {
                    // Check if this is a handshake message
                    if let Some(content) = &message.content {
                        if let Some(message_content::Content::Handshake(handshake)) = &content.content {
                            // Update peer info with real details from handshake
                            let updated_peer_info = PeerInfo {
                                id: handshake.peer_id.clone(),
                                name: handshake.peer_name.clone(),
                                ip: peer_info_for_incoming.ip.clone(),
                                port: handshake.tcp_port,
                                last_seen: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            };
                            
                            // Send update command to actor
                            let (tx, _) = tokio::sync::oneshot::channel();
                            let _ = command_sender_clone.send(PeerCommand::UpdatePeerInfo {
                                old_peer_id: temp_peer_id.clone(),
                                new_peer_info: updated_peer_info,
                                respond_to: tx,
                            });
                            
                            // Don't forward handshake messages as regular messages
                            continue;
                        }
                    }
                    
                    let _ = event_sender_clone.send(P2PEvent::MessageReceived(message));
                } else {
                    break;
                }
            }
            
            // Connection closed
            let _ = event_sender_clone.send(P2PEvent::PeerDisconnected(peer_info_for_incoming));
        });
        
        Ok(())
    }

    async fn handle_update_peer_info(&mut self, old_peer_id: String, new_peer_info: PeerInfo) -> P2PResult<()> {
        // Check if this is an update from "Unknown" to real info
        let is_initial_handshake = if let Some(old_info) = self.peer_info_map.get(&old_peer_id) {
            old_info.name == "Unknown"
        } else {
            false
        };
        
        // Remove old entry and add new one with correct info
        if let Some(connection_sender) = self.connections.remove(&old_peer_id) {
            self.connections.insert(new_peer_info.id.clone(), connection_sender);
        }
        
        // Update peer info
        self.peer_info_map.remove(&old_peer_id);
        self.peer_info_map.insert(new_peer_info.id.clone(), new_peer_info.clone());
        
        // Only emit PeerConnected event if this is the initial handshake (Unknown -> Real name)
        if is_initial_handshake {
            let _ = self.event_sender.send(P2PEvent::PeerConnected(new_peer_info));
        }
        
        Ok(())
    }
}