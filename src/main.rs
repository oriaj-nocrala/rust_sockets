use archsockrust::{P2PMessenger, P2PEvent, message_content};
use std::env;
use std::io::{self, Write};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 ArchSockRust CLI - P2P Messenger Testing Tool");
    println!("===============================================");

    // Parse CLI args: [name] [tcp_port] [discovery_port]
    let args: Vec<String> = env::args().collect();
    let (name, tcp_port, discovery_port) = if args.len() > 1 {
        let name = args[1].clone();
        let tcp_port = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(6969);
        let discovery_port = args.get(3).and_then(|p| p.parse().ok()).unwrap_or(6968);
        (name, tcp_port, discovery_port)
    } else {
        print!("Enter your name: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        (input.trim().to_string(), 6969, 6968)
    };

    let mut messenger = P2PMessenger::with_ports(name, tcp_port, discovery_port)?;
    println!("✅ Created messenger with ID: {}", messenger.peer_id());
    println!("📡 Local IP: {}", messenger.get_local_ip());
    println!("🔍 Discovery port: {}, TCP port: {}", discovery_port, tcp_port);

    messenger.start().await?;
    println!("🚀 Messenger started! Auto-discovering peers every 5s...");

    let mut event_receiver = messenger.get_event_receiver().unwrap();
    
    let messenger_clone = std::sync::Arc::new(messenger);
    let messenger_for_events = messenger_clone.clone();

    // Event handler task
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            handle_event(event, &messenger_for_events).await;
        }
    });

    // Auto-discovery task
    let messenger_for_discovery = messenger_clone.clone();
    tokio::spawn(async move {
        loop {
            let _ = messenger_for_discovery.discover_peers();
            messenger_for_discovery.cleanup_stale_peers();
            sleep(Duration::from_secs(5)).await;
        }
    });

    // Main CLI loop
    loop {
        print_menu();
        let choice = read_input("Choose option: ");

        match choice.trim() {
            "1" => list_discovered_peers(&messenger_clone),
            "2" => list_connected_peers(&messenger_clone).await,
            "3" => connect_to_peer(&messenger_clone).await,
            "4" => send_message(&messenger_clone).await,
            "5" => send_file(&messenger_clone).await,
            "6" => disconnect_peer(&messenger_clone).await,
            "7" => show_status(&messenger_clone).await,
            "8" => force_discovery(&messenger_clone),
            "h" | "help" => show_help(),
            "0" | "q" | "quit" => break,
            _ => println!("❌ Invalid option. Type 'h' for help."),
        }
    }

    messenger_clone.stop().await;
    println!("👋 Goodbye!");
    Ok(())
}

fn print_menu() {
    println!("\n📋 Menu:");
    println!("1. List discovered peers     5. Send file");
    println!("2. List connected peers      6. Disconnect from peer");
    println!("3. Connect to peer           7. Show status");
    println!("4. Send text message         8. Force discovery");
    println!("h. Help                      0/q. Exit");
}

fn show_help() {
    println!("\n🆘 Help:");
    println!("This CLI tool helps test the ArchSockRust P2P library.");
    println!("\n🔧 Commands:");
    println!("• Basic: cargo run -- \"Your Name\"");
    println!("• With ports: cargo run -- \"Name\" 7000 7001");
    println!("• Interactive: cargo run");
    println!("• Discovery runs automatically every 5 seconds");
    println!("• Connect to peers before sending messages");
    println!("• Files are saved to 'recibidos/' directory");
    println!("\n🌐 Network:");
    println!("• UDP Discovery: configurable port (default 6968)");
    println!("• TCP Messages: configurable port (default 6969)");
    println!("• Multiple instances: use different ports");
    println!("• Works on local network without internet");
}

async fn show_status(messenger: &P2PMessenger) {
    let discovered = messenger.get_discovered_peers();
    let connected = messenger.get_connected_peers().await;
    
    println!("\n📊 Status:");
    println!("• Name: {}", messenger.peer_name());
    println!("• ID: {}", messenger.peer_id());
    println!("• Local IP: {}", messenger.get_local_ip());
    println!("• Discovered peers: {}", discovered.len());
    println!("• Connected peers: {}", connected.len());
}

fn force_discovery(messenger: &P2PMessenger) {
    match messenger.discover_peers() {
        Ok(_) => println!("🔍 Discovery broadcast sent!"),
        Err(e) => println!("❌ Discovery failed: {}", e),
    }
}

fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input
}

fn list_discovered_peers(messenger: &P2PMessenger) {
    let peers = messenger.get_discovered_peers();
    println!("\n🔍 Discovered peers ({}):", peers.len());
    if peers.is_empty() {
        println!("   No peers discovered yet...");
        println!("   💡 Make sure other instances are running on the same network");
    } else {
        for (i, peer) in peers.iter().enumerate() {
            println!("   {}. {} ({}:{}) - ID: {:.8}...", 
                i + 1, peer.name, peer.ip, peer.port, peer.id);
        }
    }
}

async fn list_connected_peers(messenger: &P2PMessenger) {
    let peers = messenger.get_connected_peers().await;
    println!("\n🔗 Connected peers ({}):", peers.len());
    if peers.is_empty() {
        println!("   No peers connected");
        println!("   💡 Use option 3 to connect to discovered peers");
    } else {
        for (i, peer) in peers.iter().enumerate() {
            println!("   {}. {} ({}:{}) - ID: {:.8}...", 
                i + 1, peer.name, peer.ip, peer.port, peer.id);
        }
    }
}

