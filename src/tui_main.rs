use archsockrust::app::{AppState, AppEventHandler, ChatMessage, MessageType, PeerStatus};
use archsockrust::{P2PMessenger, format_timestamp};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame, Terminal,
};
use std::env;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ActivePanel {
    Peers,
    Messages,
    Input,
}

struct TuiState {
    app_state: Arc<Mutex<AppState>>,
    active_panel: ActivePanel,
    peer_list_state: ListState,
    input_buffer: String,
    status_message: String,
    should_quit: bool,
    show_help: bool,
}

impl TuiState {
    fn new(app_state: Arc<Mutex<AppState>>) -> Self {
        let mut peer_list_state = ListState::default();
        // Don't select anything initially - will be set properly in first update
        peer_list_state.select(None);

        Self {
            app_state,
            active_panel: ActivePanel::Peers,
            peer_list_state,
            input_buffer: String::new(),
            status_message: "Ready - Press 'h' for help".to_string(),
            should_quit: false,
            show_help: false,
        }
    }
    
    // Helper method to ensure selection is always on a valid peer
    async fn ensure_valid_selection(&mut self) {
        let app_state = self.app_state.lock().await;
        let peer_indices = get_visual_peer_indices(&app_state);
        
        if !peer_indices.is_empty() {
            let current = self.peer_list_state.selected();
            
            // If no selection or invalid selection, select first peer
            if current.is_none() || !peer_indices.contains(&current.unwrap()) {
                self.peer_list_state.select(Some(peer_indices[0]));
            }
        } else {
            // No peers available, clear selection
            self.peer_list_state.select(None);
        }
    }

