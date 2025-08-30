// Unit tests for individual components and functions

use archsockrust::*;
use std::net::{SocketAddr, Ipv4Addr};

// Test PeerInfo structure and operations
#[test]
fn test_peer_info_creation_and_validation() {
    let current_time = archsockrust::get_current_timestamp();
    
    let peer = PeerInfo {
        id: "test-peer-12345".to_string(),
        name: "Test Peer Name".to_string(),
        ip: "192.168.1.100".to_string(),
        port: 6969,
        last_seen: current_time,
    };
    
    // Validate all fields
    assert_eq!(peer.id, "test-peer-12345");
    assert_eq!(peer.name, "Test Peer Name");
    assert_eq!(peer.ip, "192.168.1.100");
    assert_eq!(peer.port, 6969);
    assert_eq!(peer.last_seen, current_time);
    
    // Test with Unicode characters
    let unicode_peer = PeerInfo {
        id: "unicode-测试-🚀".to_string(),
        name: "用户名 🌟".to_string(),
        ip: "10.0.0.1".to_string(),
        port: 7000,
        last_seen: current_time,
    };
    
    assert_eq!(unicode_peer.name, "用户名 🌟");
    assert_eq!(unicode_peer.id, "unicode-测试-🚀");
    
    println!("✅ PeerInfo validation tests passed");
    println!("   Standard peer: {} at {}:{}", peer.name, peer.ip, peer.port);
    println!("   Unicode peer: {} ({})", unicode_peer.name, unicode_peer.id);
}

#[test]
fn test_timestamp_functions_precision() {
    // Test timestamp precision and consistency
    
    let timestamps: Vec<u64> = (0..10)
        .map(|_| {
            std::thread::sleep(std::time::Duration::from_millis(1));
            archsockrust::get_current_timestamp()
        })
        .collect();
    
    // Verify timestamps are monotonically increasing (allowing for equal values)
    for i in 1..timestamps.len() {
        assert!(
            timestamps[i] >= timestamps[i-1],
            "Timestamps should be monotonically increasing: {} >= {}",
            timestamps[i], timestamps[i-1]
        );
    }
    
    // Test formatting consistency
    for timestamp in &timestamps {
        let formatted = archsockrust::format_timestamp(*timestamp);
        
        // Should be in HH:MM:SS format
        assert!(formatted.len() >= 8, "Formatted time should be at least 8 characters");
        assert!(formatted.contains(':'), "Formatted time should contain colons");
        assert!(!formatted.contains("??"), "Should not contain error markers");
        
        // Verify format pattern (HH:MM:SS)
        let parts: Vec<&str> = formatted.split(':').collect();
        if parts.len() == 3 {
            // Validate each part is numeric and correct length
            assert!(parts[0].len() == 2 && parts[0].chars().all(|c| c.is_numeric()), "Hours should be 2 digits");
            assert!(parts[1].len() == 2 && parts[1].chars().all(|c| c.is_numeric()), "Minutes should be 2 digits");
            assert!(parts[2].len() == 2 && parts[2].chars().all(|c| c.is_numeric()), "Seconds should be 2 digits");
        }
    }
    
    println!("✅ Timestamp precision tests passed");
    println!("   Generated {} sequential timestamps", timestamps.len());
    println!("   First: {} -> {}", timestamps[0], archsockrust::format_timestamp(timestamps[0]));
    println!("   Last: {} -> {}", timestamps[timestamps.len()-1], archsockrust::format_timestamp(timestamps[timestamps.len()-1]));
}

#[test]
fn test_broadcast_address_calculation_edge_cases() {
    // Test broadcast address calculation with various edge cases
    
    struct TestCase {
        description: &'static str,
        ip: Ipv4Addr,
        netmask: Ipv4Addr,
        expected: Ipv4Addr,
    }
    
    let test_cases = vec![
        TestCase {
            description: "Single host /32",
            ip: Ipv4Addr::new(192, 168, 1, 1),
            netmask: Ipv4Addr::new(255, 255, 255, 255),
            expected: Ipv4Addr::new(192, 168, 1, 1),
        },
        TestCase {
            description: "Point-to-point /30",
            ip: Ipv4Addr::new(192, 168, 1, 1),
            netmask: Ipv4Addr::new(255, 255, 255, 252),
            expected: Ipv4Addr::new(192, 168, 1, 3),
        },
        TestCase {
            description: "Subnet /29",
            ip: Ipv4Addr::new(192, 168, 1, 10),
            netmask: Ipv4Addr::new(255, 255, 255, 248),
            expected: Ipv4Addr::new(192, 168, 1, 15),
        },
        TestCase {
            description: "Edge of class B",
            ip: Ipv4Addr::new(172, 31, 255, 254),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            expected: Ipv4Addr::new(172, 31, 255, 255),
        },
        TestCase {
            description: "Unusual netmask",
            ip: Ipv4Addr::new(10, 1, 2, 3),
            netmask: Ipv4Addr::new(255, 255, 240, 0),
            expected: Ipv4Addr::new(10, 1, 15, 255),
        },
    ];
    
    for test_case in test_cases {
        let ip_octets = test_case.ip.octets();
        let mask_octets = test_case.netmask.octets();
        
        let calculated = Ipv4Addr::new(
            ip_octets[0] | (!mask_octets[0]),
            ip_octets[1] | (!mask_octets[1]),
            ip_octets[2] | (!mask_octets[2]),
            ip_octets[3] | (!mask_octets[3]),
        );
        
        assert_eq!(
            calculated, test_case.expected,
            "Failed for {}: IP {} with mask {} should give {} but got {}",
            test_case.description, test_case.ip, test_case.netmask, test_case.expected, calculated
        );
        
        println!("✅ {}: {} + {} = {}", 
                test_case.description, test_case.ip, test_case.netmask, calculated);
    }
}

