use crate::error::P2PResult;
use crate::protocol::{discovery::*, PeerInfo};
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
    socket: UdpSocket,
    peers: Arc<Mutex<HashMap<String, PeerInfo>>>,
    is_running: Arc<Mutex<bool>>,
}

impl DiscoveryService {
    pub fn new(peer_name: String, tcp_port: u16) -> P2PResult<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT))?;
        socket.set_broadcast(true)?;
        socket.set_nonblocking(true)?;

        Ok(Self {
            peer_id: Uuid::new_v4().to_string(),
            peer_name,
            tcp_port,
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
                        if let Ok(msg) = DiscoveryMessage::from_bytes(&buffer[..size]) {
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
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                if !*is_running.lock().unwrap() {
                    break;
                }

                let announce = DiscoveryMessage::Announce {
                    peer_name: peer_name.clone(),
                    peer_id: peer_id.clone(),
                    tcp_port,
                };

                if let Ok(data) = announce.to_bytes() {
                    let addr = format!("{}:{}", BROADCAST_ADDR, DISCOVERY_PORT);
                    let _ = socket.send_to(&data, &addr);
                }
            }
        });
    }

    fn handle_discovery_message(
        msg: DiscoveryMessage,
        src: SocketAddr,
        peers: &Arc<Mutex<HashMap<String, PeerInfo>>>,
    ) {
        match msg {
            DiscoveryMessage::Announce {
                peer_name,
                peer_id,
                tcp_port,
            } => {
                let mut peers_map = peers.lock().unwrap();
                let peer_info = PeerInfo::new(peer_name, src.ip().to_string(), tcp_port);
                peers_map.insert(peer_id, peer_info);
            }
            DiscoveryMessage::Request => {}
        }
    }

    pub fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.lock().unwrap();
        peers.values().cloned().collect()
    }

    pub fn request_peers(&self) -> P2PResult<()> {
        let request = DiscoveryMessage::Request;
        let data = request.to_bytes()?;
        let addr = format!("{}:{}", BROADCAST_ADDR, DISCOVERY_PORT);
        self.socket.send_to(&data, &addr)?;
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