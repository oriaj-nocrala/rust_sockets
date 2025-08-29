using System;
using System.Runtime.InteropServices;

namespace ArchSockRust.Interop;

/// <summary>
/// High-level C# wrapper for the ArchSockRust P2P messenger
/// </summary>
public class P2PMessenger : IDisposable
{
    private IntPtr _handle = IntPtr.Zero;
    private NativeMethods.EventCallback? _nativeCallback;
    private bool _disposed = false;

    // Events
    public event EventHandler<PeerEventArgs>? PeerDiscovered;
    public event EventHandler<PeerEventArgs>? PeerConnected;
    public event EventHandler<PeerEventArgs>? PeerDisconnected;
    public event EventHandler<MessageReceivedEventArgs>? MessageReceived;
    public event EventHandler<ErrorEventArgs>? Error;

    /// <summary>
    /// Create a new P2P messenger with default ports
    /// </summary>
    /// <param name="name">Your peer name</param>
    public P2PMessenger(string name) : this(name, 6969, 6968) { }

    /// <summary>
    /// Create a new P2P messenger with custom ports
    /// </summary>
    /// <param name="name">Your peer name</param>
    /// <param name="tcpPort">TCP port for connections</param>
    /// <param name="discoveryPort">UDP port for discovery</param>
    public P2PMessenger(string name, ushort tcpPort, ushort discoveryPort)
    {
        if (string.IsNullOrWhiteSpace(name))
            throw new ArgumentException("Name cannot be null or empty", nameof(name));

        // Create native callback
        _nativeCallback = OnNativeEvent;
        NativeMethods.p2p_set_event_callback(_nativeCallback);

        // Create messenger
        _handle = NativeMethods.p2p_create_messenger_with_ports(name, tcpPort, discoveryPort);
        if (_handle == IntPtr.Zero)
            throw new P2PException(-1, "Failed to create P2P messenger");
    }

    /// <summary>
    /// Start the messenger (begin listening and discovery)
    /// </summary>
    public void Start()
    {
        ThrowIfDisposed();
        var result = NativeMethods.p2p_start(_handle);
        ThrowIfError(result, "Failed to start messenger");
    }