#[test]
fn test_protocol_constants() {
    // Test that protocol constants are correctly defined
    
    use archsockrust::protocol::discovery::{DISCOVERY_PORT, BROADCAST_ADDR, MULTICAST_ADDR};
    
    // Test discovery port
    assert_eq!(DISCOVERY_PORT, 6968, "Discovery port should be 6968");
    
    // Test broadcast address
    assert_eq!(BROADCAST_ADDR, "255.255.255.255", "Broadcast address should be universal");
    assert!(BROADCAST_ADDR.parse::<std::net::Ipv4Addr>().is_ok(), "Broadcast address should be valid IPv4");
    
    // Test multicast address
    assert_eq!(MULTICAST_ADDR, "224.0.0.251", "Multicast address should be mDNS");
    assert!(MULTICAST_ADDR.parse::<std::net::Ipv4Addr>().is_ok(), "Multicast address should be valid IPv4");
    
    // Verify multicast range (224.0.0.0 to 239.255.255.255)
    let multicast_ip: std::net::Ipv4Addr = MULTICAST_ADDR.parse().unwrap();
    let first_octet = multicast_ip.octets()[0];
    assert!(first_octet >= 224 && first_octet <= 239, "Should be in multicast range");
    
    println!("✅ Protocol constants validation passed");
    println!("   Discovery port: {}", DISCOVERY_PORT);
    println!("   Broadcast address: {}", BROADCAST_ADDR);
    println!("   Multicast address: {}", MULTICAST_ADDR);
}

#[tokio::test]
async fn test_event_system_basic_operations() {
    // Test basic event system functionality
    
    use archsockrust::events::EventManager;
    
    let event_manager = EventManager::new();
    let sender = event_manager.get_sender();
    
    // Verify sender is functional
    assert!(sender.is_closed() == false, "Event sender should not be closed initially");
    
    // Create test peer for events
    let test_peer = PeerInfo {
        id: "event-test-peer".to_string(),
        name: "Event Test Peer".to_string(),
        ip: "192.168.1.200".to_string(),
        port: 7777,
        last_seen: archsockrust::get_current_timestamp(),
    };
    
    // Test sending different event types
    let events_to_test = vec![
        P2PEvent::PeerDiscovered(test_peer.clone()),
        P2PEvent::PeerConnected(test_peer.clone()),
        P2PEvent::PeerDisconnected(test_peer.clone()),
        P2PEvent::Error("Test error message".to_string()),
    ];
    
    for event in events_to_test {
        let send_result = sender.send(event);
        assert!(send_result.is_ok(), "Should be able to send event");
    }
    
    println!("✅ Event system basic operations test passed");
    println!("   Successfully sent {} different event types", 4);
}

#[test]
fn test_ip_address_validation() {
    // Test IP address validation and parsing
    
    let valid_ips = vec![
        "127.0.0.1",
        "192.168.1.1",
        "10.0.0.1",
        "172.16.0.1", 
        "8.8.8.8",
        "255.255.255.255",
        "0.0.0.0",
    ];
    
    let invalid_ips = vec![
        "256.1.1.1",        // Invalid octet
        "192.168.1",        // Missing octet
        "192.168.1.1.1",    // Extra octet
        "",                 // Empty
        "hello",            // Non-numeric
        "192.168.1.-1",     // Negative
        "192.168.1.1.1",    // Too many octets
    ];
    
    println!("✅ Testing valid IP addresses:");
    for ip_str in valid_ips {
        let parse_result = ip_str.parse::<std::net::IpAddr>();
        assert!(parse_result.is_ok(), "Should be able to parse valid IP: {}", ip_str);
        println!("   ✓ {}", ip_str);
    }
    
    println!("✅ Testing invalid IP addresses:");
    for ip_str in invalid_ips {
        let parse_result = ip_str.parse::<std::net::IpAddr>();
        assert!(parse_result.is_err(), "Should reject invalid IP: {}", ip_str);
        println!("   ✗ {} (correctly rejected)", ip_str);
    }
}

