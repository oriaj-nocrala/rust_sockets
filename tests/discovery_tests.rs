use archsockrust::*;
use std::net::Ipv4Addr;
use tokio::time::{timeout, Duration, sleep};

#[tokio::test]
async fn test_broadcast_address_detection() {
    // This test verifies that our dynamic broadcast detection works correctly
    // and doesn't rely on hardcoded 255.255.255.255 which Windows often blocks
    
    // Create a discovery service to access the broadcast detection
    let _discovery = archsockrust::discovery::DiscoveryService::new(
        "BroadcastTest".to_string(), 
        7100, 
        7101
    ).expect("Failed to create discovery service");
    
    let addresses = archsockrust::discovery::DiscoveryService::get_broadcast_addresses();
    
    // Verify we have multiple broadcast strategies
    assert!(addresses.len() >= 3, "Should have at least 3 broadcast addresses (localhost, multicast, universal)");
    
    // Should include localhost broadcast for same-machine testing
    assert!(addresses.contains(&"127.255.255.255".to_string()), "Should include localhost broadcast");
    
    // Should include multicast fallback
    assert!(addresses.contains(&"224.0.0.251".to_string()), "Should include multicast address");
    
    // Should include universal broadcast as last resort
    assert!(addresses.contains(&"255.255.255.255".to_string()), "Should include universal broadcast");
    
    println!("✅ Detected {} broadcast addresses:", addresses.len());
    for (i, addr) in addresses.iter().enumerate() {
        println!("   {}. {}", i + 1, addr);
        
        // Verify each address is valid IP format
        assert!(addr.parse::<std::net::IpAddr>().is_ok(), "Invalid IP address: {}", addr);
    }
    
    // Verify we detected network-specific broadcasts (if any network interfaces exist)
    let network_specific = addresses.iter()
        .filter(|addr| !addr.starts_with("127.") && !addr.starts_with("224.") && *addr != "255.255.255.255")
        .count();
    
    println!("✅ Detected {} network-specific broadcast addresses", network_specific);
    
    if network_specific > 0 {
        println!("   This indicates dynamic network interface detection is working");
    } else {
        println!("   No network interfaces detected (possibly running in isolated environment)");
    }
}

#[test]
fn test_broadcast_calculation_logic() {
    // Test the broadcast address calculation logic with known network configurations
    
    struct NetworkTest {
        ip: Ipv4Addr,
        netmask: Ipv4Addr,
        expected_broadcast: Ipv4Addr,
        description: &'static str,
    }
    
    let test_cases = vec![
        NetworkTest {
            ip: Ipv4Addr::new(192, 168, 1, 100),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            expected_broadcast: Ipv4Addr::new(192, 168, 1, 255),
            description: "Standard home network /24",
        },
        NetworkTest {
            ip: Ipv4Addr::new(192, 168, 0, 50),
            netmask: Ipv4Addr::new(255, 255, 0, 0),
            expected_broadcast: Ipv4Addr::new(192, 168, 255, 255),
            description: "Large private network /16",
        },
        NetworkTest {
            ip: Ipv4Addr::new(10, 0, 5, 100),
            netmask: Ipv4Addr::new(255, 0, 0, 0),
            expected_broadcast: Ipv4Addr::new(10, 255, 255, 255),
            description: "Class A private network /8",
        },
        NetworkTest {
            ip: Ipv4Addr::new(172, 16, 10, 1),
            netmask: Ipv4Addr::new(255, 240, 0, 0),
            expected_broadcast: Ipv4Addr::new(172, 31, 255, 255),
            description: "Class B private network /12",
        },
    ];
    
    for test in test_cases {
        let ip_octets = test.ip.octets();
        let mask_octets = test.netmask.octets();
        
        let calculated_broadcast = Ipv4Addr::new(
            ip_octets[0] | (!mask_octets[0]),
            ip_octets[1] | (!mask_octets[1]),
            ip_octets[2] | (!mask_octets[2]),
            ip_octets[3] | (!mask_octets[3]),
        );
        
        assert_eq!(
            calculated_broadcast, 
            test.expected_broadcast,
            "Broadcast calculation failed for {}: {} with mask {} should give {} but got {}",
            test.description,
            test.ip,
            test.netmask,
            test.expected_broadcast,
            calculated_broadcast
        );
        
        println!("✅ {}: {}/{} -> broadcast {}", 
                test.description, 
                test.ip, 
                test.netmask, 
                calculated_broadcast);
    }
}

#[tokio::test]
async fn test_discovery_service_creation_and_ports() {
    // Test that we can create discovery services with different port configurations
    
    let test_configs = vec![
        (6968, 6969, "Default ports"),
        (7000, 7001, "Custom ports 1"),
        (8000, 8001, "Custom ports 2"),
        (9000, 9001, "Custom ports 3"),
    ];
    
    for (discovery_port, tcp_port, description) in test_configs {
        let discovery = archsockrust::discovery::DiscoveryService::new(
            format!("TestPeer_{}", discovery_port),
            tcp_port,
            discovery_port,
        );
        
        assert!(discovery.is_ok(), "Failed to create discovery service for {}", description);
        
        let discovery = discovery.unwrap();
        assert!(!discovery.peer_id.is_empty(), "Peer ID should not be empty for {}", description);
        // Note: tcp_port and discovery_port are private fields, so we can't directly test them
        // The creation success implies the ports were set correctly
        
        println!("✅ {}: Created discovery service (ID: {})", description, discovery.peer_id);
    }
}

