# 🦀 ArchSockRust - Modern P2P Messaging Library

A modern peer-to-peer messaging library written in Rust for seamless local network communication. Features automatic peer discovery, direct P2P connections, and real-time messaging without requiring central servers.

## ✨ Features

### 🌐 **P2P Architecture**
- **Automatic Peer Discovery**: Find peers on local network via UDP broadcast
- **Direct P2P Connections**: No central server required
- **Seamless Integration**: Clean API for any UI framework

### 📡 **Modern Communication**
- **Real-time Messaging**: Async event-driven architecture
- **File Transfers**: Send any file type with progress tracking
- **Cross-Language Protocol**: Protocol Buffers for universal compatibility
- **Concurrent Connections**: Connect to multiple peers simultaneously

### 🛠️ **Developer Experience**
- **Modern TUI Interface**: Beautiful terminal UI with ratatui
- **Traditional CLI Tool**: Menu-driven testing and debugging
- **Event System**: React to network events in real-time
- **Clean Architecture**: Modular, async-first design
- **Comprehensive Logging**: Full visibility into P2P operations

## 🚀 Quick Start

### Prerequisites

- Rust (latest stable version)
- Local network connectivity

### Using as Library

Add to your `Cargo.toml`:

```toml
[dependencies]
archsockrust = { git = "https://github.com/oriaj-nocrala/rust_sockets.git" }
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use archsockrust::{P2PMessenger, P2PEvent, message_content};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create messenger
    let mut messenger = P2PMessenger::new("Alice".to_string())?;
    
    // Start discovery and listening
    messenger.start().await?;
    
    // Get event receiver for UI updates
    let mut events = messenger.get_event_receiver().unwrap();
    
    // Handle events
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            match event {
                P2PEvent::PeerDiscovered(peer) => {
                    println!("Found peer: {}", peer.name);
                }
                P2PEvent::MessageReceived(message) => {
                    if let Some(content) = &message.content {
                        if let Some(message_content::Content::Text(text_msg)) = &content.content {
                            println!("{}: {}", message.sender_name, text_msg.text);
                        }
                    }
                }
                _ => {}
            }
        }
    });
    
    // Discover and connect to peers
    let peers = messenger.discover_peers()?;
    if !peers.is_empty() {
        messenger.connect_to_peer(&peers[0]).await?;
        messenger.send_text_message(&peers[0].id, "Hello!".to_string()).await?;
    }
    
    Ok(())
}
```

### Testing Tools

The library includes both modern TUI and traditional CLI for testing and development:

```bash
# Clone and build
git clone https://github.com/oriaj-nocrala/rust_sockets.git
cd rust_sockets
cargo build --release

# Modern TUI (recommended) - Beautiful visual interface
cargo run --bin archsockrust-tui -- "Alice"

# Traditional CLI - Menu-driven interface
cargo run --bin archsockrust-cli -- "Alice"

# Or interactive mode
cargo run --bin archsockrust-tui  # Default name: "TUI User"
cargo run --bin archsockrust-cli  # Prompts for name
```

#### 🖥️ **TUI Interface Features**
- **3-panel layout**: Peers | Messages | Input
- **Real-time updates**: Live peer discovery and messaging
- **Keyboard navigation**: Tab/arrows, intuitive shortcuts
- **Visual feedback**: Color-coded status and message types
- **Interactive help**: Press `h` for comprehensive guide
- **Modern experience**: No more menu numbers - direct interaction

#### 📋 **TUI Controls**
- `Tab/Shift+Tab`: Switch panels
- `↑/↓`: Select peers  
- `c`: Connect, `d`: Disconnect, `f`: Send file
- `h`: Help popup, `q`: Quit
- `Enter`: Send message (in input panel)
- `F5`: Force discovery

## 📖 How to Use

### 1. **Discovery Phase**
- Start the application on multiple devices
- Peers automatically discover each other via UDP broadcast
- No manual IP configuration needed

### 2. **Connection Phase** 
- Select discovered peers from the list
- Establish direct P2P TCP connections
- Real-time connection status updates

### 3. **Communication Phase**
- Send text messages instantly
- Transfer files with progress tracking
- Receive real-time notifications

### 4. **Interface Options**

#### **TUI (Modern - Recommended)**
Visual interface with real-time panels:
- **Left panel**: Live peer list with status indicators
- **Right panels**: Message history + input field
- **Keyboard shortcuts**: Direct actions (c/d/f/h/q)
- **Visual feedback**: Color-coded messages and status

#### **CLI (Traditional)**
Menu-driven interface:
- `1`: List discovered peers
- `2`: List connected peers
- `3`: Connect to peer
- `4`: Send text message
- `5`: Send file
- `6`: Disconnect peer
- `7`: Show status
- `8`: Force discovery
- `h`: Help
- `0/q`: Exit

## 🏗️ Architecture

### Core Components

- **Discovery Service** (`src/discovery/`): UDP broadcast peer discovery (configurable port)
- **Peer Manager** (`src/peer/`): TCP connection management (configurable port)
- **Protocol Layer** (`src/protocol/`): Message serialization with Protocol Buffers
- **Event System** (`src/events/`): Async event notifications
- **Public API** (`src/lib.rs`): Clean library interface

