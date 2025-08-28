use crate::error::{P2PError, P2PResult};
use crate::events::P2PEvent;
use crate::{P2pMessage as Message, PeerInfo};
use prost::Message as ProstMessage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

pub struct PeerConnection {
    peer_info: PeerInfo,
    stream: TcpStream,
    is_connected: bool,
}

impl PeerConnection {
    pub fn new(peer_info: PeerInfo, stream: TcpStream) -> Self {
        Self {
            peer_info,
            stream,
            is_connected: true,
        }
    }

    pub async fn send_message(&mut self, message: &Message) -> P2PResult<()> {
        if !self.is_connected {
            return Err(P2PError::PeerNotFound {
                peer_id: self.peer_info.id.clone(),
            });
        }

        let mut data = Vec::new();
        message.encode(&mut data)?;
        let size = data.len() as u64;

        self.stream.write_all(&size.to_be_bytes()).await?;
        self.stream.write_all(&data).await?;
        self.stream.flush().await?;

        Ok(())
    }

    pub async fn receive_message(&mut self) -> P2PResult<Message> {
        if !self.is_connected {
            return Err(P2PError::PeerNotFound {
                peer_id: self.peer_info.id.clone(),
            });
        }

        let mut size_bytes = [0u8; 8];
        self.stream.read_exact(&mut size_bytes).await?;
        let size = u64::from_be_bytes(size_bytes) as usize;

        let mut buffer = vec![0u8; size];
        self.stream.read_exact(&mut buffer).await?;

        let message = Message::decode(&buffer[..])?;
        Ok(message)
    }

    pub fn disconnect(&mut self) {
        self.is_connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    pub fn peer_info(&self) -> &PeerInfo {
        &self.peer_info
    }
}

pub struct PeerManager {
    connections: Arc<Mutex<HashMap<String, PeerConnection>>>,
    event_sender: mpsc::UnboundedSender<P2PEvent>,
    listener_port: u16,
    is_listening: Arc<tokio::sync::Mutex<bool>>,
}

impl PeerManager {
    pub fn new(
        listener_port: u16,
        event_sender: mpsc::UnboundedSender<P2PEvent>,
    ) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            listener_port,
            is_listening: Arc::new(tokio::sync::Mutex::new(false)),
        }
    }

    pub async fn start_listening(&self) -> P2PResult<()> {
        {
            let mut listening = self.is_listening.lock().await;
            if *listening {
                return Ok(());
            }
            *listening = true;
        }

        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.listener_port)).await?;
        let connections_clone = self.connections.clone();
        let event_sender_clone = self.event_sender.clone();
        let is_listening_clone = self.is_listening.clone();

        tokio::spawn(async move {
            loop {
                if !*is_listening_clone.lock().await {
                    break;
                }

                match listener.accept().await {
                    Ok((stream, addr)) => {
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

                        let connection = PeerConnection::new(peer_info.clone(), stream);
                        
                        {
                            let mut connections = connections_clone.lock().await;
                            connections.insert(peer_info.id.clone(), connection);
                        }

                        let _ = event_sender_clone.send(P2PEvent::PeerConnected(peer_info.clone()));

                        let connections_for_handler = connections_clone.clone();
                        let event_sender_for_handler = event_sender_clone.clone();
                        let peer_id = peer_info.id.clone();

                        tokio::spawn(async move {
                            Self::handle_peer_messages(
                                peer_id,
                                connections_for_handler,
                                event_sender_for_handler,
                            )
                            .await;
                        });
                    }
                    Err(_) => {}
                }
            }
        });

        Ok(())
    }

    async fn handle_peer_messages(
        peer_id: String,
        connections: Arc<Mutex<HashMap<String, PeerConnection>>>,
        event_sender: mpsc::UnboundedSender<P2PEvent>,
    ) {
        loop {
            let message_result = {
                let mut connections_map = connections.lock().await;
                if let Some(connection) = connections_map.get_mut(&peer_id) {
                    if !connection.is_connected() {
                        break;
                    }
                    connection.receive_message().await
                } else {
                    break;
                }
            };

            match message_result {
                Ok(message) => {
                    let _ = event_sender.send(P2PEvent::MessageReceived(message));
                }
                Err(_) => {
                    let mut connections_map = connections.lock().await;
                    if let Some(connection) = connections_map.get_mut(&peer_id) {
                        connection.disconnect();
                        let peer_info = connection.peer_info().clone();
                        let _ = event_sender.send(P2PEvent::PeerDisconnected(peer_info));
                    }
                    break;
                }
            }
        }
    }

    pub async fn connect_to_peer(&self, peer_info: &PeerInfo) -> P2PResult<()> {
        let addr = format!("{}:{}", peer_info.ip, peer_info.port);
        let stream = TcpStream::connect(&addr).await?;
        
        let connection = PeerConnection::new(peer_info.clone(), stream);
        
        {
            let mut connections = self.connections.lock().await;
            connections.insert(peer_info.id.clone(), connection);
        }

        let _ = self
            .event_sender
            .send(P2PEvent::PeerConnected(peer_info.clone()));

        let connections_clone = self.connections.clone();
        let event_sender_clone = self.event_sender.clone();
        let peer_id = peer_info.id.clone();

        tokio::spawn(async move {
            Self::handle_peer_messages(peer_id, connections_clone, event_sender_clone).await;
        });

        Ok(())
    }

    pub async fn send_message_to_peer(
        &self,
        peer_id: &str,
        message: &Message,
    ) -> P2PResult<()> {
        let mut connections = self.connections.lock().await;
        if let Some(connection) = connections.get_mut(peer_id) {
            connection.send_message(message).await
        } else {
            Err(P2PError::PeerNotFound {
                peer_id: peer_id.to_string(),
            })
        }
    }

    pub async fn disconnect_peer(&self, peer_id: &str) -> P2PResult<()> {
        let mut connections = self.connections.lock().await;
        if let Some(connection) = connections.get_mut(peer_id) {
            connection.disconnect();
            Ok(())
        } else {
            Err(P2PError::PeerNotFound {
                peer_id: peer_id.to_string(),
            })
        }
    }

    pub async fn get_connected_peers(&self) -> Vec<PeerInfo> {
        let connections = self.connections.lock().await;
        connections
            .values()
            .filter(|conn| conn.is_connected())
            .map(|conn| conn.peer_info().clone())
            .collect()
    }

    pub async fn stop_listening(&self) {
        *self.is_listening.lock().await = false;
    }
}