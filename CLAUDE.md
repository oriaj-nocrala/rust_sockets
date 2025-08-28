# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ArchSockRust is a modern P2P (peer-to-peer) messaging library written in Rust. It enables seamless local network communication with automatic peer discovery, supporting both text messages and file transfers without requiring a central server.

## Architecture

The library is built around a modular P2P architecture with clean separation of concerns:

### Core Components

- **Discovery Service** (`src/discovery/`): UDP broadcast-based peer discovery on port 6968
- **Peer Manager** (`src/peer/`): TCP connection management and message routing on port 6969  
- **Protocol Layer** (`src/protocol/`): Modern message serialization using `bincode` and `serde`
- **Event System** (`src/events/`): Async event-driven architecture with `tokio::mpsc`
- **Public API** (`src/lib.rs`): Clean, ergonomic interface for library consumers

### Key Features

- **Automatic Peer Discovery**: UDP broadcast to find local network peers
- **P2P Communication**: Direct TCP connections without intermediary servers
- **Modern Async**: Built on `tokio` for high-performance async I/O
- **Type-Safe Messaging**: Strong typing with `serde` serialization
- **Event-Driven**: Non-blocking event system for UI integration
- **File Transfer**: Seamless file sharing with progress tracking

### Message Protocol

- Uses `bincode` 2.0 for efficient binary serialization
- Messages contain UUID, timestamp, sender info, and typed content
- Supports text messages and file transfers with metadata
- Size-prefixed TCP protocol (8-byte header + payload)

## Common Commands

```bash
# Build the library and CLI
cargo build --release

# Run the CLI testing tool
cargo run --release

# Run with a name directly
cargo run --release -- "Alice"

# Build just the library (without CLI)
cargo build --lib --release

# Run tests (when available)
cargo test
```

## Library Usage

```rust
use archsockrust::{P2PMessenger, P2PEvent, MessageContent};

// Create messenger
let mut messenger = P2PMessenger::new("My Name".to_string())?;

// Start discovery and listening
messenger.start().await?;

// Get event receiver for UI updates
let mut events = messenger.get_event_receiver().unwrap();

// Discover peers
let peers = messenger.discover_peers()?;

// Connect and send message
messenger.connect_to_peer(&peers[0]).await?;
messenger.send_text_message(&peers[0].id, "Hello!".to_string()).await?;
```

## CLI Testing Tool

The project includes a built-in CLI for testing the library:

```bash
# Start the CLI
cargo run --release

# Or with a name
cargo run --release -- "YourName"
```

**CLI Features:**
- Automatic peer discovery every 5 seconds
- Real-time event notifications
- Interactive peer management
- File transfer testing
- Network status monitoring

**CLI Commands:**
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

## Development Notes

- Uses `tokio::sync::Mutex` for async-safe concurrent access
- All blocking operations are async to maintain responsiveness  
- Event system allows UI frameworks to react to network events
- Received files are saved to `recibidos/` directory
- Supports concurrent connections to multiple peers
- Built with clean code principles and modular design
- CLI tool is perfect for testing library functionality during development