using System;
using System.Net;
using System.Net.Sockets;
using System.Threading.Tasks;
using Google.Protobuf;
using Archsockrust;

namespace ArchSockRustCSharpExample
{
    class Program
    {
        private static readonly int DISCOVERY_PORT = 6968;
        private static readonly int TCP_PORT = 6970; // Different port to avoid conflicts
        private static string peerId = Guid.NewGuid().ToString();
        private static string peerName = "CSharpPeer";

        static async Task Main(string[] args)
        {
            Console.WriteLine("🔷 ArchSockRust C# Interop Example");
            Console.WriteLine("===================================");
            
            if (args.Length > 0)
                peerName = args[0];

            Console.WriteLine($"Peer Name: {peerName}");
            Console.WriteLine($"Peer ID: {peerId}");
            Console.WriteLine($"TCP Port: {TCP_PORT}");
            Console.WriteLine();

            // Start UDP discovery listener
            _ = Task.Run(StartDiscoveryListener);
            
            // Start TCP server
            _ = Task.Run(StartTcpServer);
            
            // Send discovery announcements
            _ = Task.Run(SendPeriodicAnnouncements);

            Console.WriteLine("🚀 C# peer started! Press any key to exit...");
            Console.ReadKey();
        }

        static async Task StartDiscoveryListener()
        {
            using var udpClient = new UdpClient(DISCOVERY_PORT);
            udpClient.EnableBroadcast = true;
            
            Console.WriteLine($"📡 UDP Discovery listener started on port {DISCOVERY_PORT}");

            while (true)
            {
                try
                {
                    var result = await udpClient.ReceiveAsync();
                    var message = DiscoveryMessage.Parser.ParseFrom(result.Buffer);
                    
                    if (message.MessageCase == DiscoveryMessage.MessageOneofCase.Announce)
                    {
                        var announce = message.Announce;
                        Console.WriteLine($"🔍 Discovered peer: {announce.PeerName} ({announce.PeerId}) at {result.RemoteEndPoint.Address}:{announce.TcpPort}");
                    }
                    else if (message.MessageCase == DiscoveryMessage.MessageOneofCase.Request)
                    {
                        Console.WriteLine($"📢 Peer discovery request from {result.RemoteEndPoint}");
                        await SendAnnouncement(udpClient);
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"❌ Discovery error: {ex.Message}");
                }
            }
        }

        static async Task SendPeriodicAnnouncements()
        {
            using var udpClient = new UdpClient();
            udpClient.EnableBroadcast = true;

            while (true)
            {
                await SendAnnouncement(udpClient);
                await Task.Delay(5000); // Every 5 seconds
            }
        }

        static async Task SendAnnouncement(UdpClient udpClient)
        {
            var announce = new DiscoveryMessage
            {
                Announce = new PeerAnnouncement
                {
                    PeerName = peerName,
                    PeerId = peerId,
                    TcpPort = (uint)TCP_PORT
                }
            };

            var data = announce.ToByteArray();
            await udpClient.SendAsync(data, data.Length, new IPEndPoint(IPAddress.Broadcast, DISCOVERY_PORT));
        }

        static async Task StartTcpServer()
        {
            var listener = new TcpListener(IPAddress.Any, TCP_PORT);
            listener.Start();
            
            Console.WriteLine($"🔗 TCP server started on port {TCP_PORT}");

            while (true)
            {
                try
                {
                    var tcpClient = await listener.AcceptTcpClientAsync();
                    Console.WriteLine($"📞 New connection from {tcpClient.Client.RemoteEndPoint}");
                    
                    _ = Task.Run(() => HandleTcpClient(tcpClient));
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"❌ TCP error: {ex.Message}");
                }
            }
        }

        static async Task HandleTcpClient(TcpClient client)
        {
            try
            {
                var stream = client.GetStream();
                var buffer = new byte[8192];

                while (client.Connected)
                {
                    // Read message size (8 bytes)
                    var sizeBytes = new byte[8];
                    var bytesRead = await stream.ReadAsync(sizeBytes, 0, 8);
                    if (bytesRead != 8) break;

                    var messageSize = BitConverter.ToUInt64(sizeBytes, 0);
                    if (BitConverter.IsLittleEndian)
                        messageSize = ReverseBytes(messageSize);

                    // Read message data
                    var messageBytes = new byte[messageSize];
                    var totalRead = 0;
                    while (totalRead < messageSize)
                    {
                        var read = await stream.ReadAsync(messageBytes, totalRead, (int)(messageSize - totalRead));
                        if (read == 0) break;
                        totalRead += read;
                    }

                    // Parse protobuf message
                    var p2pMessage = P2pMessage.Parser.ParseFrom(messageBytes);
                    HandleP2PMessage(p2pMessage);
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"❌ Client handler error: {ex.Message}");
            }
            finally
            {
                client?.Close();
            }
        }

        static void HandleP2PMessage(P2pMessage message)
        {
            var timestamp = DateTimeOffset.FromUnixTimeSeconds((long)message.Timestamp).ToString("HH:mm:ss");
            
            switch (message.Content.ContentCase)
            {
                case MessageContent.ContentOneofCase.Text:
                    Console.WriteLine($"💬 [{timestamp}] {message.SenderName}: {message.Content.Text.Text}");
                    break;
                    
                case MessageContent.ContentOneofCase.File:
                    var file = message.Content.File;
                    var sizeKb = file.Data.Length / 1024;
                    Console.WriteLine($"📁 [{timestamp}] File from {message.SenderName}: {file.Filename} ({sizeKb} KB)");
                    
                    // Save file
                    var saveDir = "received";
                    Directory.CreateDirectory(saveDir);
                    var filePath = Path.Combine(saveDir, file.Filename);
                    await File.WriteAllBytesAsync(filePath, file.Data.ToByteArray());
                    Console.WriteLine($"   Saved to: {filePath}");
                    break;
                    
                default:
                    Console.WriteLine($"📨 [{timestamp}] Unknown message type from {message.SenderName}");
                    break;
            }
        }

        static ulong ReverseBytes(ulong value)
        {
            return ((value & 0x00000000000000FF) << 56) |
                   ((value & 0x000000000000FF00) << 40) |
                   ((value & 0x0000000000FF0000) << 24) |
                   ((value & 0x00000000FF000000) << 8) |
                   ((value & 0x000000FF00000000) >> 8) |
                   ((value & 0x0000FF0000000000) >> 24) |
                   ((value & 0x00FF000000000000) >> 40) |
                   ((value & 0xFF00000000000000) >> 56);
        }
    }
}