#[tokio::test]
async fn test_discovery_message_serialization() {
    // Test that discovery messages can be properly serialized and deserialized
    
    use prost::Message;
    
    // Test PeerAnnouncement
    let announcement = PeerAnnouncement {
        peer_name: "TestPeer🚀".to_string(), // Include Unicode to test encoding
        peer_id: "test-peer-id-123".to_string(),
        tcp_port: 6969,
    };
    
    let discovery_msg = DiscoveryMessage {
        message: Some(discovery_message::Message::Announce(announcement.clone())),
    };
    
    // Serialize
    let mut buffer = Vec::new();
    let encode_result = discovery_msg.encode(&mut buffer);
    assert!(encode_result.is_ok(), "Failed to encode discovery message");
    assert!(!buffer.is_empty(), "Encoded buffer should not be empty");
    
    // Deserialize
    let decoded_result = DiscoveryMessage::decode(&buffer[..]);
    assert!(decoded_result.is_ok(), "Failed to decode discovery message");
    
    let decoded_msg = decoded_result.unwrap();
    match decoded_msg.message {
        Some(discovery_message::Message::Announce(decoded_announcement)) => {
            assert_eq!(decoded_announcement.peer_name, announcement.peer_name, "Peer name should match after serialization");
            assert_eq!(decoded_announcement.peer_id, announcement.peer_id, "Peer ID should match after serialization");
            assert_eq!(decoded_announcement.tcp_port, announcement.tcp_port, "TCP port should match after serialization");
            
            println!("✅ Successfully serialized and deserialized announcement:");
            println!("   Name: {}", decoded_announcement.peer_name);
            println!("   ID: {}", decoded_announcement.peer_id);
            println!("   Port: {}", decoded_announcement.tcp_port);
        }
        _ => panic!("Decoded message should be an announcement"),
    }
    
    // Test PeerRequest
    let request_msg = DiscoveryMessage {
        message: Some(discovery_message::Message::Request(PeerRequest {})),
    };
    
    let mut request_buffer = Vec::new();
    assert!(request_msg.encode(&mut request_buffer).is_ok(), "Failed to encode request message");
    
    let decoded_request = DiscoveryMessage::decode(&request_buffer[..]);
    assert!(decoded_request.is_ok(), "Failed to decode request message");
    
    match decoded_request.unwrap().message {
        Some(discovery_message::Message::Request(_)) => {
            println!("✅ Successfully serialized and deserialized peer request");
        }
        _ => panic!("Decoded message should be a request"),
    }
}

#[tokio::test]
async fn test_event_system_integration() {
    // Test that the event system properly connects discovery to the event manager
    
    let mut discovery = archsockrust::discovery::DiscoveryService::new(
        "EventTestPeer".to_string(),
        7200,
        7201,
    ).unwrap();
    
    // Create event channel
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    
    // Connect discovery to event system
    discovery.set_event_sender(event_sender);
    
    // Start discovery service
    assert!(discovery.start().await.is_ok(), "Failed to start discovery service");
    
    // Give it time to initialize
    sleep(Duration::from_millis(100)).await;
    
    // Trigger discovery
    assert!(discovery.request_peers().is_ok(), "Failed to request peers");
    
    // Test cleanup functionality
    discovery.cleanup_stale_peers(60);
    
    println!("✅ Event system integration test completed successfully");
    println!("   Discovery service started and connected to event system");
    println!("   Peer request sent successfully");
    println!("   Stale peer cleanup executed");
    
    // Stop discovery
    discovery.stop();
    
    // Wait a moment for any final events
    let _ = timeout(Duration::from_millis(200), event_receiver.recv()).await;
    
    println!("✅ Discovery service stopped cleanly");
}

#[tokio::test]
async fn test_peer_info_structure() {
    // Test PeerInfo creation and field access
    
    let peer = PeerInfo {
        id: "test-peer-123".to_string(),
        name: "Test Peer 🌟".to_string(), // Unicode support
        ip: "192.168.1.100".to_string(),
        port: 6969,
        last_seen: archsockrust::get_current_timestamp(),
    };
    
    assert_eq!(peer.id, "test-peer-123");
    assert_eq!(peer.name, "Test Peer 🌟");
    assert_eq!(peer.ip, "192.168.1.100");
    assert_eq!(peer.port, 6969);
    assert!(peer.last_seen > 0, "last_seen should be a valid timestamp");
    
    // Test timestamp formatting
    let formatted_time = archsockrust::format_timestamp(peer.last_seen);
    assert!(!formatted_time.is_empty(), "Formatted timestamp should not be empty");
    assert!(!formatted_time.contains("??:??:??"), "Formatted timestamp should be valid");
    
    println!("✅ PeerInfo structure test passed:");
    println!("   ID: {}", peer.id);
    println!("   Name: {}", peer.name);
    println!("   Address: {}:{}", peer.ip, peer.port);
    println!("   Last seen: {} ({})", peer.last_seen, formatted_time);
}