#[test]
fn test_port_range_validation() {
    // Test port number validation and edge cases
    
    let valid_ports = vec![
        1,          // Minimum valid port
        80,         // HTTP
        443,        // HTTPS
        6968,       // Our discovery port
        6969,       // Our TCP port
        65535,      // Maximum port
    ];
    
    let edge_case_ports = vec![
        0,          // Reserved port
        1023,       // Last privileged port
        1024,       // First unprivileged port
        49152,      // Start of dynamic/private range
        65534,      // One below max
    ];
    
    println!("✅ Testing standard valid ports:");
    for port in valid_ports {
        // Test that port fits in u16 (implicit)
        assert!(port <= u16::MAX as u32, "Port should fit in u16: {}", port);
        println!("   ✓ Port {}", port);
    }
    
    println!("✅ Testing edge case ports:");
    for port in edge_case_ports {
        assert!(port <= u16::MAX as u32, "Port should fit in u16: {}", port);
        println!("   ✓ Port {} (edge case)", port);
    }
    
    // Test socket address creation with various ports
    let test_ip = "127.0.0.1";
    for &port in &[6968u16, 6969u16, 7000u16] {
        let addr_str = format!("{}:{}", test_ip, port);
        let socket_addr = addr_str.parse::<SocketAddr>();
        assert!(socket_addr.is_ok(), "Should create valid SocketAddr: {}", addr_str);
        
        let addr = socket_addr.unwrap();
        assert_eq!(addr.port(), port, "Port should match");
        println!("   ✓ SocketAddr: {}", addr);
    }
}

#[test]
fn test_string_encoding_and_unicode() {
    // Test string handling, especially Unicode support
    
    let test_strings = vec![
        ("ASCII only", "Hello World"),
        ("Latin-1", "Café señorita"),
        ("Chinese", "你好世界"),
        ("Japanese", "こんにちは"),
        ("Russian", "Привет мир"),
        ("Emoji", "Hello 👋 World 🌍"),
        ("Mixed", "Test 测试 🚀 Тест"),
        ("Empty", ""),
    ];
    
    for (description, test_str) in test_strings {
        // Test string length calculations
        let byte_len = test_str.len();
        let char_count = test_str.chars().count();
        
        println!("✅ {}: '{}' ({} bytes, {} chars)", description, test_str, byte_len, char_count);
        
        // Test that string is valid UTF-8 (should always be true in Rust)
        assert!(std::str::from_utf8(test_str.as_bytes()).is_ok(), "Should be valid UTF-8");
        
        // Test conversion to/from String
        let owned_string = test_str.to_string();
        assert_eq!(owned_string.as_str(), test_str, "String conversion should preserve content");
        
        // Test clone operation
        let cloned = owned_string.clone();
        assert_eq!(cloned, owned_string, "Clone should be equal");
        
        // Test that we can create PeerInfo with this string
        let peer = PeerInfo {
            id: format!("id-{}", description.to_lowercase().replace(' ', "-")),
            name: test_str.to_string(),
            ip: "127.0.0.1".to_string(),
            port: 6969,
            last_seen: archsockrust::get_current_timestamp(),
        };
        
        assert_eq!(peer.name, test_str, "PeerInfo should preserve Unicode in name");
    }
}

#[test]
fn test_error_conditions_and_edge_cases() {
    // Test various error conditions and edge cases
    
    println!("🛡️ Testing error conditions and edge cases:");
    
    // Test timestamp with edge values
    let edge_timestamps = vec![
        0u64,                    // Unix epoch
        1u64,                    // One second after epoch
        u32::MAX as u64,         // Year 2106 problem
        u64::MAX,                // Far future
    ];
    
    for timestamp in edge_timestamps {
        let formatted = archsockrust::format_timestamp(timestamp);
        assert!(!formatted.is_empty(), "Should format something for timestamp {}", timestamp);
        println!("   Timestamp {}: {}", timestamp, formatted);
    }
    
    // Test PeerInfo with edge case values
    let edge_peer = PeerInfo {
        id: "x".repeat(1000), // Very long ID
        name: "n".repeat(500), // Very long name
        ip: "255.255.255.255".to_string(), // Max IP
        port: u16::MAX as u32, // Max port
        last_seen: u64::MAX, // Max timestamp
    };
    
    assert_eq!(edge_peer.id.len(), 1000, "Should handle long IDs");
    assert_eq!(edge_peer.name.len(), 500, "Should handle long names");
    assert_eq!(edge_peer.port, u16::MAX as u32, "Should handle max port");
    
    println!("   ✅ Edge case PeerInfo created successfully");
    println!("      ID length: {}", edge_peer.id.len());
    println!("      Name length: {}", edge_peer.name.len());
    println!("      Port: {}", edge_peer.port);
    
    println!("✅ Error conditions and edge cases test completed");
}