### Network Protocol

- **Discovery**: UDP broadcast (default port 6968, configurable)
- **Messaging**: Direct TCP P2P (default port 6969, configurable)
- **Serialization**: Efficient binary with Protocol Buffers
- **Message Format**: Size-prefixed with UUID, timestamp, and typed protobuf content

## 🔧 Technical Details

- **Language**: Rust 2021 Edition
- **Async Runtime**: Tokio for high-performance I/O
- **Serialization**: Protocol Buffers with prost for cross-language compatibility
- **Concurrency**: Tokio Mutex for async-safe operations
- **Error Handling**: Comprehensive error types with thiserror
- **ID System**: UUID v4 for unique peer identification
- **Build System**: Native protobuf code generation via build.rs

## 📦 Dependencies

- `tokio` - Async runtime
- `prost` - Protocol Buffers implementation
- `prost-types` - Common protobuf types
- `uuid` - Unique identifiers
- `local-ip-address` - Network detection
- `thiserror` - Error handling
- `ratatui` - Modern terminal UI framework
- `crossterm` - Cross-platform terminal control

### Build Dependencies

- `prost-build` - Protocol Buffers code generation
- `protoc` - Protocol Buffers compiler (system dependency)

## 🚦 Getting Started for Developers

### Prerequisites

- Rust (latest stable)
- Protocol Buffers compiler (`protoc`)

```bash
# Ubuntu/Debian
sudo apt install protobuf-compiler

# macOS
brew install protobuf

# Windows
# Download from https://github.com/protocolbuffers/protobuf/releases
```

### Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/oriaj-nocrala/rust_sockets.git
   cd rust_sockets
   ```

2. **Test the library**
   ```bash
   # Terminal 1 - Modern TUI
   cargo run --bin archsockrust-tui -- "Alice"
   
   # Terminal 2 - Different machine or custom ports
   cargo run --bin archsockrust-tui -- "Bob" 7000 7001
   
   # Or use traditional CLI
   cargo run --bin archsockrust-cli -- "Alice"
   ```

3. **Watch them discover and connect automatically!**

4. **Test C# interoperability**
   ```bash
   # Terminal 1 - Rust
   cargo run --release -- "RustPeer"
   
   # Terminal 2 - C#
   cd examples/csharp
   dotnet run -- "CSharpPeer"
   ```

5. **Integrate into your project**
   - Use the library API for your UI
   - Handle P2PEvent for real-time updates
   - Customize peer discovery and messaging
   - Generate protobuf bindings for other languages

## 🌐 Cross-Language Interoperability

### C# Integration

This library now supports **full C# interoperability** via Protocol Buffers:

- ✅ **Shared Protocol**: Both Rust and C# use same `.proto` schemas
- ✅ **Binary Compatible**: Native protobuf serialization works across languages  
- ✅ **Complete Example**: Working C# application in `examples/csharp/`
- ✅ **Real-time Communication**: C# peers can discover and communicate with Rust peers

### Supported Languages

Any language with Protocol Buffers support can interoperate:

- **C#** - Full example provided
- **Python** - Generate bindings with `protoc --python_out`
- **Java** - Generate bindings with `protoc --java_out`
- **Go** - Generate bindings with `protoc --go_out`
- **JavaScript/TypeScript** - Generate bindings with `protoc --js_out`

### Creating Language Bindings

```bash
# Generate C# bindings
protoc --csharp_out=. proto/messages.proto proto/discovery.proto

# Generate Python bindings  
protoc --python_out=. proto/messages.proto proto/discovery.proto

# Generate Java bindings
protoc --java_out=. proto/messages.proto proto/discovery.proto
```

## 🖥️ Interface Comparison

| Feature | TUI (Modern) | CLI (Traditional) |
|---------|-------------|-------------------|
| **Experience** | Visual, real-time | Menu-driven |
| **Learning Curve** | Intuitive | Requires memorization |
| **Peer Discovery** | Live updates | Manual refresh |
| **Message Display** | Scrollable history | Print to stdout |
| **Navigation** | Keyboard shortcuts | Number selections |
| **Status Info** | Always visible | On-demand |
| **Help System** | Interactive popup | Static text |
| **Multitasking** | Simultaneous actions | Sequential menu |
| **Modern Feel** | Native terminal app | Classic CLI tool |

### 💡 **Recommendations**

- **Use TUI for**: Development, testing, demonstrations, daily use
- **Use CLI for**: Automation, scripting, CI/CD, headless environments
- **Both share**: Same core functionality, identical P2P protocol, cross-language compatibility

### 📚 **Additional Documentation**

- **[TUI_USAGE.md](TUI_USAGE.md)**: Comprehensive TUI guide with screenshots and troubleshooting
- **[CLAUDE.md](CLAUDE.md)**: Development instructions and architecture details  
- **[proto/](proto/)**: Protocol Buffers schemas for cross-language integration

## 🤝 Contributing

Contributions welcome! This library is designed for:
- **Local network messaging apps** - Chat applications for LANs
- **P2P file sharing tools** - Direct file transfers without servers  
- **Distributed applications** - Cross-language P2P networks
- **Real-time collaboration software** - Multi-platform team tools

## 📄 License

MIT License - feel free to use in your projects!