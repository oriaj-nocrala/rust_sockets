use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{P2PMessenger, P2PEvent};

// Opaque handle for C# interop
pub struct P2PHandle {
    messenger: Arc<Mutex<P2PMessenger>>,
    runtime: tokio::runtime::Runtime,
}

// Event callback type for C#
pub type EventCallback = extern "C" fn(event_type: i32, peer_id: *const c_char, peer_name: *const c_char, message: *const c_char);

// Global event callback storage
static mut EVENT_CALLBACK: Option<EventCallback> = None;

// Error codes for C# interop
pub const FFI_SUCCESS: i32 = 0;
pub const FFI_ERROR_INVALID_HANDLE: i32 = -1;
pub const FFI_ERROR_INVALID_PARAMETER: i32 = -2;
pub const FFI_ERROR_NETWORK: i32 = -3;
pub const FFI_ERROR_RUNTIME: i32 = -4;

// Event types for C# interop
pub const EVENT_PEER_DISCOVERED: i32 = 1;
pub const EVENT_PEER_CONNECTED: i32 = 2;
pub const EVENT_PEER_DISCONNECTED: i32 = 3;
pub const EVENT_MESSAGE_RECEIVED: i32 = 4;
pub const EVENT_FILE_RECEIVED: i32 = 5;
pub const EVENT_ERROR: i32 = 6;

// Helper functions for string conversion
fn cstr_to_string(cstr: *const c_char) -> Result<String, i32> {
    if cstr.is_null() {
        return Err(FFI_ERROR_INVALID_PARAMETER);
    }
    
    unsafe {
        CStr::from_ptr(cstr)
            .to_str()
            .map(|s| s.to_string())
            .map_err(|_| FFI_ERROR_INVALID_PARAMETER)
    }
}

fn string_to_cstring(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cstring) => cstring.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

// Core FFI functions

/// Create a new P2P messenger instance
/// Returns opaque handle or null on failure
#[no_mangle]
pub extern "C" fn p2p_create_messenger(name: *const c_char) -> *mut P2PHandle {
    p2p_create_messenger_with_ports(name, 6969, 6968)
}

/// Create a new P2P messenger instance with custom ports
#[no_mangle]
pub extern "C" fn p2p_create_messenger_with_ports(
    name: *const c_char, 
    tcp_port: u16, 
    discovery_port: u16
) -> *mut P2PHandle {
    let name_str = match cstr_to_string(name) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // Create tokio runtime for async operations
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return ptr::null_mut(),
    };

    // Create messenger inside the runtime context
    let messenger = match runtime.block_on(async {
        P2PMessenger::with_ports(name_str, tcp_port, discovery_port)
    }) {
        Ok(m) => Arc::new(Mutex::new(m)),
        Err(_) => return ptr::null_mut(),
    };

    let handle = Box::new(P2PHandle {
        messenger,
        runtime,
    });

    Box::into_raw(handle)
}

/// Start the P2P messenger (begins listening and discovery)
#[no_mangle]
pub extern "C" fn p2p_start(handle: *mut P2PHandle) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    let handle = unsafe { &*handle };
    
    match handle.runtime.block_on(async {
        let mut messenger = handle.messenger.lock().await;
        
        // Setup event receiver and spawn background task
        if let Some(mut event_receiver) = messenger.get_event_receiver() {
            tokio::spawn(async move {
                while let Some(event) = event_receiver.recv().await {
                    emit_event_to_callback(&event);
                }
            });
        }
        
        messenger.start().await
    }) {
        Ok(_) => FFI_SUCCESS,
        Err(_) => FFI_ERROR_NETWORK,
    }
}

/// Stop the P2P messenger
#[no_mangle]
pub extern "C" fn p2p_stop(handle: *mut P2PHandle) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    let handle = unsafe { &*handle };
    
    handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.stop().await;
    });

    FFI_SUCCESS
}

/// Get peer name
#[no_mangle]
pub extern "C" fn p2p_get_peer_name(handle: *mut P2PHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    
    let name = handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.peer_name().to_string()
    });

    string_to_cstring(&name)
}

/// Get peer ID
#[no_mangle]
pub extern "C" fn p2p_get_peer_id(handle: *mut P2PHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    
    let id = handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.peer_id().to_string()
    });

    string_to_cstring(&id)
}

/// Get local IP address
#[no_mangle]
pub extern "C" fn p2p_get_local_ip(handle: *mut P2PHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    
    let ip = handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.get_local_ip()
    });

    string_to_cstring(&ip)
}

/// Discover peers on the network
#[no_mangle]
pub extern "C" fn p2p_discover_peers(handle: *mut P2PHandle) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    let handle = unsafe { &*handle };
    
    match handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.discover_peers()
    }) {
        Ok(_) => FFI_SUCCESS,
        Err(_) => FFI_ERROR_NETWORK,
    }
}

/// Get discovered peers count
#[no_mangle]
pub extern "C" fn p2p_get_discovered_peers_count(handle: *mut P2PHandle) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    let handle = unsafe { &*handle };
    
    let count = handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.get_discovered_peers().len()
    });

    count as i32
}

/// Get connected peers count
#[no_mangle]
pub extern "C" fn p2p_get_connected_peers_count(handle: *mut P2PHandle) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    let handle = unsafe { &*handle };
    
    let count = handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.get_connected_peers().await.len()
    });

    count as i32
}

