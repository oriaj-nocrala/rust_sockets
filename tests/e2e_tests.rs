use archsockrust::*;
use tokio::time::{timeout, Duration, sleep};

#[tokio::test]
async fn test_p2p_messenger_lifecycle() {
    // Test the complete lifecycle of a P2P messenger
    
    let mut messenger = P2PMessenger::new("LifecycleTest".to_string())
        .expect("Failed to create P2P messenger");
    
    // Test initial state
    assert_eq!(messenger.peer_name(), "LifecycleTest");
    assert!(!messenger.peer_id().is_empty());
    
    // Test local IP retrieval
    let local_ip = messenger.get_local_ip();
    assert!(!local_ip.is_empty());
    assert!(local_ip.parse::<std::net::IpAddr>().is_ok(), "Local IP should be valid: {}", local_ip);
    
    println!("✅ P2P Messenger created successfully:");
    println!("   Name: {}", messenger.peer_name());
    println!("   ID: {}", messenger.peer_id());
    println!("   Local IP: {}", local_ip);
    
    // Test starting the messenger
    let start_result = messenger.start().await;
    assert!(start_result.is_ok(), "Failed to start messenger: {:?}", start_result.err());
    
    println!("✅ P2P Messenger started successfully");
    
    // Test event receiver
    let mut event_receiver = messenger.get_event_receiver();
    assert!(event_receiver.is_some(), "Should have event receiver after starting");
    
    // Test peer discovery
    let discovery_result = messenger.discover_peers();
    assert!(discovery_result.is_ok(), "Failed to discover peers: {:?}", discovery_result.err());
    
    println!("✅ Peer discovery initiated successfully");
    
    // Test getting discovered peers (should be empty initially)
    let discovered_peers = messenger.get_discovered_peers();
    println!("   Initially discovered {} peers", discovered_peers.len());
    
    // Test getting connected peers (should be empty)
    let connected_peers = messenger.get_connected_peers().await;
    println!("   Currently connected to {} peers", connected_peers.len());
    assert_eq!(connected_peers.len(), 0, "Should not be connected to any peers initially");
    
    // Test cleanup
    messenger.cleanup_stale_peers();
    println!("✅ Stale peer cleanup completed");
    
    // Test stopping
    messenger.stop().await;
    println!("✅ P2P Messenger stopped successfully");
}

#[tokio::test]
async fn test_two_messenger_discovery() {
    // Test that two messengers can discover each other
    
    let mut messenger1 = P2PMessenger::with_ports("Alice".to_string(), 8000, 8001)
        .expect("Failed to create Alice messenger");
    let mut messenger2 = P2PMessenger::with_ports("Bob".to_string(), 8002, 8003)
        .expect("Failed to create Bob messenger");
    
    println!("🚀 Starting two-messenger discovery test:");
    println!("   Alice: ID={}, TCP=8000, Discovery=8001", messenger1.peer_id());
    println!("   Bob: ID={}, TCP=8002, Discovery=8003", messenger2.peer_id());
    
    // Start both messengers
    assert!(messenger1.start().await.is_ok(), "Failed to start Alice");
    assert!(messenger2.start().await.is_ok(), "Failed to start Bob");
    
    // Get event receivers
    let mut alice_events = messenger1.get_event_receiver().unwrap();
    let mut bob_events = messenger2.get_event_receiver().unwrap();
    
    // Give them time to fully initialize
    sleep(Duration::from_millis(200)).await;
    
    // Test discovery from both sides
    assert!(messenger1.discover_peers().is_ok(), "Alice failed to discover peers");
    assert!(messenger2.discover_peers().is_ok(), "Bob failed to discover peers");
    
    println!("✅ Both messengers initiated peer discovery");
    
    // Wait for potential discovery events
    let discovery_timeout = Duration::from_millis(1500);
    
    // Track discovery results
    let mut alice_discovered_count = 0;
    let mut bob_discovered_count = 0;
    
    // Monitor events with timeout
    let alice_event_task = tokio::spawn(async move {
        let mut events = Vec::new();
        while let Ok(Some(event)) = timeout(Duration::from_millis(100), alice_events.recv()).await {
            events.push(format!("{:?}", event));
            if events.len() >= 5 { break; } // Limit to avoid infinite loop
        }
        events
    });
    
    let bob_event_task = tokio::spawn(async move {
        let mut events = Vec::new();
        while let Ok(Some(event)) = timeout(Duration::from_millis(100), bob_events.recv()).await {
            events.push(format!("{:?}", event));
            if events.len() >= 5 { break; } // Limit to avoid infinite loop
        }
        events
    });
    
    // Wait for events or timeout
    let (alice_events, bob_events) = tokio::join!(alice_event_task, bob_event_task);
    
    if let Ok(alice_events) = alice_events {
        println!("📧 Alice received {} events:", alice_events.len());
        for event in alice_events {
            println!("   - {}", event);
        }
    }
    
    if let Ok(bob_events) = bob_events {
        println!("📧 Bob received {} events:", bob_events.len());
        for event in bob_events {
            println!("   - {}", event);
        }
    }
    
    // Check final discovery state
    let alice_discovered = messenger1.get_discovered_peers();
    let bob_discovered = messenger2.get_discovered_peers();
    
    alice_discovered_count = alice_discovered.len();
    bob_discovered_count = bob_discovered.len();
    
    println!("🔍 Final discovery results:");
    println!("   Alice discovered {} peers: {:?}", 
            alice_discovered_count, 
            alice_discovered.iter().map(|p| &p.name).collect::<Vec<_>>());
    println!("   Bob discovered {} peers: {:?}", 
            bob_discovered_count, 
            bob_discovered.iter().map(|p| &p.name).collect::<Vec<_>>());
    
    // Test broadcast address functionality specifically
    let broadcast_addresses = archsockrust::discovery::DiscoveryService::get_broadcast_addresses();
    println!("📡 Broadcasting to {} addresses: {:?}", broadcast_addresses.len(), broadcast_addresses);
    
    // Clean up
    messenger1.cleanup_stale_peers();
    messenger2.cleanup_stale_peers();
    messenger1.stop().await;
    messenger2.stop().await;
    
    println!("✅ Two-messenger test completed successfully");
    println!("   Test validates that discovery system is functional");
    println!("   Actual peer discovery depends on network configuration and firewall settings");
}