async fn connect_to_peer(messenger: &P2PMessenger) {
    let peers = messenger.get_discovered_peers();
    if peers.is_empty() {
        println!("❌ No peers discovered yet");
        return;
    }

    list_discovered_peers(messenger);
    let choice = read_input("Select peer number to connect: ");
    
    if let Ok(index) = choice.trim().parse::<usize>() {
        if index > 0 && index <= peers.len() {
            let peer = &peers[index - 1];
            match messenger.connect_to_peer(peer).await {
                Ok(()) => println!("✅ Connecting to {}...", peer.name),
                Err(e) => println!("❌ Failed to connect: {}", e),
            }
        } else {
            println!("❌ Invalid peer number");
        }
    }
}

async fn send_message(messenger: &P2PMessenger) {
    let peers = messenger.get_connected_peers().await;
    if peers.is_empty() {
        println!("❌ No peers connected");
        return;
    }

    list_connected_peers(messenger).await;
    let choice = read_input("Select peer number: ");
    
    if let Ok(index) = choice.trim().parse::<usize>() {
        if index > 0 && index <= peers.len() {
            let peer = &peers[index - 1];
            let message = read_input("Enter message: ");
            
            match messenger.send_text_message(&peer.id, message.trim().to_string()).await {
                Ok(()) => println!("✅ Message sent to {}", peer.name),
                Err(e) => println!("❌ Failed to send message: {}", e),
            }
        }
    }
}

async fn send_file(messenger: &P2PMessenger) {
    let peers = messenger.get_connected_peers().await;
    if peers.is_empty() {
        println!("❌ No peers connected");
        return;
    }

    list_connected_peers(messenger).await;
    let choice = read_input("Select peer number: ");
    
    if let Ok(index) = choice.trim().parse::<usize>() {
        if index > 0 && index <= peers.len() {
            let peer = &peers[index - 1];
            let file_path = read_input("Enter file path: ");
            
            match messenger.send_file(&peer.id, file_path.trim()).await {
                Ok(()) => println!("✅ File sent to {}", peer.name),
                Err(e) => println!("❌ Failed to send file: {}", e),
            }
        }
    }
}

async fn disconnect_peer(messenger: &P2PMessenger) {
    let peers = messenger.get_connected_peers().await;
    if peers.is_empty() {
        println!("❌ No peers connected");
        return;
    }

    list_connected_peers(messenger).await;
    let choice = read_input("Select peer number to disconnect: ");
    
    if let Ok(index) = choice.trim().parse::<usize>() {
        if index > 0 && index <= peers.len() {
            let peer = &peers[index - 1];
            match messenger.disconnect_peer(&peer.id).await {
                Ok(()) => println!("✅ Disconnected from {}", peer.name),
                Err(e) => println!("❌ Failed to disconnect: {}", e),
            }
        }
    }
}

async fn handle_event(event: P2PEvent, messenger: &P2PMessenger) {
    match event {
        P2PEvent::PeerDiscovered(peer) => {
            println!("\n🔍 Peer discovered: {} ({}:{}) ID:{:.8}...", 
                peer.name, peer.ip, peer.port, peer.id);
            print!("Choose option: ");
            io::stdout().flush().unwrap();
        }
        P2PEvent::PeerConnected(peer) => {
            println!("\n🔗 Peer connected: {} ({}:{}) ID:{:.8}...", 
                peer.name, peer.ip, peer.port, peer.id);
            print!("Choose option: ");
            io::stdout().flush().unwrap();
        }
        P2PEvent::PeerDisconnected(peer) => {
            println!("\n💔 Peer disconnected: {} ({}:{}) ID:{:.8}...", 
                peer.name, peer.ip, peer.port, peer.id);
            print!("Choose option: ");
            io::stdout().flush().unwrap();
        }
        P2PEvent::MessageReceived(message) => {
            let timestamp = format!("{}s", message.timestamp % 86400); // Simple seconds format
            
            if let Some(content) = &message.content {
                match &content.content {
                    Some(message_content::Content::Text(text_msg)) => {
                        println!("\n💬 [{}] {}: {}", timestamp, message.sender_name, text_msg.text);
                        print!("Choose option: ");
                        io::stdout().flush().unwrap();
                    }
                    Some(message_content::Content::File(file_msg)) => {
                        let size_kb = file_msg.data.len() / 1024;
                        match messenger.save_received_file(&message) {
                            Ok(path) => {
                                println!("\n📁 [{}] File from {}: {} ({} KB) -> {}", 
                                    timestamp, message.sender_name, file_msg.filename, size_kb, path);
                                print!("Choose option: ");
                                io::stdout().flush().unwrap();
                            }
                            Err(e) => {
                                println!("\n❌ Failed to save file {}: {}", file_msg.filename, e);
                                print!("Choose option: ");
                                io::stdout().flush().unwrap();
                            }
                        }
                    }
                    _ => {
                        println!("\n📨 [{}] Unknown message type from {}", timestamp, message.sender_name);
                        print!("Choose option: ");
                        io::stdout().flush().unwrap();
                    }
                }
            }
        }
        P2PEvent::FileTransferStarted { filename, size, .. } => {
            let size_kb = size / 1024;
            println!("\n📤 Sending file {} ({} KB)...", filename, size_kb);
        }
        P2PEvent::FileTransferCompleted { filename, .. } => {
            println!("\n✅ File sent successfully: {}", filename);
        }
        P2PEvent::FileTransferFailed { filename, error, .. } => {
            println!("\n❌ File transfer failed for {}: {}", filename, error);
        }
        P2PEvent::Error(error) => {
            println!("\n❌ Library error: {}", error);
        }
        _ => {}
    }
}