    /// <summary>
    /// Stop the messenger
    /// </summary>
    public void Stop()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.p2p_stop(_handle);
        }
    }

    /// <summary>
    /// Get this peer's name
    /// </summary>
    public string? PeerName
    {
        get
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.p2p_get_peer_name(_handle);
            return NativeMethods.PtrToString(ptr);
        }
    }

    /// <summary>
    /// Get this peer's unique ID
    /// </summary>
    public string? PeerId
    {
        get
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.p2p_get_peer_id(_handle);
            return NativeMethods.PtrToString(ptr);
        }
    }

    /// <summary>
    /// Get local IP address
    /// </summary>
    public string? LocalIp
    {
        get
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.p2p_get_local_ip(_handle);
            return NativeMethods.PtrToString(ptr);
        }
    }

    /// <summary>
    /// Discover peers on the network
    /// </summary>
    public void DiscoverPeers()
    {
        ThrowIfDisposed();
        var result = NativeMethods.p2p_discover_peers(_handle);
        ThrowIfError(result, "Failed to discover peers");
    }

    /// <summary>
    /// Get count of discovered peers
    /// </summary>
    public int DiscoveredPeersCount
    {
        get
        {
            ThrowIfDisposed();
            var result = NativeMethods.p2p_get_discovered_peers_count(_handle);
            return result < 0 ? 0 : result;
        }
    }

    /// <summary>
    /// Get count of connected peers
    /// </summary>
    public int ConnectedPeersCount
    {
        get
        {
            ThrowIfDisposed();
            var result = NativeMethods.p2p_get_connected_peers_count(_handle);
            return result < 0 ? 0 : result;
        }
    }

    /// <summary>
    /// Connect to a peer by ID
    /// </summary>
    /// <param name="peerId">The peer ID to connect to</param>
    public void ConnectToPeer(string peerId)
    {
        ThrowIfDisposed();
        if (string.IsNullOrWhiteSpace(peerId))
            throw new ArgumentException("Peer ID cannot be null or empty", nameof(peerId));

        var result = NativeMethods.p2p_connect_to_peer(_handle, peerId);
        ThrowIfError(result, $"Failed to connect to peer {peerId}");
    }

    /// <summary>
    /// Disconnect from a peer
    /// </summary>
    /// <param name="peerId">The peer ID to disconnect from</param>
    public void DisconnectPeer(string peerId)
    {
        ThrowIfDisposed();
        if (string.IsNullOrWhiteSpace(peerId))
            throw new ArgumentException("Peer ID cannot be null or empty", nameof(peerId));

        var result = NativeMethods.p2p_disconnect_peer(_handle, peerId);
        ThrowIfError(result, $"Failed to disconnect from peer {peerId}");
    }

    /// <summary>
    /// Send a text message to a peer
    /// </summary>
    /// <param name="peerId">The peer ID to send to</param>
    /// <param name="message">The message text</param>
    public void SendTextMessage(string peerId, string message)
    {
        ThrowIfDisposed();
        if (string.IsNullOrWhiteSpace(peerId))
            throw new ArgumentException("Peer ID cannot be null or empty", nameof(peerId));
        if (string.IsNullOrWhiteSpace(message))
            throw new ArgumentException("Message cannot be null or empty", nameof(message));

        var result = NativeMethods.p2p_send_text_message(_handle, peerId, message);
        ThrowIfError(result, $"Failed to send message to peer {peerId}");
    }

    /// <summary>
    /// Send a file to a peer
    /// </summary>
    /// <param name="peerId">The peer ID to send to</param>
    /// <param name="filePath">Path to the file to send</param>
    public void SendFile(string peerId, string filePath)
    {
        ThrowIfDisposed();
        if (string.IsNullOrWhiteSpace(peerId))
            throw new ArgumentException("Peer ID cannot be null or empty", nameof(peerId));
        if (string.IsNullOrWhiteSpace(filePath))
            throw new ArgumentException("File path cannot be null or empty", nameof(filePath));

        var result = NativeMethods.p2p_send_file(_handle, peerId, filePath);
        ThrowIfError(result, $"Failed to send file to peer {peerId}");
    }

    // Native event callback
    private void OnNativeEvent(int eventType, IntPtr peerIdPtr, IntPtr peerNamePtr, IntPtr messagePtr)
    {
        try
        {
            var peerId = peerIdPtr != IntPtr.Zero ? Marshal.PtrToStringAnsi(peerIdPtr) : null;
            var peerName = peerNamePtr != IntPtr.Zero ? Marshal.PtrToStringAnsi(peerNamePtr) : null;
            var message = messagePtr != IntPtr.Zero ? Marshal.PtrToStringAnsi(messagePtr) : null;

            switch (eventType)
            {
                case NativeMethods.EVENT_PEER_DISCOVERED:
                    if (peerId != null && peerName != null)
                        PeerDiscovered?.Invoke(this, new PeerEventArgs(P2PEventType.PeerDiscovered, peerId, peerName));
                    break;

                case NativeMethods.EVENT_PEER_CONNECTED:
                    if (peerId != null && peerName != null)
                        PeerConnected?.Invoke(this, new PeerEventArgs(P2PEventType.PeerConnected, peerId, peerName));
                    break;

                case NativeMethods.EVENT_PEER_DISCONNECTED:
                    if (peerId != null && peerName != null)
                        PeerDisconnected?.Invoke(this, new PeerEventArgs(P2PEventType.PeerDisconnected, peerId, peerName));
                    break;

                case NativeMethods.EVENT_MESSAGE_RECEIVED:
                    if (peerId != null && peerName != null && message != null)
                        MessageReceived?.Invoke(this, new MessageReceivedEventArgs(peerId, peerName, message));
                    break;

                case NativeMethods.EVENT_ERROR:
                    if (message != null)
                        Error?.Invoke(this, new ErrorEventArgs(message));
                    break;
            }
        }
        catch (Exception ex)
        {
            // Don't let exceptions propagate to native code
            Error?.Invoke(this, new ErrorEventArgs($"Event callback error: {ex.Message}"));
        }
    }

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(P2PMessenger));
    }

    private static void ThrowIfError(int result, string message)
    {
        if (result != NativeMethods.FFI_SUCCESS)
        {
            throw P2PException.FromErrorCode(result);
        }
    }

    protected virtual void Dispose(bool disposing)
    {
        if (!_disposed)
        {
            if (_handle != IntPtr.Zero)
            {
                NativeMethods.p2p_stop(_handle);
                NativeMethods.p2p_destroy(_handle);
                _handle = IntPtr.Zero;
            }

            _nativeCallback = null;
            _disposed = true;
        }
    }

    ~P2PMessenger()
    {
        Dispose(disposing: false);
    }

    public void Dispose()
    {
        Dispose(disposing: true);
        GC.SuppressFinalize(this);
    }
}