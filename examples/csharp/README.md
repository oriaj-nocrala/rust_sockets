# ArchSockRust C# Interop Example

This example demonstrates how to create a C# application that can communicate with the Rust P2P messaging library using Protocol Buffers.

## Prerequisites

- .NET 8.0 SDK
- Protocol Buffers compiler (`protoc`) - already installed

## Setup

1. **Build the C# project:**
   ```bash
   cd examples/csharp
   dotnet build
   ```

2. **Run the C# peer:**
   ```bash
   dotnet run -- "CSharpPeer"
   ```

## How it works

The C# application implements the same P2P protocol as the Rust library:

### Protocol Compatibility

- **Discovery Protocol**: UDP broadcasts on port 6968 using `DiscoveryMessage` protobuf
- **P2P Communication**: TCP connections using `P2pMessage` protobuf  
- **Message Types**: Supports text messages and file transfers

### Features Implemented

- ✅ **Peer Discovery**: Listens for and responds to UDP discovery messages
- ✅ **TCP Server**: Accepts incoming P2P connections
- ✅ **Message Handling**: Processes text and file messages from Rust peers
- ✅ **File Saving**: Automatically saves received files to `received/` directory

### Testing Interoperability

1. **Start Rust peer:**
   ```bash
   cargo run --release -- "RustPeer"
   ```

2. **Start C# peer (in another terminal):**
   ```bash
   cd examples/csharp
   dotnet run -- "CSharpPeer"
   ```

3. **Expected behavior:**
   - Both peers will discover each other via UDP broadcasts
   - Rust peer can connect to C# peer and send messages
   - C# peer will receive and display messages from Rust
   - File transfers from Rust to C# work seamlessly

### Protocol Details

The C# implementation uses the same protobuf schemas (`proto/messages.proto` and `proto/discovery.proto`) ensuring binary compatibility with the Rust implementation.

**Message Flow:**
```
Rust Peer  ←→ UDP Discovery ←→  C# Peer
    ↓                              ↑
TCP Connection  ←→  P2pMessage  ←→  TCP Server
```

This demonstrates true cross-language P2P communication using Protocol Buffers as the serialization format.