    fn next_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Peers => ActivePanel::Messages,
            ActivePanel::Messages => ActivePanel::Input,
            ActivePanel::Input => ActivePanel::Peers,
        }
    }

    fn prev_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Peers => ActivePanel::Input,
            ActivePanel::Messages => ActivePanel::Peers,
            ActivePanel::Input => ActivePanel::Messages,
        }
    }

    async fn next_peer(&mut self) {
        let app_state = self.app_state.lock().await;
        let peer_indices = get_visual_peer_indices(&app_state);
        
        if !peer_indices.is_empty() {
            let current = self.peer_list_state.selected().unwrap_or(0);
            
            // Find current position in peer indices or start from first peer
            let current_pos = peer_indices.iter().position(|&i| i == current)
                .unwrap_or(peer_indices.len()); // If not found, wrap to start
            
            let next_pos = (current_pos + 1) % peer_indices.len();
            self.peer_list_state.select(Some(peer_indices[next_pos]));
        }
    }

    async fn prev_peer(&mut self) {
        let app_state = self.app_state.lock().await;
        let peer_indices = get_visual_peer_indices(&app_state);
        
        if !peer_indices.is_empty() {
            let current = self.peer_list_state.selected().unwrap_or(0);
            
            // Find current position in peer indices or start from last peer
            let current_pos = peer_indices.iter().position(|&i| i == current)
                .unwrap_or(0); // If not found, start from beginning
            
            let prev_pos = if current_pos == 0 { 
                peer_indices.len() - 1 
            } else { 
                current_pos - 1 
            };
            self.peer_list_state.select(Some(peer_indices[prev_pos]));
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI args
    let args: Vec<String> = env::args().collect();
    let (name, tcp_port, discovery_port) = if args.len() > 1 {
        let name = args[1].clone();
        let tcp_port = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(6969);
        let discovery_port = args.get(3).and_then(|p| p.parse().ok()).unwrap_or(6968);
        (name, tcp_port, discovery_port)
    } else {
        ("TUI User".to_string(), 6969, 6968)
    };

    // Create messenger
    let mut messenger = P2PMessenger::with_ports(name, tcp_port, discovery_port)?;
    messenger.start().await?;

    let mut event_receiver = messenger.get_event_receiver().unwrap();
    let app_state = Arc::new(Mutex::new(AppState::new(messenger)));
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut tui_state = TuiState::new(app_state.clone());

    // Event handler task
    let app_state_for_events = app_state.clone();
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            let mut app_state = app_state_for_events.lock().await;
            AppEventHandler::handle_p2p_event(event, &mut app_state).await;
        }
    });

    // Auto-discovery task
    let app_state_for_discovery = app_state.clone();
    tokio::spawn(async move {
        loop {
            {
                let app_state = app_state_for_discovery.lock().await;
                let _ = app_state.messenger.discover_peers();
                app_state.messenger.cleanup_stale_peers();
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

    // Auto-refresh task
    let app_state_for_refresh = app_state.clone();
    tokio::spawn(async move {
        loop {
            {
                let mut app_state = app_state_for_refresh.lock().await;
                app_state.refresh_peers().await;
            }
            sleep(Duration::from_secs(2)).await;
        }
    });

    // Main TUI loop
    let res = run_tui(&mut terminal, &mut tui_state).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Stop messenger
    {
        let app_state = app_state.lock().await;
        app_state.messenger.stop().await;
    }

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_tui<B: Backend>(
    terminal: &mut Terminal<B>,
    tui_state: &mut TuiState,
) -> io::Result<()> {
    loop {
        // Ensure selection is always on a valid peer
        tui_state.ensure_valid_selection().await;
        
        terminal.draw(|f| ui(f, tui_state))?;

        if tui_state.should_quit {
            break;
        }

        // Handle events with timeout
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key_event(key.code, tui_state).await;
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, tui_state: &TuiState) {
    let size = f.area();
    
    if tui_state.show_help {
        draw_help_popup(f, size);
        return;
    }

    // Main layout: horizontal split
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(size);

    // Left panel: peers
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(main_chunks[0]);

    // Right panel: messages + input
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3), Constraint::Length(3)].as_ref())
        .split(main_chunks[1]);

    // Draw panels
    draw_peers_panel(f, left_chunks[0], tui_state);
    draw_status_panel(f, left_chunks[1], tui_state);
    draw_messages_panel(f, right_chunks[0], tui_state);
    draw_input_panel(f, right_chunks[1], tui_state);
    draw_controls_panel(f, right_chunks[2]);
}

fn draw_peers_panel(f: &mut Frame, area: Rect, tui_state: &TuiState) {
    // This is async-safe since we're just reading the current state snapshot
    let app_state_lock = tui_state.app_state.try_lock();
    if app_state_lock.is_err() {
        return;
    }
    let app_state = app_state_lock.unwrap();

    let mut items = Vec::new();

    // Add discovered peers
    if !app_state.discovered_peers.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "🔍 Discovered Peers:",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))));

        for peer in &app_state.discovered_peers {
            let status = if peer.is_connected { " [CONNECTED]" } else { "" };
            items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(&peer.name, Style::default().fg(Color::Cyan)),
                Span::raw(format!(" ({}:{}){}", peer.ip, peer.port, status)),
            ])));
        }
    }

    // Add connected peers
    if !app_state.connected_peers.is_empty() {
        if !items.is_empty() {
            items.push(ListItem::new(Line::from(""))); // Empty line separator
        }
        items.push(ListItem::new(Line::from(Span::styled(
            "🔗 Connected Peers:",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ))));

        for peer in &app_state.connected_peers {
            items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(&peer.name, Style::default().fg(Color::Green)),
                Span::raw(format!(" ({}:{})", peer.ip, peer.port)),
            ])));
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "No peers discovered yet...",
            Style::default().fg(Color::DarkGray),
        ))));
        items.push(ListItem::new(Line::from("")));
        items.push(ListItem::new(Line::from(Span::styled(
            "Make sure other instances",
            Style::default().fg(Color::DarkGray),
        ))));
        items.push(ListItem::new(Line::from(Span::styled(
            "are running on the network",
            Style::default().fg(Color::DarkGray),
        ))));
    }

    let peers_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Peers")
                .border_style(if tui_state.active_panel == ActivePanel::Peers {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                })
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("→ ");

    f.render_stateful_widget(peers_list, area, &mut tui_state.peer_list_state.clone());
}

