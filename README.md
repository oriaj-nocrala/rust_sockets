# 🦀 ArchSockRust - File Transfer Socket Application

A simple yet robust TCP socket application written in Rust for transferring text messages and files between devices on a network.

## ✨ Features

- **Bidirectional Communication**: Send and receive messages/files between any two devices
- **File Transfer**: Send any type of file over the network
- **Text Messaging**: Send plain text messages
- **Auto IP Detection**: Automatically detects your local IP address
- **Simple CLI Interface**: Easy-to-use command-line interface

## 🚀 Quick Start

### Prerequisites

- Rust (latest stable version)
- Network connectivity between devices

### Building

```bash
cargo build --release
```

### Running

```bash
cargo run
```

## 📖 How to Use

1. **Run the application** on both devices
2. **Choose mode**:
   - `0` to send (client mode)
   - `1` to receive (server mode)

### Sending Files/Messages

1. Choose option `0` (send)
2. Enter the IP address of the receiving device
3. Choose what to send:
   - `0` for text message
   - `1` for file
4. Follow the prompts

### Receiving Files/Messages

1. Choose option `1` (receive)
2. The application will listen on port `6969`
3. Files are saved to the `recibidos/` directory
4. Text messages are displayed in the console

## 🔧 Technical Details

- **Protocol**: TCP over port 6969
- **Serialization**: Uses `bincode` for efficient binary serialization
- **File Handling**: Creates `recibidos/` directory for incoming files
- **Error Handling**: Graceful error handling for network and file operations

## 📦 Dependencies

- `bincode` - For binary serialization
- `local-ip-address` - For automatic IP detection

## 🤝 Contributing

Feel free to open issues or submit pull requests to improve the application!

## 📄 License

This project is open source and available under the MIT License.