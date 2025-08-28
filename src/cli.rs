use crate::app::AppState;
use crate::{P2PMessenger, P2PEvent};
use std::env;
use std::io::{self, Write};
use tokio::time::{sleep, Duration};

pub async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
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
    let mut app_state = AppState::new(messenger);

    // Event handler task - simplified for CLI
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            print_event(&event);
        }
    });

    // Auto-discovery task - use Arc clone
    let discovery_messenger = app_state.messenger.clone();
    tokio::spawn(async move {
        loop {
            let _ = discovery_messenger.discover_peers();
            discovery_messenger.cleanup_stale_peers();
            sleep(Duration::from_secs(5)).await;
        }
    });

    // Main CLI loop
    loop {
        print_menu();
        let choice = read_input("Choose option: ");

        match choice.trim() {
            "1" => list_discovered_peers(&mut app_state).await,
            "2" => list_connected_peers(&mut app_state).await,
            "3" => connect_to_peer(&mut app_state).await,
            "4" => send_message(&mut app_state).await,
            "5" => send_file(&mut app_state).await,
            "6" => disconnect_peer(&mut app_state).await,
            "7" => show_status(&mut app_state).await,
            "8" => force_discovery(&mut app_state),
            "h" | "help" => show_help(),
            "0" | "q" | "quit" => break,
            _ => println!("❌ Invalid option. Type 'h' for help."),
        }
    }

    app_state.messenger.stop().await;
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
    println!("• Basic: cargo run --bin archsockrust-cli -- \"Your Name\"");
    println!("• With ports: cargo run --bin archsockrust-cli -- \"Name\" 7000 7001");
    println!("• Interactive: cargo run --bin archsockrust-cli");
    println!("• TUI version: cargo run --bin archsockrust-tui -- \"Your Name\"");
    println!("• Discovery runs automatically every 5 seconds");
    println!("• Connect to peers before sending messages");
    println!("• Files are saved to 'recibidos/' directory");
    println!("\n🌐 Network:");
    println!("• UDP Discovery: configurable port (default 6968)");
    println!("• TCP Messages: configurable port (default 6969)");
    println!("• Multiple instances: use different ports");
    println!("• Works on local network without internet");
}

async fn show_status(app_state: &mut AppState) {
    app_state.refresh_peers().await;
    println!("\n📊 Status:");
    println!("• Name: {}", app_state.messenger.peer_name());
    println!("• ID: {}", app_state.messenger.peer_id());
    println!("• Local IP: {}", app_state.messenger.get_local_ip());
    println!("• Discovered peers: {}", app_state.discovered_peers.len());
    println!("• Connected peers: {}", app_state.connected_peers.len());
}

fn force_discovery(app_state: &mut AppState) {
    match app_state.force_discovery() {
        Ok(msg) => println!("🔍 {}", msg),
        Err(e) => println!("❌ {}", e),
    }
}

fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input
}

async fn list_discovered_peers(app_state: &mut AppState) {
    app_state.refresh_peers().await;
    println!("\n🔍 Discovered peers ({}):", app_state.discovered_peers.len());
    if app_state.discovered_peers.is_empty() {
        println!("   No peers discovered yet...");
        println!("   💡 Make sure other instances are running on the same network");
    } else {
        for (i, peer) in app_state.discovered_peers.iter().enumerate() {
            let status = if peer.is_connected { " [CONNECTED]" } else { "" };
            println!("   {}. {} ({}:{}){} - ID: {:.8}...", 
                i + 1, peer.name, peer.ip, peer.port, status, peer.id);
        }
    }
}

async fn list_connected_peers(app_state: &mut AppState) {
    app_state.refresh_peers().await;
    println!("\n🔗 Connected peers ({}):", app_state.connected_peers.len());
    if app_state.connected_peers.is_empty() {
        println!("   No peers connected");
        println!("   💡 Use option 3 to connect to discovered peers");
    } else {
        for (i, peer) in app_state.connected_peers.iter().enumerate() {
            println!("   {}. {} ({}:{}) - ID: {:.8}...", 
                i + 1, peer.name, peer.ip, peer.port, peer.id);
        }
    }
}

async fn connect_to_peer(app_state: &mut AppState) {
    app_state.refresh_peers().await;
    if app_state.discovered_peers.is_empty() {
        println!("❌ No peers discovered yet");
        return;
    }

    list_discovered_peers(app_state).await;
    let choice = read_input("Select peer number to connect: ");
    
    if let Ok(index) = choice.trim().parse::<usize>() {
        if index > 0 && index <= app_state.discovered_peers.len() {
            app_state.selected_peer = Some(index - 1);
            match app_state.connect_to_selected_peer().await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => println!("❌ {}", e),
            }
        } else {
            println!("❌ Invalid peer number");
        }
    }
}

async fn send_message(app_state: &mut AppState) {
    app_state.refresh_peers().await;
    if app_state.connected_peers.is_empty() {
        println!("❌ No peers connected");
        return;
    }

    list_connected_peers(app_state).await;
    let choice = read_input("Select peer number: ");
    
    if let Ok(index) = choice.trim().parse::<usize>() {
        if index > 0 && index <= app_state.connected_peers.len() {
            app_state.selected_peer = Some(index - 1);
            let message = read_input("Enter message: ");
            
            match app_state.send_text_message(message.trim().to_string()).await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => println!("❌ {}", e),
            }
        }
    }
}

async fn send_file(app_state: &mut AppState) {
    app_state.refresh_peers().await;
    if app_state.connected_peers.is_empty() {
        println!("❌ No peers connected");
        return;
    }

    list_connected_peers(app_state).await;
    let choice = read_input("Select peer number: ");
    
    if let Ok(index) = choice.trim().parse::<usize>() {
        if index > 0 && index <= app_state.connected_peers.len() {
            app_state.selected_peer = Some(index - 1);
            let file_path = read_input("Enter file path: ");
            
            match app_state.send_file(file_path.trim().to_string()).await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => println!("❌ {}", e),
            }
        }
    }
}

async fn disconnect_peer(app_state: &mut AppState) {
    app_state.refresh_peers().await;
    if app_state.connected_peers.is_empty() {
        println!("❌ No peers connected");
        return;
    }

    list_connected_peers(app_state).await;
    let choice = read_input("Select peer number to disconnect: ");
    
    if let Ok(index) = choice.trim().parse::<usize>() {
        if index > 0 && index <= app_state.connected_peers.len() {
            app_state.selected_peer = Some(index - 1);
            match app_state.disconnect_from_selected_peer().await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => println!("❌ {}", e),
            }
        }
    }
}

fn print_event(event: &P2PEvent) {
    // Simple event printing for CLI - just show the event occurred
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
            if let Some(content) = &message.content {
                match &content.content {
                    Some(crate::message_content::Content::Text(text_msg)) => {
                        println!("\n💬 {}: {}", message.sender_name, text_msg.text);
                        print!("Choose option: ");
                        io::stdout().flush().unwrap();
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}