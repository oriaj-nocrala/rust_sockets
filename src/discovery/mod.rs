use crate::error::P2PResult;
use crate::protocol::discovery::{DISCOVERY_PORT, MULTICAST_ADDR};
use crate::{PeerInfo, DiscoveryMessage, PeerAnnouncement, PeerRequest, discovery_message, P2PEvent};
use prost::Message;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::interval;
use uuid::Uuid;
use if_addrs::get_if_addrs;

pub struct DiscoveryService {
    pub peer_id: String,
    peer_name: String,
    tcp_port: u16,
    discovery_port: u16,
    socket: UdpSocket,
    peers: Arc<Mutex<HashMap<String, PeerInfo>>>,
    is_running: Arc<Mutex<bool>>,
    event_sender: Option<mpsc::UnboundedSender<P2PEvent>>,
}

impl DiscoveryService {
    /// Get all available broadcast addresses for local network interfaces
    pub fn get_broadcast_addresses() -> Vec<String> {
        let mut addresses = Vec::new();
        
        // Add localhost for same-machine testing
        addresses.push("127.255.255.255".to_string());
        
        // Add multicast address as fallback
        addresses.push(MULTICAST_ADDR.to_string());
        
        // Get network interfaces and calculate broadcast addresses
        if let Ok(interfaces) = get_if_addrs() {
            for iface in interfaces {
                if let if_addrs::IfAddr::V4(ifv4) = iface.addr {
                    let ipv4 = ifv4.ip;
                    
                    // Skip loopback interfaces
                    if ipv4.is_loopback() {
                        continue;
                    }
                    
                    // Calculate broadcast address from IP and netmask
                    let netmask = ifv4.netmask;
                    let ip_octets = ipv4.octets();
                    let mask_octets = netmask.octets();
                    
                    let broadcast = Ipv4Addr::new(
                        ip_octets[0] | (!mask_octets[0]),
                        ip_octets[1] | (!mask_octets[1]),
                        ip_octets[2] | (!mask_octets[2]),
                        ip_octets[3] | (!mask_octets[3]),
                    );
                    
                    addresses.push(broadcast.to_string());
                }
            }
        }
        
        // Add universal broadcast as last resort (may be blocked)
        addresses.push("255.255.255.255".to_string());
        
        addresses
    }

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
            event_sender: None,
        })
    }
    
    /// Set event sender for sending peer discovery events
    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<P2PEvent>) {
        self.event_sender = Some(sender);
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
        let event_sender_clone = self.event_sender.clone();

        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            
            loop {
                if !*is_running_clone.lock().unwrap() {
                    break;
                }

                match socket.recv_from(&mut buffer) {
                    Ok((size, src)) => {
                        if let Ok(msg) = DiscoveryMessage::decode(&buffer[..size]) {
                            Self::handle_discovery_message(msg, src, &peers_clone, &event_sender_clone);
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
                    // Get dynamic broadcast addresses
                    let broadcast_addresses = Self::get_broadcast_addresses();
                    
                    // Send to each broadcast address on multiple discovery ports
                    // to support multiple instances with different discovery ports
                    let discovery_ports = [DISCOVERY_PORT, 6970, 6972, 6974, 6976, 6978, 7001, 7003];
                    
                    for addr in broadcast_addresses {
                        for port in discovery_ports {
                            let target = format!("{}:{}", addr, port);
                            if let Err(_e) = socket.send_to(&buf, &target) {
                                // Silently ignore errors to avoid spam
                                // Most ports won't be listening anyway
                            }
                        }
                    }
                }
            }
        });
    }

    fn handle_discovery_message(
        msg: DiscoveryMessage,
        src: SocketAddr,
        peers: &Arc<Mutex<HashMap<String, PeerInfo>>>,
        event_sender: &Option<mpsc::UnboundedSender<P2PEvent>>,
    ) {
        if let Some(discovery_message::Message::Announce(announce)) = msg.message {
            let mut peers_map = peers.lock().unwrap();
            
            // Check if this is a new peer
            let is_new_peer = !peers_map.contains_key(&announce.peer_id);
            
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
            
            peers_map.insert(announce.peer_id.clone(), peer_info.clone());
            
            // Send event for newly discovered peer
            if is_new_peer {
                if let Some(sender) = event_sender {
                    let _ = sender.send(P2PEvent::PeerDiscovered(peer_info));
                }
            }
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
        
        // Get dynamic broadcast addresses
        let broadcast_addresses = Self::get_broadcast_addresses();
        
        // Send to each broadcast address on multiple discovery ports
        let discovery_ports = [DISCOVERY_PORT, 6970, 6972, 6974, 6976, 6978, 7001, 7003];
        
        for addr in broadcast_addresses {
            for port in discovery_ports {
                let target = format!("{}:{}", addr, port);
                if let Err(_e) = self.socket.send_to(&buf, &target) {
                    // Silently ignore errors to avoid spam
                }
            }
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