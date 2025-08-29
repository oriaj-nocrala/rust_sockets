using System;
using System.Threading;
using System.Threading.Tasks;
using ArchSockRust.Interop;

namespace ArchSockRust.TestApp;

class Program
{
    private static P2PMessenger? _messenger;
    private static readonly CancellationTokenSource _cancellationTokenSource = new();

    static async Task Main(string[] args)
    {
        Console.WriteLine("🦀 ArchSockRust C# Test Application");
        Console.WriteLine("===================================");

        try
        {
            // Get name and ports from args or prompt
            var (name, tcpPort, discoveryPort) = GetArgsFromInput(args);
            Console.WriteLine($"Starting as: {name} (TCP: {tcpPort}, Discovery: {discoveryPort})");

            // Create messenger
            _messenger = new P2PMessenger(name, tcpPort, discoveryPort);
            
            // Subscribe to events
            SetupEventHandlers(_messenger);

            // Start messenger
            _messenger.Start();
            Console.WriteLine("✅ Messenger started!");
            
            Console.WriteLine($"📡 Local IP: {_messenger.LocalIp}");
            Console.WriteLine($"🆔 Peer ID: {_messenger.PeerId}");
            
            // Start auto-discovery task
            var discoveryTask = StartAutoDiscovery(_cancellationTokenSource.Token);
            
            // Start user input task
            var inputTask = HandleUserInput(_cancellationTokenSource.Token);
            
            // Wait for cancellation
            await Task.WhenAny(discoveryTask, inputTask);
        }
        catch (Exception ex)
        {
            Console.WriteLine($"❌ Error: {ex.Message}");
        }
        finally
        {
            _messenger?.Stop();
            _messenger?.Dispose();
            Console.WriteLine("👋 Goodbye!");
        }
    }

    private static (string name, ushort tcpPort, ushort discoveryPort) GetArgsFromInput(string[] args)
    {
        // Parse args: [name] [tcp_port] [discovery_port]
        if (args.Length >= 3)
        {
            var name = args[0];
            var tcpPort = ushort.TryParse(args[1], out var tcp) ? tcp : (ushort)6969;
            var discoveryPort = ushort.TryParse(args[2], out var disc) ? disc : (ushort)6968;
            return (name, tcpPort, discoveryPort);
        }
        else if (args.Length >= 1 && !string.IsNullOrWhiteSpace(args[0]))
        {
            return (args[0], 6969, 6968);
        }

        Console.Write("Enter your name: ");
        var input = Console.ReadLine();
        var userName = string.IsNullOrWhiteSpace(input) ? "C#User" : input;
        return (userName, 6969, 6968);
    }

    private static void SetupEventHandlers(P2PMessenger messenger)
    {
        messenger.PeerDiscovered += (sender, e) =>
        {
            Console.WriteLine($"🔍 Peer discovered: {e.PeerName} (ID: {e.PeerId[..8]}...)");
        };

        messenger.PeerConnected += (sender, e) =>
        {
            Console.WriteLine($"🔗 Peer connected: {e.PeerName} (ID: {e.PeerId[..8]}...)");
        };

        messenger.PeerDisconnected += (sender, e) =>
        {
            Console.WriteLine($"💔 Peer disconnected: {e.PeerName} (ID: {e.PeerId[..8]}...)");
        };

        messenger.MessageReceived += (sender, e) =>
        {
            Console.WriteLine($"💬 {e.PeerName}: {e.Message}");
        };

        messenger.Error += (sender, e) =>
        {
            Console.WriteLine($"❌ Error: {e.ErrorMessage}");
        };
    }

