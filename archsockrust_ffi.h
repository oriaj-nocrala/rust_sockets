#ifndef ARCHSOCKRUST_FFI_H
#define ARCHSOCKRUST_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle for P2P messenger
typedef struct P2PHandle P2PHandle;

// Error codes
#define FFI_SUCCESS 0
#define FFI_ERROR_INVALID_HANDLE -1
#define FFI_ERROR_INVALID_PARAMETER -2
#define FFI_ERROR_NETWORK -3
#define FFI_ERROR_RUNTIME -4

// Event types
#define EVENT_PEER_DISCOVERED 1
#define EVENT_PEER_CONNECTED 2
#define EVENT_PEER_DISCONNECTED 3
#define EVENT_MESSAGE_RECEIVED 4
#define EVENT_FILE_RECEIVED 5
#define EVENT_ERROR 6

// Event callback type
typedef void (*EventCallback)(int event_type, const char* peer_id, const char* peer_name, const char* message);

// Core functions
P2PHandle* p2p_create_messenger(const char* name);
P2PHandle* p2p_create_messenger_with_ports(const char* name, unsigned short tcp_port, unsigned short discovery_port);
int p2p_start(P2PHandle* handle);
int p2p_stop(P2PHandle* handle);
int p2p_destroy(P2PHandle* handle);

// Peer information
char* p2p_get_peer_name(P2PHandle* handle);
char* p2p_get_peer_id(P2PHandle* handle);
char* p2p_get_local_ip(P2PHandle* handle);

// Discovery and connection
int p2p_discover_peers(P2PHandle* handle);
int p2p_get_discovered_peers_count(P2PHandle* handle);
int p2p_get_connected_peers_count(P2PHandle* handle);
int p2p_connect_to_peer(P2PHandle* handle, const char* peer_id);
int p2p_disconnect_peer(P2PHandle* handle, const char* peer_id);

// Messaging
int p2p_send_text_message(P2PHandle* handle, const char* peer_id, const char* message);
int p2p_send_file(P2PHandle* handle, const char* peer_id, const char* file_path);

// Event handling
int p2p_set_event_callback(EventCallback callback);

// Memory management
void p2p_free_string(char* str_ptr);

#ifdef __cplusplus
}
#endif

#endif // ARCHSOCKRUST_FFI_H