/// Connect to a peer by ID
#[no_mangle]
pub extern "C" fn p2p_connect_to_peer(handle: *mut P2PHandle, peer_id: *const c_char) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    let peer_id_str = match cstr_to_string(peer_id) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let handle = unsafe { &*handle };
    
    // Find peer in discovered peers
    let peer_info = handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.get_discovered_peers()
            .into_iter()
            .find(|p| p.id == peer_id_str)
    });

    match peer_info {
        Some(peer) => {
            match handle.runtime.block_on(async {
                let messenger = handle.messenger.lock().await;
                messenger.connect_to_peer(&peer).await
            }) {
                Ok(_) => FFI_SUCCESS,
                Err(_) => FFI_ERROR_NETWORK,
            }
        }
        None => FFI_ERROR_INVALID_PARAMETER,
    }
}

/// Disconnect from a peer
#[no_mangle]
pub extern "C" fn p2p_disconnect_peer(handle: *mut P2PHandle, peer_id: *const c_char) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    let peer_id_str = match cstr_to_string(peer_id) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let handle = unsafe { &*handle };
    
    match handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.disconnect_peer(&peer_id_str).await
    }) {
        Ok(_) => FFI_SUCCESS,
        Err(_) => FFI_ERROR_NETWORK,
    }
}

/// Send text message to a peer
#[no_mangle]
pub extern "C" fn p2p_send_text_message(
    handle: *mut P2PHandle, 
    peer_id: *const c_char, 
    message: *const c_char
) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    let peer_id_str = match cstr_to_string(peer_id) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let message_str = match cstr_to_string(message) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let handle = unsafe { &*handle };
    
    match handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.send_text_message(&peer_id_str, message_str).await
    }) {
        Ok(_) => FFI_SUCCESS,
        Err(_) => FFI_ERROR_NETWORK,
    }
}

/// Send file to a peer
#[no_mangle]
pub extern "C" fn p2p_send_file(
    handle: *mut P2PHandle, 
    peer_id: *const c_char, 
    file_path: *const c_char
) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    let peer_id_str = match cstr_to_string(peer_id) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let file_path_str = match cstr_to_string(file_path) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let handle = unsafe { &*handle };
    
    match handle.runtime.block_on(async {
        let messenger = handle.messenger.lock().await;
        messenger.send_file(&peer_id_str, &file_path_str).await
    }) {
        Ok(_) => FFI_SUCCESS,
        Err(_) => FFI_ERROR_NETWORK,
    }
}

/// Set event callback for receiving events
#[no_mangle]
pub extern "C" fn p2p_set_event_callback(callback: EventCallback) -> i32 {
    unsafe {
        EVENT_CALLBACK = Some(callback);
    }
    FFI_SUCCESS
}

/// Free a C string returned by the library
#[no_mangle]
pub extern "C" fn p2p_free_string(str_ptr: *mut c_char) {
    if !str_ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(str_ptr);
        }
    }
}

/// Destroy P2P messenger handle
#[no_mangle]
pub extern "C" fn p2p_destroy(handle: *mut P2PHandle) -> i32 {
    if handle.is_null() {
        return FFI_ERROR_INVALID_HANDLE;
    }

    unsafe {
        let handle = Box::from_raw(handle);
        // Stop messenger before destroying
        handle.runtime.block_on(async {
            let messenger = handle.messenger.lock().await;
            messenger.stop().await;
        });
        // Runtime will be dropped automatically
    }

    FFI_SUCCESS
}

// Helper function to emit events to C# (internal use)
pub(crate) fn emit_event_to_callback(event: &P2PEvent) {
    unsafe {
        if let Some(callback) = EVENT_CALLBACK {
            match event {
                P2PEvent::PeerDiscovered(peer) => {
                    let peer_id = string_to_cstring(&peer.id);
                    let peer_name = string_to_cstring(&peer.name);
                    callback(EVENT_PEER_DISCOVERED, peer_id, peer_name, ptr::null());
                    if !peer_id.is_null() { p2p_free_string(peer_id); }
                    if !peer_name.is_null() { p2p_free_string(peer_name); }
                }
                P2PEvent::PeerConnected(peer) => {
                    let peer_id = string_to_cstring(&peer.id);
                    let peer_name = string_to_cstring(&peer.name);
                    callback(EVENT_PEER_CONNECTED, peer_id, peer_name, ptr::null());
                    if !peer_id.is_null() { p2p_free_string(peer_id); }
                    if !peer_name.is_null() { p2p_free_string(peer_name); }
                }
                P2PEvent::PeerDisconnected(peer) => {
                    let peer_id = string_to_cstring(&peer.id);
                    let peer_name = string_to_cstring(&peer.name);
                    callback(EVENT_PEER_DISCONNECTED, peer_id, peer_name, ptr::null());
                    if !peer_id.is_null() { p2p_free_string(peer_id); }
                    if !peer_name.is_null() { p2p_free_string(peer_name); }
                }
                P2PEvent::MessageReceived(message) => {
                    let peer_id = string_to_cstring(&message.sender_id);
                    let peer_name = string_to_cstring(&message.sender_name);
                    
                    if let Some(content) = &message.content {
                        if let Some(crate::message_content::Content::Text(text_msg)) = &content.content {
                            let msg_text = string_to_cstring(&text_msg.text);
                            callback(EVENT_MESSAGE_RECEIVED, peer_id, peer_name, msg_text);
                            if !msg_text.is_null() { p2p_free_string(msg_text); }
                        }
                    }
                    
                    if !peer_id.is_null() { p2p_free_string(peer_id); }
                    if !peer_name.is_null() { p2p_free_string(peer_name); }
                }
                P2PEvent::Error(error) => {
                    let error_msg = string_to_cstring(&error);
                    callback(EVENT_ERROR, ptr::null(), ptr::null(), error_msg);
                    if !error_msg.is_null() { p2p_free_string(error_msg); }
                }
                _ => {} // Other events not needed for basic C# integration
            }
        }
    }
}