fn draw_status_panel(f: &mut Frame, area: Rect, tui_state: &TuiState) {
    let app_state_lock = tui_state.app_state.try_lock();
    if app_state_lock.is_err() {
        return;
    }
    let app_state = app_state_lock.unwrap();

    let status_text = format!(
        "📡 {} | ID: {:.8}... | 🔍{} 🔗{}",
        app_state.messenger.peer_name(),
        app_state.messenger.peer_id(),
        app_state.discovered_peers.len(),
        app_state.connected_peers.len()
    );

    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .wrap(Wrap { trim: true });

    f.render_widget(status, area);
}

fn draw_messages_panel(f: &mut Frame, area: Rect, tui_state: &TuiState) {
    let app_state_lock = tui_state.app_state.try_lock();
    if app_state_lock.is_err() {
        return;
    }
    let app_state = app_state_lock.unwrap();

    let messages: Vec<ListItem> = app_state
        .messages
        .iter()
        .map(|msg| {
            let timestamp = format_timestamp(msg.timestamp);
            let content = match &msg.message_type {
                archsockrust::app::MessageType::Text => {
                    format!("[{}] {}: {}", timestamp, msg.sender, msg.content)
                }
                archsockrust::app::MessageType::File { filename, size, .. } => {
                    format!("[{}] {} sent file: {} ({} bytes)", timestamp, msg.sender, filename, size)
                }
                archsockrust::app::MessageType::System => {
                    format!("[{}] System: {}", timestamp, msg.content)
                }
            };

            let style = match &msg.message_type {
                archsockrust::app::MessageType::System => Style::default().fg(Color::Yellow),
                archsockrust::app::MessageType::File { .. } => Style::default().fg(Color::Magenta),
                _ => Style::default().fg(Color::White),
            };

            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    let messages_list = List::new(messages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Messages")
                .border_style(if tui_state.active_panel == ActivePanel::Messages {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                })
        );

    f.render_widget(messages_list, area);
}

fn draw_input_panel(f: &mut Frame, area: Rect, tui_state: &TuiState) {
    let input = Paragraph::new(tui_state.input_buffer.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input (Enter to send, Tab to switch panels)")
                .border_style(if tui_state.active_panel == ActivePanel::Input {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                })
        );

    f.render_widget(input, area);
}

fn draw_controls_panel(f: &mut Frame, area: Rect) {
    let controls = Paragraph::new("c: Connect | d: Disconnect | f: Send File | h: Help | q: Quit")
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(controls, area);
}

fn draw_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(80, 80, area);

    let help_text = vec![
        Line::from(Span::styled("🦀 ArchSockRust TUI Help", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Navigation:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from("  Tab / Shift+Tab  - Switch between panels"),
        Line::from("  ↑/↓ (in peers)   - Select peer"),
        Line::from("  Enter (in input) - Send message"),
        Line::from(""),
        Line::from(Span::styled("Actions:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from("  c - Connect to selected peer"),
        Line::from("  d - Disconnect from selected peer"),
        Line::from("  f - Send file to selected peer"),
        Line::from("  F5 - Force discovery"),
        Line::from(""),
        Line::from(Span::styled("General:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from("  h - Toggle this help"),
        Line::from("  q - Quit application"),
        Line::from(""),
        Line::from(Span::styled("Features:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
        Line::from("  • Auto-discovery every 5 seconds"),
        Line::from("  • Real-time peer connections"),
        Line::from("  • Text and file messaging"),
        Line::from("  • Handshake-based peer identification"),
        Line::from(""),
        Line::from(Span::styled("Press 'h' again to close", Style::default().fg(Color::Yellow))),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, popup_area);
    f.render_widget(help_paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

async fn handle_key_event(key: KeyCode, tui_state: &mut TuiState) {
    match key {
        KeyCode::Char('q') => {
            tui_state.should_quit = true;
        }
        KeyCode::Char('h') => {
            tui_state.show_help = !tui_state.show_help;
        }
        KeyCode::Tab => {
            if !tui_state.show_help {
                tui_state.next_panel();
            }
        }
        KeyCode::BackTab => {
            if !tui_state.show_help {
                tui_state.prev_panel();
            }
        }
        _ => {
            if tui_state.show_help {
                return;
            }

            match tui_state.active_panel {
                ActivePanel::Peers => handle_peers_key(key, tui_state).await,
                ActivePanel::Messages => handle_messages_key(key, tui_state).await,
                ActivePanel::Input => handle_input_key(key, tui_state).await,
            }
        }
    }
}

async fn handle_peers_key(key: KeyCode, tui_state: &mut TuiState) {
    match key {
        KeyCode::Up => tui_state.prev_peer().await,
        KeyCode::Down => tui_state.next_peer().await,
        KeyCode::Char('c') => connect_to_selected_peer(tui_state).await,
        KeyCode::Char('d') => disconnect_selected_peer(tui_state).await,
        KeyCode::Char('f') => send_file_to_selected_peer(tui_state).await,
        KeyCode::F(5) => force_discovery(tui_state).await,
        _ => {}
    }
}

async fn handle_messages_key(_key: KeyCode, _tui_state: &mut TuiState) {
    // Messages panel is read-only for now
}

async fn handle_input_key(key: KeyCode, tui_state: &mut TuiState) {
    match key {
        KeyCode::Enter => {
            if !tui_state.input_buffer.trim().is_empty() {
                send_message(tui_state).await;
            }
        }
        KeyCode::Backspace => {
            tui_state.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            tui_state.input_buffer.push(c);
        }
        _ => {}
    }
}

async fn connect_to_selected_peer(tui_state: &mut TuiState) {
    let selected = tui_state.peer_list_state.selected();
    if let Some(visual_index) = selected {
        let mut app_state = tui_state.app_state.lock().await;
        if let Some(real_index) = map_visual_to_real_peer_index(visual_index, &app_state) {
            app_state.selected_peer = Some(real_index);
            match app_state.connect_to_selected_peer().await {
                Ok(msg) => tui_state.status_message = msg,
                Err(e) => tui_state.status_message = e,
            }
        } else {
            tui_state.status_message = "Invalid selection".to_string();
        }
    }
}

async fn disconnect_selected_peer(tui_state: &mut TuiState) {
    let selected = tui_state.peer_list_state.selected();
    if let Some(visual_index) = selected {
        let mut app_state = tui_state.app_state.lock().await;
        if let Some(real_index) = map_visual_to_real_peer_index(visual_index, &app_state) {
            app_state.selected_peer = Some(real_index);
            match app_state.disconnect_from_selected_peer().await {
                Ok(msg) => tui_state.status_message = msg,
                Err(e) => tui_state.status_message = e,
            }
        } else {
            tui_state.status_message = "Invalid selection".to_string();
        }
    }
}

async fn send_message(tui_state: &mut TuiState) {
    let message = tui_state.input_buffer.clone();
    tui_state.input_buffer.clear();

    let selected = tui_state.peer_list_state.selected();
    if let Some(visual_index) = selected {
        let mut app_state = tui_state.app_state.lock().await;
        if let Some(peer_info) = get_peer_from_visual_index(visual_index, &app_state) {
            // Send message directly using peer ID instead of relying on selected_peer index
            match app_state.messenger.send_text_message(&peer_info.id, message.clone()).await {
                Ok(()) => {
                    let chat_message = ChatMessage {
                        sender: format!("{} (You)", app_state.messenger.peer_name()),
                        content: message.clone(),
                        timestamp: archsockrust::get_current_timestamp(),
                        message_type: MessageType::Text,
                    };
                    app_state.add_message(chat_message);
                    tui_state.status_message = format!("Message sent to {}", peer_info.name);
                }
                Err(e) => tui_state.status_message = format!("Failed to send message: {}", e),
            }
        } else {
            tui_state.status_message = "Invalid selection".to_string();
        }
    } else {
        tui_state.status_message = "No peer selected".to_string();
    }
}

async fn send_file_to_selected_peer(tui_state: &mut TuiState) {
    // For now, use a hardcoded test file path
    // In a real implementation, you'd want a file picker dialog
    let file_path = "test.txt".to_string();
    
    let selected = tui_state.peer_list_state.selected();
    if let Some(visual_index) = selected {
        let mut app_state = tui_state.app_state.lock().await;
        if let Some(real_index) = map_visual_to_real_peer_index(visual_index, &app_state) {
            app_state.selected_peer = Some(real_index);
            match app_state.send_file(file_path).await {
                Ok(msg) => tui_state.status_message = msg,
                Err(e) => tui_state.status_message = e,
            }
        } else {
            tui_state.status_message = "Invalid selection".to_string();
        }
    } else {
        tui_state.status_message = "No peer selected".to_string();
    }
}

async fn force_discovery(tui_state: &mut TuiState) {
    let app_state = tui_state.app_state.lock().await;
    match app_state.force_discovery() {
        Ok(msg) => tui_state.status_message = msg,
        Err(e) => tui_state.status_message = e,
    }
}

// Maps visual list index to real peer index in the combined discovered+connected peers
fn map_visual_to_real_peer_index(visual_index: usize, app_state: &AppState) -> Option<usize> {
    let mut current_index = 0;
    let mut peer_count = 0;
    
    // Process discovered peers section
    if !app_state.discovered_peers.is_empty() {
        // Skip "🔍 Discovered Peers:" header
        if visual_index == current_index {
            return None; // Header selected, not a peer
        }
        current_index += 1;
        
        // Check if selection is in discovered peers
        for i in 0..app_state.discovered_peers.len() {
            if visual_index == current_index {
                return Some(peer_count + i); // Return index in combined list
            }
            current_index += 1;
        }
        peer_count += app_state.discovered_peers.len();
    }
    
    // Process connected peers section
    if !app_state.connected_peers.is_empty() {
        // Skip empty line separator if we had discovered peers
        if !app_state.discovered_peers.is_empty() {
            if visual_index == current_index {
                return None; // Empty line selected
            }
            current_index += 1;
        }
        
        // Skip "🔗 Connected Peers:" header
        if visual_index == current_index {
            return None; // Header selected, not a peer
        }
        current_index += 1;
        
        // Check if selection is in connected peers
        for i in 0..app_state.connected_peers.len() {
            if visual_index == current_index {
                return Some(peer_count + i); // Return index in combined list
            }
            current_index += 1;
        }
    }
    
    None // Invalid selection
}

// Returns the visual indices of only the real peers (skipping headers and separators)
fn get_visual_peer_indices(app_state: &AppState) -> Vec<usize> {
    let mut peer_indices = Vec::new();
    let mut current_index = 0;
    
    // Process discovered peers section
    if !app_state.discovered_peers.is_empty() {
        // Skip "🔍 Discovered Peers:" header
        current_index += 1;
        
        // Add discovered peer indices
        for _ in 0..app_state.discovered_peers.len() {
            peer_indices.push(current_index);
            current_index += 1;
        }
    }
    
    // Process connected peers section
    if !app_state.connected_peers.is_empty() {
        // Skip empty line separator if we had discovered peers
        if !app_state.discovered_peers.is_empty() {
            current_index += 1;
        }
        
        // Skip "🔗 Connected Peers:" header
        current_index += 1;
        
        // Add connected peer indices
        for _ in 0..app_state.connected_peers.len() {
            peer_indices.push(current_index);
            current_index += 1;
        }
    }
    
    peer_indices
}

// Returns the actual PeerInfo from visual index, or None if invalid
fn get_peer_from_visual_index(visual_index: usize, app_state: &AppState) -> Option<PeerStatus> {
    let mut current_index = 0;
    
    // Process discovered peers section
    if !app_state.discovered_peers.is_empty() {
        // Skip "🔍 Discovered Peers:" header
        current_index += 1;
        
        // Check discovered peers
        for peer in &app_state.discovered_peers {
            if visual_index == current_index {
                return Some(peer.clone());
            }
            current_index += 1;
        }
    }
    
    // Process connected peers section
    if !app_state.connected_peers.is_empty() {
        // Skip empty line separator if we had discovered peers
        if !app_state.discovered_peers.is_empty() {
            current_index += 1;
        }
        
        // Skip "🔗 Connected Peers:" header
        current_index += 1;
        
        // Check connected peers
        for peer in &app_state.connected_peers {
            if visual_index == current_index {
                return Some(peer.clone());
            }
            current_index += 1;
        }
    }
    
    None
}