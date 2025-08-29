using System;

namespace ArchSockRust.Interop;

/// <summary>
/// Event types for P2P messenger events
/// </summary>
public enum P2PEventType
{
    PeerDiscovered = 1,
    PeerConnected = 2,
    PeerDisconnected = 3,
    MessageReceived = 4,
    FileReceived = 5,
    Error = 6
}

/// <summary>
/// Base class for P2P event arguments
/// </summary>
public abstract class P2PEventArgs : EventArgs
{
    public P2PEventType EventType { get; }
    public DateTime Timestamp { get; }

    protected P2PEventArgs(P2PEventType eventType)
    {
        EventType = eventType;
        Timestamp = DateTime.Now;
    }
}

/// <summary>
/// Event args for peer discovery events
/// </summary>
public class PeerEventArgs : P2PEventArgs
{
    public string PeerId { get; }
    public string PeerName { get; }

    public PeerEventArgs(P2PEventType eventType, string peerId, string peerName) 
        : base(eventType)
    {
        PeerId = peerId ?? throw new ArgumentNullException(nameof(peerId));
        PeerName = peerName ?? throw new ArgumentNullException(nameof(peerName));
    }
}

/// <summary>
/// Event args for message received events
/// </summary>
public class MessageReceivedEventArgs : PeerEventArgs
{
    public string Message { get; }

    public MessageReceivedEventArgs(string peerId, string peerName, string message) 
        : base(P2PEventType.MessageReceived, peerId, peerName)
    {
        Message = message ?? throw new ArgumentNullException(nameof(message));
    }
}

/// <summary>
/// Event args for error events
/// </summary>
public class ErrorEventArgs : P2PEventArgs
{
    public string ErrorMessage { get; }

    public ErrorEventArgs(string errorMessage) : base(P2PEventType.Error)
    {
        ErrorMessage = errorMessage ?? throw new ArgumentNullException(nameof(errorMessage));
    }
}

/// <summary>
/// Exception thrown by P2P operations
/// </summary>
public class P2PException : Exception
{
    public int ErrorCode { get; }

    public P2PException(int errorCode, string message) : base(message)
    {
        ErrorCode = errorCode;
    }

    public P2PException(int errorCode, string message, Exception innerException) : base(message, innerException)
    {
        ErrorCode = errorCode;
    }

    public static P2PException FromErrorCode(int errorCode)
    {
        return errorCode switch
        {
            NativeMethods.FFI_ERROR_INVALID_HANDLE => new P2PException(errorCode, "Invalid handle"),
            NativeMethods.FFI_ERROR_INVALID_PARAMETER => new P2PException(errorCode, "Invalid parameter"),
            NativeMethods.FFI_ERROR_NETWORK => new P2PException(errorCode, "Network error"),
            NativeMethods.FFI_ERROR_RUNTIME => new P2PException(errorCode, "Runtime error"),
            _ => new P2PException(errorCode, $"Unknown error: {errorCode}")
        };
    }
}