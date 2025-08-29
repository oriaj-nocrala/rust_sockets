using System;
using System.Runtime.InteropServices;

namespace ArchSockRust.Interop;

/// <summary>
/// P/Invoke declarations for ArchSockRust native library
/// </summary>
internal static class NativeMethods
{
    private const string LibraryName = "archsockrust";

    // Error codes
    public const int FFI_SUCCESS = 0;
    public const int FFI_ERROR_INVALID_HANDLE = -1;
    public const int FFI_ERROR_INVALID_PARAMETER = -2;
    public const int FFI_ERROR_NETWORK = -3;
    public const int FFI_ERROR_RUNTIME = -4;

    // Event types
    public const int EVENT_PEER_DISCOVERED = 1;
    public const int EVENT_PEER_CONNECTED = 2;
    public const int EVENT_PEER_DISCONNECTED = 3;
    public const int EVENT_MESSAGE_RECEIVED = 4;
    public const int EVENT_FILE_RECEIVED = 5;
    public const int EVENT_ERROR = 6;

    // Event callback delegate
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate void EventCallback(int eventType, IntPtr peerId, IntPtr peerName, IntPtr message);

    // Core functions
    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    public static extern IntPtr p2p_create_messenger([MarshalAs(UnmanagedType.LPStr)] string name);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    public static extern IntPtr p2p_create_messenger_with_ports(
        [MarshalAs(UnmanagedType.LPStr)] string name, 
        ushort tcpPort, 
        ushort discoveryPort);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int p2p_start(IntPtr handle);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int p2p_stop(IntPtr handle);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int p2p_destroy(IntPtr handle);

    // Peer information
    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr p2p_get_peer_name(IntPtr handle);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr p2p_get_peer_id(IntPtr handle);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr p2p_get_local_ip(IntPtr handle);

    // Discovery and connection
    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int p2p_discover_peers(IntPtr handle);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int p2p_get_discovered_peers_count(IntPtr handle);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int p2p_get_connected_peers_count(IntPtr handle);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    public static extern int p2p_connect_to_peer(
        IntPtr handle, 
        [MarshalAs(UnmanagedType.LPStr)] string peerId);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    public static extern int p2p_disconnect_peer(
        IntPtr handle, 
        [MarshalAs(UnmanagedType.LPStr)] string peerId);

    // Messaging
    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    public static extern int p2p_send_text_message(
        IntPtr handle, 
        [MarshalAs(UnmanagedType.LPStr)] string peerId, 
        [MarshalAs(UnmanagedType.LPStr)] string message);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    public static extern int p2p_send_file(
        IntPtr handle, 
        [MarshalAs(UnmanagedType.LPStr)] string peerId, 
        [MarshalAs(UnmanagedType.LPStr)] string filePath);

    // Event handling
    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int p2p_set_event_callback(EventCallback callback);

    // Memory management
    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void p2p_free_string(IntPtr strPtr);

    // Helper method to convert native string to C# string
    public static string? PtrToString(IntPtr ptr)
    {
        if (ptr == IntPtr.Zero)
            return null;
        
        var result = Marshal.PtrToStringAnsi(ptr);
        p2p_free_string(ptr);
        return result;
    }
}