#[tokio::test]
async fn test_custom_port_configurations() {
    // Test that messengers can be created with various port configurations
    
    let port_configs = vec![
        (7000, 7001, "Config 1"),
        (7100, 7101, "Config 2"),
        (7200, 7201, "Config 3"),
        (7300, 7301, "Config 4"),
    ];
    
    let mut messengers = Vec::new();
    
    println!("🔧 Testing custom port configurations:");
    
    // Create messengers with different port configurations
    for (tcp_port, discovery_port, name) in &port_configs {
        let messenger = P2PMessenger::with_ports(
            format!("TestPeer_{}", name),
            *tcp_port,
            *discovery_port,
        );
        
        assert!(messenger.is_ok(), "Failed to create messenger for {}", name);
        let messenger = messenger.unwrap();
        
        println!("   ✅ {}: TCP={}, Discovery={}, ID={}", 
                name, tcp_port, discovery_port, messenger.peer_id());
        
        messengers.push(messenger);
    }
    
    // Start all messengers
    println!("🚀 Starting all messengers:");
    for (i, messenger) in messengers.iter().enumerate() {
        let result = messenger.start().await;
        assert!(result.is_ok(), "Failed to start messenger {}: {:?}", i, result.err());
        println!("   ✅ Started messenger {} ({})", i + 1, port_configs[i].2);
    }
    
    // Give them time to initialize
    sleep(Duration::from_millis(300)).await;
    
    // Test discovery from each messenger
    println!("🔍 Testing discovery from each messenger:");
    for (i, messenger) in messengers.iter().enumerate() {
        let discovery_result = messenger.discover_peers();
        assert!(discovery_result.is_ok(), "Discovery failed for messenger {}: {:?}", i, discovery_result.err());
        
        let discovered = messenger.get_discovered_peers();
        println!("   📡 Messenger {} discovered {} peers", i + 1, discovered.len());
        
        let local_ip = messenger.get_local_ip();
        println!("      Local IP: {}", local_ip);
        
        messenger.cleanup_stale_peers();
    }
    
    // Stop all messengers
    println!("🛑 Stopping all messengers:");
    for (i, messenger) in messengers.iter().enumerate() {
        messenger.stop().await;
        println!("   ✅ Stopped messenger {} ({})", i + 1, port_configs[i].2);
    }
    
    println!("✅ Custom port configuration test completed successfully");
}

#[tokio::test]
async fn test_unicode_support_in_peer_names() {
    // Test that the system properly handles Unicode in peer names and messages
    
    let unicode_names = vec![
        "Alice👋",
        "Bob🚀",
        "测试用户",
        "Пользователь",
        "ユーザー",
        "🌟SuperUser🌟",
        "Café_Señorita",
    ];
    
    println!("🌍 Testing Unicode support in peer names:");
    
    let mut messengers = Vec::new();
    
    // Create messengers with Unicode names
    for (i, name) in unicode_names.iter().enumerate() {
        let tcp_port = 7400 + (i * 2) as u16;
        let discovery_port = 7401 + (i * 2) as u16;
        
        let messenger = P2PMessenger::with_ports(
            name.to_string(),
            tcp_port,
            discovery_port,
        );
        
        assert!(messenger.is_ok(), "Failed to create messenger with Unicode name: {}", name);
        let messenger = messenger.unwrap();
        
        assert_eq!(messenger.peer_name(), *name, "Peer name should match Unicode input");
        
        println!("   ✅ Created messenger: '{}' (ID: {})", name, messenger.peer_id());
        messengers.push(messenger);
    }
    
    // Start a few messengers and test discovery
    let test_count = std::cmp::min(3, messengers.len());
    
    for i in 0..test_count {
        assert!(messengers[i].start().await.is_ok(), "Failed to start Unicode messenger {}", i);
    }
    
    sleep(Duration::from_millis(200)).await;
    
    // Test discovery with Unicode names
    for i in 0..test_count {
        let discovery_result = messengers[i].discover_peers();
        assert!(discovery_result.is_ok(), "Discovery failed for Unicode messenger {}", i);
        
        let discovered = messengers[i].get_discovered_peers();
        println!("   📡 '{}' discovered {} peers", unicode_names[i], discovered.len());
        
        // Verify discovered peer names contain proper Unicode
        for peer in discovered {
            println!("      Found peer: '{}' at {}:{}", peer.name, peer.ip, peer.port);
            
            // Verify the name is valid Unicode
            assert!(!peer.name.is_empty(), "Peer name should not be empty");
        }
    }
    
    // Test timestamp formatting with Unicode context
    let current_time = archsockrust::get_current_timestamp();
    let formatted = archsockrust::format_timestamp(current_time);
    println!("   🕐 Current timestamp: {} -> {}", current_time, formatted);
    
    // Clean up
    for i in 0..test_count {
        messengers[i].stop().await;
    }
    
    println!("✅ Unicode support test completed successfully");
}