    private static async Task StartAutoDiscovery(CancellationToken cancellationToken)
    {
        Console.WriteLine("🔄 Starting auto-discovery (every 5 seconds)...");
        
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                _messenger?.DiscoverPeers();
                await Task.Delay(5000, cancellationToken);
            }
            catch (TaskCanceledException)
            {
                break;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"❌ Discovery error: {ex.Message}");
            }
        }
    }

    private static async Task HandleUserInput(CancellationToken cancellationToken)
    {
        PrintMenu();
        
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                Console.Write("\nChoose option: ");
                var input = await ReadLineAsync();
                
                if (string.IsNullOrWhiteSpace(input))
                    continue;

                switch (input.Trim().ToLower())
                {
                    case "1":
                        ShowStatus();
                        break;
                    case "2":
                        await ConnectToPeer();
                        break;
                    case "3":
                        await SendMessage();
                        break;
                    case "4":
                        await SendFile();
                        break;
                    case "5":
                        ForceDiscovery();
                        break;
                    case "h":
                    case "help":
                        PrintMenu();
                        break;
                    case "q":
                    case "quit":
                    case "0":
                        _cancellationTokenSource.Cancel();
                        return;
                    default:
                        Console.WriteLine("❌ Invalid option. Type 'h' for help.");
                        break;
                }
            }
            catch (TaskCanceledException)
            {
                break;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"❌ Input error: {ex.Message}");
            }
        }
    }

    private static void PrintMenu()
    {
        Console.WriteLine("\n📋 Menu:");
        Console.WriteLine("1. Show status");
        Console.WriteLine("2. Connect to peer");
        Console.WriteLine("3. Send message");
        Console.WriteLine("4. Send file");
        Console.WriteLine("5. Force discovery");
        Console.WriteLine("h. Help");
        Console.WriteLine("q. Quit");
    }

    private static void ShowStatus()
    {
        if (_messenger == null) return;
        
        Console.WriteLine("\n📊 Status:");
        Console.WriteLine($"• Name: {_messenger.PeerName}");
        Console.WriteLine($"• ID: {_messenger.PeerId}");
        Console.WriteLine($"• Local IP: {_messenger.LocalIp}");
        Console.WriteLine($"• Discovered peers: {_messenger.DiscoveredPeersCount}");
        Console.WriteLine($"• Connected peers: {_messenger.ConnectedPeersCount}");
    }

    private static async Task ConnectToPeer()
    {
        if (_messenger == null) return;

        if (_messenger.DiscoveredPeersCount == 0)
        {
            Console.WriteLine("❌ No peers discovered yet. Try discovery first (option 5).");
            return;
        }

        Console.Write("Enter peer ID to connect: ");
        var peerId = await ReadLineAsync();
        
        if (string.IsNullOrWhiteSpace(peerId))
        {
            Console.WriteLine("❌ Invalid peer ID.");
            return;
        }

        try
        {
            _messenger.ConnectToPeer(peerId);
            Console.WriteLine($"✅ Connecting to peer {peerId[..8]}...");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"❌ Connection failed: {ex.Message}");
        }
    }

    private static async Task SendMessage()
    {
        if (_messenger == null) return;

        if (_messenger.ConnectedPeersCount == 0)
        {
            Console.WriteLine("❌ No connected peers. Connect to a peer first.");
            return;
        }

        Console.Write("Enter peer ID: ");
        var peerId = await ReadLineAsync();
        
        if (string.IsNullOrWhiteSpace(peerId))
        {
            Console.WriteLine("❌ Invalid peer ID.");
            return;
        }

        Console.Write("Enter message: ");
        var message = await ReadLineAsync();
        
        if (string.IsNullOrWhiteSpace(message))
        {
            Console.WriteLine("❌ Message cannot be empty.");
            return;
        }

        try
        {
            _messenger.SendTextMessage(peerId, message);
            Console.WriteLine($"✅ Message sent to {peerId[..8]}...");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"❌ Send failed: {ex.Message}");
        }
    }

    private static async Task SendFile()
    {
        if (_messenger == null) return;

        if (_messenger.ConnectedPeersCount == 0)
        {
            Console.WriteLine("❌ No connected peers. Connect to a peer first.");
            return;
        }

        Console.Write("Enter peer ID: ");
        var peerId = await ReadLineAsync();
        
        if (string.IsNullOrWhiteSpace(peerId))
        {
            Console.WriteLine("❌ Invalid peer ID.");
            return;
        }

        Console.Write("Enter file path: ");
        var filePath = await ReadLineAsync();
        
        if (string.IsNullOrWhiteSpace(filePath))
        {
            Console.WriteLine("❌ File path cannot be empty.");
            return;
        }

        try
        {
            _messenger.SendFile(peerId, filePath);
            Console.WriteLine($"✅ File sent to {peerId[..8]}...");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"❌ File send failed: {ex.Message}");
        }
    }

    private static void ForceDiscovery()
    {
        if (_messenger == null) return;

        try
        {
            _messenger.DiscoverPeers();
            Console.WriteLine("🔍 Discovery broadcast sent!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"❌ Discovery failed: {ex.Message}");
        }
    }

    private static Task<string> ReadLineAsync()
    {
        return Task.Run(() => Console.ReadLine() ?? string.Empty);
    }
}