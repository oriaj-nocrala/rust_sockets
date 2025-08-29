# ArchSockRust .NET Wrapper

C# P/Invoke wrapper for the ArchSockRust P2P messaging library.

## Structure

- **ArchSockRust.Interop** - P/Invoke wrapper library
- **ArchSockRust.TestApp** - Console test application

## Requirements

- .NET 8.0 or later
- ArchSockRust native library (`libarchsockrust.so` on Linux, `archsockrust.dll` on Windows)

## Building

### Linux (with dotnet installed)
```bash
cd ArchSockRust.NET
dotnet build
dotnet run --project ArchSockRust.TestApp -- "YourName"
```

### Without dotnet (manual setup)
The project structure is ready for:
- Visual Studio 2022 on Windows
- JetBrains Rider
- VS Code with C# extension

## Usage Example

```csharp
using ArchSockRust.Interop;

// Create messenger
using var messenger = new P2PMessenger("MyApp");

// Subscribe to events
messenger.PeerDiscovered += (s, e) => 
    Console.WriteLine($"Found peer: {e.PeerName}");
    
messenger.MessageReceived += (s, e) => 
    Console.WriteLine($"{e.PeerName}: {e.Message}");

// Start
messenger.Start();

// Discover peers
messenger.DiscoverPeers();

// Connect and send message
messenger.ConnectToPeer("peer_id");
messenger.SendTextMessage("peer_id", "Hello from C#!");
```

## API Overview

### Core Methods
- `new P2PMessenger(name)` - Create messenger
- `Start()` - Begin listening and discovery
- `Stop()` - Stop messenger
- `DiscoverPeers()` - Broadcast discovery
- `ConnectToPeer(peerId)` - Connect to peer
- `SendTextMessage(peerId, message)` - Send text
- `SendFile(peerId, filePath)` - Send file

### Properties
- `PeerName` - This peer's name
- `PeerId` - This peer's unique ID
- `LocalIp` - Local IP address
- `DiscoveredPeersCount` - Number of discovered peers
- `ConnectedPeersCount` - Number of connected peers

### Events
- `PeerDiscovered` - New peer found
- `PeerConnected` - Peer connection established
- `PeerDisconnected` - Peer disconnected
- `MessageReceived` - Text message received
- `Error` - Error occurred

## Native Library Location

The wrapper expects the native library at:
- Linux: `runtimes/linux-x64/native/libarchsockrust.so`
- Windows: `runtimes/win-x64/native/archsockrust.dll`

The build system automatically copies from `../../../target/release/` to the output directory.

## Testing

Run multiple instances to test P2P communication:

Terminal 1:
```bash
dotnet run --project ArchSockRust.TestApp -- "Alice"
```

Terminal 2:
```bash  
dotnet run --project ArchSockRust.TestApp -- "Bob"
```

Both should discover each other and allow messaging.