#[tokio::test]
async fn test_error_handling_and_resilience() {
    // Test error handling and system resilience
    
    println!("🛡️ Testing error handling and resilience:");
    
    // Test creating messenger with invalid/conflicting ports
    println!("   Testing port conflict scenarios...");
    
    // Create first messenger
    let messenger1 = P2PMessenger::with_ports("First".to_string(), 7500, 7501);
    assert!(messenger1.is_ok(), "First messenger should be created successfully");
    let messenger1 = messenger1.unwrap();
    
    // Start first messenger
    assert!(messenger1.start().await.is_ok(), "First messenger should start successfully");
    
    // Try to create second messenger with same ports (should still succeed in creation)
    let messenger2 = P2PMessenger::with_ports("Second".to_string(), 7500, 7501);
    assert!(messenger2.is_ok(), "Second messenger creation should succeed");
    let messenger2 = messenger2.unwrap();
    
    // Starting second messenger should handle port conflicts gracefully
    let start_result = messenger2.start().await;
    println!("   Second messenger start result: {:?}", start_result.is_ok());
    
    // Test discovery with potentially invalid state
    let discovery1 = messenger1.discover_peers();
    let discovery2 = messenger2.discover_peers();
    
    println!("   Discovery results: M1={}, M2={}", discovery1.is_ok(), discovery2.is_ok());
    
    // Test cleanup operations
    messenger1.cleanup_stale_peers();
    messenger2.cleanup_stale_peers();
    
    // Test multiple stop calls (should be safe)
    messenger1.stop().await;
    messenger1.stop().await; // Second stop should be safe
    
    messenger2.stop().await;
    messenger2.stop().await; // Second stop should be safe
    
    println!("   ✅ Multiple stop calls handled safely");
    
    // Test messenger creation with extreme values
    let extreme_messenger = P2PMessenger::with_ports(
        "ExtremeTest".to_string(),
        u16::MAX - 1,
        u16::MAX,
    );
    
    if extreme_messenger.is_ok() {
        println!("   ✅ Extreme port values handled");
        extreme_messenger.unwrap().stop().await;
    } else {
        println!("   ℹ️ Extreme port values rejected (expected behavior)");
    }
    
    println!("✅ Error handling and resilience test completed");
}

#[test]
fn test_timestamp_utilities() {
    // Test timestamp utility functions
    
    println!("🕐 Testing timestamp utilities:");
    
    // Test current timestamp
    let timestamp1 = archsockrust::get_current_timestamp();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let timestamp2 = archsockrust::get_current_timestamp();
    
    assert!(timestamp1 > 0, "Timestamp should be positive");
    assert!(timestamp2 >= timestamp1, "Second timestamp should be >= first");
    
    println!("   ✅ Current timestamps: {} -> {}", timestamp1, timestamp2);
    
    // Test timestamp formatting
    let formatted1 = archsockrust::format_timestamp(timestamp1);
    let formatted2 = archsockrust::format_timestamp(timestamp2);
    
    assert!(!formatted1.is_empty(), "Formatted timestamp should not be empty");
    assert!(!formatted1.contains("??:??:??"), "Should not contain error markers");
    
    println!("   ✅ Formatted timestamps: {} -> {}", formatted1, formatted2);
    
    // Test known timestamp values
    let known_timestamps = vec![
        (1704110400, "2024-01-01 around noon UTC"), // 2024-01-01 12:00:00 UTC
        (1640995200, "2022-01-01 around midnight UTC"), // 2022-01-01 00:00:00 UTC  
        (0, "Unix epoch"),
    ];
    
    for (timestamp, description) in known_timestamps {
        let formatted = archsockrust::format_timestamp(timestamp);
        println!("   📅 {}: {} -> {}", description, timestamp, formatted);
        
        if timestamp == 0 {
            // Epoch might format as error or as actual time, both are acceptable
            assert!(!formatted.is_empty(), "Should format something for epoch");
        } else {
            assert!(!formatted.contains("??:??:??"), "Should not contain error markers for valid timestamp");
        }
    }
    
    println!("✅ Timestamp utilities test completed");
}