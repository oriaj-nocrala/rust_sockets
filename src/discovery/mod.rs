use crate::error::P2PResult;
use crate::protocol::discovery::*;
use crate::{PeerInfo, DiscoveryMessage, PeerAnnouncement, PeerRequest, discovery_message};
use prost::Message;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use uuid::Uuid;

pub struct DiscoveryService {
    pub peer_id: String,
    peer_name: String,
    tcp_port: u16,
    discovery_port: u16,
    socket: UdpSocket,
    peers: Arc<Mutex<HashMap<String, PeerInfo>>>,
    is_running: Arc<Mutex<bool>>,
}

impl DiscoveryService {
    pub fn new(peer_name: String, tcp_port: u16, discovery_port: u16) -> P2PResult<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", discovery_port))?;
        socket.set_broadcast(true)?;
        socket.set_nonblocking(true)?;

        Ok(Self {
            peer_id: Uuid::new_v4().to_string(),
            peer_name,
            tcp_port,
            discovery_port,
            socket,
            peers: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(Mutex::new(false)),
        })
    }

    pub async fn start(&self) -> P2PResult<()> {
        {
            let mut running = self.is_running.lock().unwrap();
            if *running {
                return Ok(());
            }
            *running = true;
        }

        let peers_clone = self.peers.clone();
        let socket = self.socket.try_clone()?;
        let is_running_clone = self.is_running.clone();

        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            
            loop {
                if !*is_running_clone.lock().unwrap() {
                    break;
                }

                match socket.recv_from(&mut buffer) {
                    Ok((size, src)) => {
                        if let Ok(msg) = DiscoveryMessage::decode(&buffer[..size]) {
                            Self::handle_discovery_message(msg, src, &peers_clone);
                        }
                    }
                    Err(_) => {}
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        self.start_announcement_loop().await;
        Ok(())
    }

    async fn start_announcement_loop(&self) {
        let socket = self.socket.try_clone().unwrap();
        let peer_id = self.peer_id.clone();
        let peer_name = self.peer_name.clone();
        let tcp_port = self.tcp_port;
        let _discovery_port = self.discovery_port;
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                if !*is_running.lock().unwrap() {
                    break;
                }

                let announce = DiscoveryMessage {
                    message: Some(discovery_message::Message::Announce(PeerAnnouncement {
                        peer_name: peer_name.clone(),
                        peer_id: peer_id.clone(),
                        tcp_port: tcp_port as u32,
                    })),
                };

                let mut buf = Vec::new();
                if announce.encode(&mut buf).is_ok() {
                    // Broadcast to multiple discovery ports for same-PC testing
                    let discovery_ports = [6968, 6970, 6972, 6974, 6976, 6978];
                    
                    for port in discovery_ports {
                        let addr = format!("{}:{}", BROADCAST_ADDR, port);
                        let _ = socket.send_to(&buf, &addr);
                    }
                }
            }
        });
    }

    fn handle_discovery_message(
        msg: DiscoveryMessage,
        src: SocketAddr,
        peers: &Arc<Mutex<HashMap<String, PeerInfo>>>,
    ) {
        if let Some(discovery_message::Message::Announce(announce)) = msg.message {
            let mut peers_map = peers.lock().unwrap();
            let peer_info = PeerInfo {
                id: announce.peer_id.clone(),
                name: announce.peer_name.clone(),
                ip: src.ip().to_string(),
                port: announce.tcp_port,
                last_seen: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            peers_map.insert(announce.peer_id.clone(), peer_info);
        }
        // Handle Request case if needed in the future
    }

    pub fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.lock().unwrap();
        peers.values().cloned().collect()
    }

    pub fn request_peers(&self) -> P2PResult<()> {
        let request = DiscoveryMessage {
            message: Some(discovery_message::Message::Request(PeerRequest {})),
        };
        let mut buf = Vec::new();
        request.encode(&mut buf)?;
        
        // Broadcast peer requests to multiple discovery ports
        let discovery_ports = [6968, 6970, 6972, 6974, 6976, 6978];
        
        for port in discovery_ports {
            let addr = format!("{}:{}", BROADCAST_ADDR, port);
            let _ = self.socket.send_to(&buf, &addr);
        }
        Ok(())
    }

    pub fn stop(&self) {
        *self.is_running.lock().unwrap() = false;
    }

    pub fn cleanup_stale_peers(&self, timeout_secs: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut peers = self.peers.lock().unwrap();
        peers.retain(|_, peer| now - peer.last_seen < timeout_secs);
    }
}