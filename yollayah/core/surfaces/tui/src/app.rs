//! Main Application
//!
//! The App struct manages the TUI lifecycle as a thin display client:
//! - Event loop (keyboard, mouse, resize)
//! - ConductorClient for orchestration
//! - DisplayState for rendering
//!
//! # Phase 2 Architecture
//!
//! The App is now a thin client that:
//! 1. Converts terminal events to SurfaceEvents
//! 2. Sends events to the embedded Conductor via ConductorClient
//! 3. Receives ConductorMessages and updates DisplayState
//! 4. Renders based on DisplayState

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, EventStream, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind,
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::Terminal;

use conductor_core::{ConductorMessage, ConductorState, ScrollDirection};

use crate::avatar::{Activity, Avatar, AvatarSize as TuiAvatarSize};
use crate::compositor::{Compositor, LayerId};
use crate::conductor_client::ConductorClient;
use crate::display::{DisplayRole, DisplayState};
use crate::theme::{
    scroll_fade_color, scroll_fade_factor, ASSISTANT_PREFIX_COLOR, INDICATOR_AGENT_ACTIVE,
    INDICATOR_AGENT_IDLE, INDICATOR_PROCESSING_ACTIVE, INPUT_TEXT_COLOR, METADATA_COLOR,
    STATUS_READY_COLOR, STATUS_THINKING_COLOR, STREAMING_CURSOR_COLOR, USER_PREFIX_COLOR,
    YOLLAYAH_MAGENTA,
};

/// Input box height (lines) for text wrapping
const INPUT_HEIGHT: u16 = 5;

/// Maximum number of input history entries to keep
const MAX_INPUT_HISTORY: usize = 50;

/// Quick goodbye messages (no LLM needed, instant)
const QUICK_GOODBYES: &[&str] = &[
    "Bye bye!",
    "Hasta luego!",
    "Cuidate!",
    "See ya!",
    "Later, gator!",
    "Nos vemos!",
    "Peace out!",
    "Take care!",
    "You got this!",
    "Believe in yourself!",
    "Go make something cool!",
    "Echale ganas!",
];

/// Main application state
pub struct App {
    // === Core State ===
    /// Is the app still running?
    running: bool,
    /// Goodbye message to show on exit
    goodbye_message: Option<String>,

    // === Conductor Integration ===
    /// Client for communicating with the embedded Conductor
    conductor: ConductorClient,
    /// Display state derived from ConductorMessages
    display: DisplayState,

    // === UI Components ===
    /// The layered compositor
    compositor: Compositor,
    /// The avatar renderer (display only)
    avatar: Avatar,
    /// Layer assignments
    layers: AppLayers,

    // === Input State ===
    /// User input buffer
    input_buffer: String,
    /// Cursor position within input buffer (char index)
    cursor_pos: usize,
    /// Input history (most recent at end)
    input_history: Vec<String>,
    /// Current position in history (None = current input, Some(i) = history[i])
    history_index: Option<usize>,
    /// Temporary storage for current input when browsing history
    history_draft: String,
    /// Scroll offset (lines from bottom, 0 = latest)
    scroll_offset: usize,
    /// Total rendered lines (for scroll bounds)
    total_lines: usize,
    /// Previous input state for dirty tracking
    prev_input_buffer: String,
    prev_cursor_pos: usize,
    /// Previous status bar state for dirty tracking
    prev_conductor_state: ConductorState,
    prev_task_count: usize,
    prev_scroll_offset: usize,
    /// Previous tasks state for dirty tracking (simple hash)
    prev_tasks_hash: u64,
    /// Conversation dirty tracking (Sprint 2 optimization)
    conversation_dirty: bool,
    last_render_width: usize,
    cached_conversation_lines: Vec<LineMeta>,

    // === Avatar Rendering State ===
    /// Avatar position (x, y)
    avatar_pos: (u16, u16),
    /// Avatar target position for smooth movement
    avatar_target: (u16, u16),
    /// Time until next wander
    wander_timer: Duration,
    /// Whether avatar animation changed this frame (for dirty tracking)
    avatar_changed: bool,

    // === Misc State ===
    /// Last frame time (for animations)
    last_frame: Instant,
    /// Developer mode
    dev_mode: bool,
    /// Terminal size
    size: (u16, u16),
}

/// Layer IDs for UI regions
struct AppLayers {
    conversation: LayerId,
    tasks: LayerId,
    input: LayerId,
    status: LayerId,
    avatar: LayerId,
}

/// Line metadata for conversation rendering
#[derive(Clone)]
struct LineMeta {
    text: String, // Keep as String for now (textwrap returns owned Cow anyway)
    base_style: Style,
    prefix_len: usize,         // Length of role prefix
    role: Option<DisplayRole>, // Role for prefix coloring
    is_streaming: bool,        // Streaming message indicator
}

impl App {
    /// Create a new App instance
    pub async fn new() -> anyhow::Result<Self> {
        let size = crossterm::terminal::size()?;
        let area = Rect::new(0, 0, size.0, size.1);

        let mut compositor = Compositor::new(area);

        // Create layers with z-ordering
        let input_and_status_height = INPUT_HEIGHT + 1;
        let conversation = compositor.create_layer(
            Rect::new(
                0,
                0,
                area.width,
                area.height.saturating_sub(input_and_status_height),
            ),
            0,
        );

        let input = compositor.create_layer(
            Rect::new(
                0,
                area.height.saturating_sub(input_and_status_height),
                area.width,
                INPUT_HEIGHT,
            ),
            10,
        );

        let status = compositor.create_layer(
            Rect::new(0, area.height.saturating_sub(1), area.width, 1),
            10,
        );

        // Task panel layer - right side
        let task_panel_width = 32u16;
        let tasks_layer = compositor.create_layer(
            Rect::new(
                area.width.saturating_sub(task_panel_width),
                0,
                task_panel_width,
                area.height.saturating_sub(input_and_status_height),
            ),
            25,
        );

        // Avatar layer
        let avatar_bounds = Rect::new(
            area.width.saturating_sub(26),
            area.height.saturating_sub(input_and_status_height + 6),
            24,
            6,
        );
        let avatar_layer = compositor.create_layer(avatar_bounds, 50);

        let layers = AppLayers {
            conversation,
            tasks: tasks_layer,
            input,
            status,
            avatar: avatar_layer,
        };

        let avatar = Avatar::new();

        // Initial avatar position
        let avatar_x = area.width.saturating_sub(26);
        let avatar_y = area.height.saturating_sub(input_and_status_height + 8);

        // Create conductor client
        let conductor = ConductorClient::new();

        let now = Instant::now();
        Ok(Self {
            running: true,
            goodbye_message: None,
            conductor,
            display: DisplayState::new(),
            compositor,
            avatar,
            layers,
            input_buffer: String::new(),
            cursor_pos: 0,
            input_history: Vec::new(),
            history_index: None,
            history_draft: String::new(),
            scroll_offset: 0,
            total_lines: 0,
            prev_input_buffer: String::new(),
            prev_cursor_pos: 0,
            prev_conductor_state: ConductorState::Initializing,
            prev_task_count: 0,
            prev_scroll_offset: 0,
            prev_tasks_hash: 0,
            conversation_dirty: true, // Start dirty to render on first frame
            last_render_width: 0,
            cached_conversation_lines: Vec::new(),
            avatar_pos: (avatar_x, avatar_y),
            avatar_target: (avatar_x, avatar_y),
            wander_timer: Duration::from_secs(5),
            avatar_changed: true, // Start dirty to render on first frame
            last_frame: now,
            dev_mode: false,
            size: (size.0, size.1),
        })
    }

    /// Main event loop
    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<()> {
        // Target ~10 FPS for terminal-style animations
        let frame_duration = Duration::from_millis(100);

        // Create async event stream for non-blocking terminal events
        let mut event_stream = EventStream::new();

        // Track startup phases
        enum StartupPhase {
            NeedStart,
            NeedConnect,
            Done,
        }
        let mut startup_phase = StartupPhase::NeedStart;

        // Render initial frame immediately so user sees UI
        self.render(terminal)?;

        while self.running {
            let frame_start = Instant::now();

            // Use select to handle events WHILE doing startup
            // This ensures we remain responsive even during slow startup
            tokio::select! {
                biased;

                // Check for terminal events - highest priority
                maybe_event = event_stream.next() => {
                    if let Some(Ok(event)) = maybe_event {
                        match event {
                            // Only handle Press events (not Release or Repeat)
                            Event::Key(key) if key.kind == KeyEventKind::Press => {
                                self.handle_key(key).await
                            }
                            Event::Mouse(mouse) => self.handle_mouse(mouse).await,
                            Event::Resize(w, h) => self.handle_resize(w, h).await,
                            _ => {}
                        }
                    }
                }

                // REACTIVE STREAMING: Process tokens as they arrive
                // This replaces the polling anti-pattern (poll_streaming())
                _ = self.conductor.process_streaming_token() => {
                    // Token was processed by conductor, which sent messages to UI
                    // Process those messages immediately
                    self.process_conductor_messages();

                    // Render immediately to display new token
                    // No need to wait for next frame tick
                    if let Err(e) = self.render(terminal) {
                        tracing::error!("Render error during streaming: {}", e);
                    }
                }

                // Frame tick - do work and render (10 FPS = 100ms)
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Handle startup phases incrementally
                    match startup_phase {
                        StartupPhase::NeedStart => {
                            // Use a short timeout so we don't block too long
                            match tokio::time::timeout(
                                Duration::from_millis(50),
                                self.conductor.start()
                            ).await {
                                Ok(Ok(())) => startup_phase = StartupPhase::NeedConnect,
                                Ok(Err(e)) => {
                                    tracing::warn!("Conductor start error: {}", e);
                                    startup_phase = StartupPhase::NeedConnect;
                                }
                                Err(_) => {
                                    // Timeout - will retry next frame
                                }
                            }
                        }
                        StartupPhase::NeedConnect => {
                            match tokio::time::timeout(
                                Duration::from_millis(50),
                                self.conductor.connect()
                            ).await {
                                Ok(Ok(())) => startup_phase = StartupPhase::Done,
                                Ok(Err(e)) => {
                                    tracing::warn!("Conductor connect error: {}", e);
                                    startup_phase = StartupPhase::Done;
                                }
                                Err(_) => {
                                    // Timeout - will retry next frame
                                }
                            }
                        }
                        StartupPhase::Done => {}
                    }
                }
            }

            // Process conductor messages (non-streaming control messages)
            // Streaming tokens are now handled reactively in tokio::select! above
            self.process_conductor_messages();

            // Update animations and display state
            self.update();

            // Render frame (streaming renders happen immediately in select!)
            self.render(terminal)?;

            // Check for quit message
            if matches!(self.display.conductor_state, ConductorState::ShuttingDown) {
                self.running = false;
            }

            // Frame rate limiting
            let elapsed = frame_start.elapsed();
            if elapsed < frame_duration {
                tokio::time::sleep(frame_duration - elapsed).await;
            }
        }

        Ok(())
    }

    /// Process all pending messages from the Conductor
    fn process_conductor_messages(&mut self) {
        let mut any_messages = false;
        for msg in self.conductor.recv_all() {
            // Check for quit message before applying
            if let ConductorMessage::Quit { message } = &msg {
                self.goodbye_message = message.clone();
            }

            // Apply message to display state
            self.display.apply_message(msg);
            any_messages = true;
        }

        // ✅ OPTIMIZATION (Sprint 2): Mark conversation dirty if any messages received
        // This includes new messages, tokens, stream end, etc.
        if any_messages {
            self.conversation_dirty = true;
        }
    }

    /// Handle keyboard input
    async fn handle_key(&mut self, key: event::KeyEvent) {
        match key.code {
            // Quit
            KeyCode::Esc => {
                self.generate_goodbye();
                let _ = self.conductor.request_quit().await;
                self.running = false;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.generate_goodbye();
                let _ = self.conductor.request_quit().await;
                self.running = false;
            }

            // Submit message
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    let message = std::mem::take(&mut self.input_buffer);
                    self.cursor_pos = 0;

                    // Save to history (avoid duplicates of the last entry)
                    if self.input_history.last() != Some(&message) {
                        self.input_history.push(message.clone());
                        // Limit history size
                        if self.input_history.len() > MAX_INPUT_HISTORY {
                            self.input_history.remove(0);
                        }
                    }
                    // Reset history navigation state
                    self.history_index = None;
                    self.history_draft.clear();

                    let _ = self.conductor.send_message(message).await;
                    self.scroll_offset = 0;
                }
            }

            // Ctrl+key shortcuts (must come before plain Char)
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Clear line
                self.input_buffer.clear();
                self.cursor_pos = 0;
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Delete word backwards
                if self.cursor_pos > 0 {
                    let chars: Vec<char> = self.input_buffer.chars().collect();
                    let mut pos = self.cursor_pos;
                    // Skip whitespace
                    while pos > 0 && chars.get(pos - 1).map_or(false, |c| c.is_whitespace()) {
                        pos -= 1;
                    }
                    // Delete word
                    while pos > 0 && chars.get(pos - 1).map_or(false, |c| !c.is_whitespace()) {
                        pos -= 1;
                    }
                    // Remove characters from pos to cursor_pos
                    let start_byte = chars[..pos].iter().collect::<String>().len();
                    let end_byte = chars[..self.cursor_pos].iter().collect::<String>().len();
                    self.input_buffer.replace_range(start_byte..end_byte, "");
                    self.cursor_pos = pos;
                }
            }

            // Typing - insert at cursor position
            KeyCode::Char(c) => {
                // Insert character at cursor position
                let byte_pos = self
                    .input_buffer
                    .char_indices()
                    .nth(self.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(self.input_buffer.len());
                self.input_buffer.insert(byte_pos, c);
                self.cursor_pos += 1;
                let _ = self.conductor.user_typing(true).await;
            }

            // Backspace - delete before cursor
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    let byte_pos = self
                        .input_buffer
                        .char_indices()
                        .nth(self.cursor_pos - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input_buffer.remove(byte_pos);
                    self.cursor_pos -= 1;
                }
            }

            // Delete - delete at cursor
            KeyCode::Delete => {
                let char_count = self.input_buffer.chars().count();
                if self.cursor_pos < char_count {
                    let byte_pos = self
                        .input_buffer
                        .char_indices()
                        .nth(self.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input_buffer.remove(byte_pos);
                }
            }

            // Word navigation with Ctrl (must come before plain arrow keys)
            KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Move to start of previous word
                let chars: Vec<char> = self.input_buffer.chars().collect();
                let mut pos = self.cursor_pos;
                // Skip whitespace
                while pos > 0 && chars.get(pos - 1).map_or(false, |c| c.is_whitespace()) {
                    pos -= 1;
                }
                // Skip word
                while pos > 0 && chars.get(pos - 1).map_or(false, |c| !c.is_whitespace()) {
                    pos -= 1;
                }
                self.cursor_pos = pos;
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Move to start of next word
                let chars: Vec<char> = self.input_buffer.chars().collect();
                let len = chars.len();
                let mut pos = self.cursor_pos;
                // Skip current word
                while pos < len && !chars[pos].is_whitespace() {
                    pos += 1;
                }
                // Skip whitespace
                while pos < len && chars[pos].is_whitespace() {
                    pos += 1;
                }
                self.cursor_pos = pos;
            }

            // Cursor movement (plain arrows)
            KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                let char_count = self.input_buffer.chars().count();
                if self.cursor_pos < char_count {
                    self.cursor_pos += 1;
                }
            }

            // Input history navigation
            KeyCode::Up => {
                if !self.input_history.is_empty() {
                    match self.history_index {
                        None => {
                            // Save current input as draft and go to most recent history
                            self.history_draft = self.input_buffer.clone();
                            let idx = self.input_history.len() - 1;
                            self.history_index = Some(idx);
                            self.input_buffer = self.input_history[idx].clone();
                            self.cursor_pos = self.input_buffer.chars().count();
                        }
                        Some(idx) if idx > 0 => {
                            // Go to older history entry
                            let new_idx = idx - 1;
                            self.history_index = Some(new_idx);
                            self.input_buffer = self.input_history[new_idx].clone();
                            self.cursor_pos = self.input_buffer.chars().count();
                        }
                        _ => {
                            // Already at oldest entry, do nothing
                        }
                    }
                }
            }
            KeyCode::Down => {
                if let Some(idx) = self.history_index {
                    if idx + 1 < self.input_history.len() {
                        // Go to newer history entry
                        let new_idx = idx + 1;
                        self.history_index = Some(new_idx);
                        self.input_buffer = self.input_history[new_idx].clone();
                        self.cursor_pos = self.input_buffer.chars().count();
                    } else {
                        // Return to current draft
                        self.history_index = None;
                        self.input_buffer = std::mem::take(&mut self.history_draft);
                        self.cursor_pos = self.input_buffer.chars().count();
                    }
                }
            }

            KeyCode::Home if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor_pos = 0;
            }
            KeyCode::End if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor_pos = self.input_buffer.chars().count();
            }

            // Conversation scrolling
            KeyCode::PageUp => {
                let page_size = self.size.1.saturating_sub(INPUT_HEIGHT + 1) / 2;
                let max_scroll = self.total_lines.saturating_sub(1);
                self.scroll_offset = (self.scroll_offset + page_size as usize).min(max_scroll);
                let _ = self
                    .conductor
                    .user_scrolled(ScrollDirection::Up, page_size as u32)
                    .await;
            }
            KeyCode::PageDown => {
                let page_size = self.size.1.saturating_sub(INPUT_HEIGHT + 1) / 2;
                self.scroll_offset = self.scroll_offset.saturating_sub(page_size as usize);
                let _ = self
                    .conductor
                    .user_scrolled(ScrollDirection::Down, page_size as u32)
                    .await;
            }
            KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.total_lines.saturating_sub(1);
                let _ = self.conductor.user_scrolled(ScrollDirection::Top, 0).await;
            }
            KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = 0;
                let _ = self
                    .conductor
                    .user_scrolled(ScrollDirection::Bottom, 0)
                    .await;
            }

            // Toggle dev mode
            KeyCode::F(12) => {
                self.dev_mode = !self.dev_mode;
            }

            _ => {}
        }
    }

    /// Handle mouse input
    async fn handle_mouse(&mut self, mouse: event::MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if self.scroll_offset < self.total_lines.saturating_sub(1) {
                    self.scroll_offset += 3;
                }
                let _ = self.conductor.user_scrolled(ScrollDirection::Up, 3).await;
            }
            MouseEventKind::ScrollDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
                let _ = self.conductor.user_scrolled(ScrollDirection::Down, 3).await;
            }
            _ => {}
        }
    }

    /// Handle terminal resize
    async fn handle_resize(&mut self, width: u16, height: u16) {
        self.size = (width, height);
        let area = Rect::new(0, 0, width, height);

        self.compositor.resize(area);

        let input_and_status_height = INPUT_HEIGHT + 1;

        // Conversation layer
        self.compositor.move_layer(self.layers.conversation, 0, 0);
        self.compositor.resize_layer(
            self.layers.conversation,
            width,
            height.saturating_sub(input_and_status_height),
        );

        // Input layer
        self.compositor.move_layer(
            self.layers.input,
            0,
            height.saturating_sub(input_and_status_height),
        );
        self.compositor
            .resize_layer(self.layers.input, width, INPUT_HEIGHT);

        // Status layer
        self.compositor
            .move_layer(self.layers.status, 0, height.saturating_sub(1));
        self.compositor.resize_layer(self.layers.status, width, 1);

        // Task panel
        let task_panel_width = 32u16;
        self.compositor
            .move_layer(self.layers.tasks, width.saturating_sub(task_panel_width), 0);
        self.compositor.resize_layer(
            self.layers.tasks,
            task_panel_width,
            height.saturating_sub(input_and_status_height),
        );

        // Avatar bounds update
        let (bounds_w, bounds_h) = self.avatar.bounds();
        let max_x = width.saturating_sub(bounds_w + 2);
        let max_y = height.saturating_sub(bounds_h + input_and_status_height + 2);
        self.avatar_target = (
            self.avatar_target.0.min(max_x),
            self.avatar_target.1.min(max_y),
        );
        self.avatar_pos = (self.avatar_pos.0.min(max_x), self.avatar_pos.1.min(max_y));

        let _ = self.conductor.resized(width as u32, height as u32).await;
    }

    /// Update animations and state
    fn update(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_frame;
        self.last_frame = now;

        // Update display state timers
        self.display.update(delta);

        // Update avatar animation (track if it changed)
        self.avatar_changed = self.avatar.update(delta);

        // Sync avatar state from display state
        self.sync_avatar_from_display();

        // Handle wandering if enabled
        if self.display.avatar.wandering {
            self.wander_timer = self.wander_timer.saturating_sub(delta);
            if self.wander_timer.is_zero() {
                self.pick_new_wander_target();
                let secs = 4 + (rand::random::<u64>() % 7);
                self.wander_timer = Duration::from_secs(secs);
            }
        }

        // Handle position updates from Conductor
        if let Some(pos) = self.display.avatar.target_position {
            self.update_avatar_target_from_position(&pos);
        }

        // Smoothly move towards target
        self.move_towards_target();

        // Update layer visibility
        self.compositor
            .set_visible(self.layers.avatar, self.display.avatar.visible);
        self.compositor
            .move_layer(self.layers.avatar, self.avatar_pos.0, self.avatar_pos.1);

        // Update task panel visibility
        let show_tasks = self.display.has_active_tasks();
        self.compositor.set_visible(self.layers.tasks, show_tasks);

        // Update z-index based on conductor state
        let should_foreground = matches!(
            self.display.conductor_state,
            ConductorState::Thinking | ConductorState::Responding
        );
        let new_z = if should_foreground { 100 } else { 50 };
        self.compositor.set_z_index(self.layers.avatar, new_z);
    }

    /// Sync TUI avatar renderer from display state
    fn sync_avatar_from_display(&mut self) {
        // Set animation based on display state
        let anim = self.display.avatar.suggested_animation();
        self.avatar.play(anim);

        // Set size
        let size = match self.display.avatar.size {
            conductor_core::AvatarSize::Tiny => TuiAvatarSize::Tiny,
            conductor_core::AvatarSize::Small => TuiAvatarSize::Small,
            conductor_core::AvatarSize::Medium => TuiAvatarSize::Medium,
            conductor_core::AvatarSize::Large => TuiAvatarSize::Large,
        };
        self.avatar.set_size(size);

        // Set activity overlay based on conductor state
        let activity = match self.display.conductor_state {
            ConductorState::Thinking | ConductorState::Responding => Activity::Thinking,
            _ => Activity::None,
        };
        self.avatar.set_activity(activity);
    }

    /// Update avatar target position from Conductor position
    fn update_avatar_target_from_position(&mut self, pos: &conductor_core::AvatarPosition) {
        let (bounds_w, bounds_h) = self.avatar.bounds();
        let input_and_status_height = INPUT_HEIGHT + 1;
        let max_x = self.size.0.saturating_sub(bounds_w + 2);
        let max_y = self
            .size
            .1
            .saturating_sub(bounds_h + input_and_status_height + 2);

        let (x, y) = match pos {
            conductor_core::AvatarPosition::TopLeft => (2, 1),
            conductor_core::AvatarPosition::TopRight => (max_x, 1),
            conductor_core::AvatarPosition::BottomLeft => (2, max_y),
            conductor_core::AvatarPosition::BottomRight => (max_x, max_y),
            conductor_core::AvatarPosition::Center => (max_x / 2, max_y / 2),
            conductor_core::AvatarPosition::Follow => (max_x, max_y.saturating_sub(3)),
            conductor_core::AvatarPosition::Percent { x: x_pct, y: y_pct } => {
                let x = (max_x as u32 * *x_pct as u32 / 100) as u16;
                let y = (max_y as u32 * *y_pct as u32 / 100) as u16;
                (x.max(2), y.max(1))
            }
        };
        self.avatar_target = (x, y);
    }

    /// Pick a new random position for Yollayah to wander to
    fn pick_new_wander_target(&mut self) {
        let (bounds_w, bounds_h) = self.avatar.bounds();
        let input_and_status_height = INPUT_HEIGHT + 1;
        let max_x = self.size.0.saturating_sub(bounds_w + 2);
        let max_y = self
            .size
            .1
            .saturating_sub(bounds_h + input_and_status_height + 2);

        let corner_bias = rand::random::<f32>();

        let (x, y) = if corner_bias < 0.3 {
            let corner = rand::random::<u8>() % 4;
            match corner {
                0 => (2, 1),
                1 => (max_x, 1),
                2 => (2, max_y),
                _ => (max_x, max_y),
            }
        } else if corner_bias < 0.5 {
            (max_x / 2, max_y / 2)
        } else {
            let x = 2 + (rand::random::<u16>() % max_x.saturating_sub(2).max(1));
            let y = 1 + (rand::random::<u16>() % max_y.saturating_sub(1).max(1));
            (x, y)
        };

        self.avatar_target = (x, y);
    }

    /// Smoothly move avatar towards target position
    fn move_towards_target(&mut self) {
        let (cx, cy) = self.avatar_pos;
        let (tx, ty) = self.avatar_target;

        let new_x = if cx < tx {
            (cx + 1).min(tx)
        } else if cx > tx {
            cx.saturating_sub(1).max(tx)
        } else {
            cx
        };

        let new_y = if cy < ty {
            (cy + 1).min(ty)
        } else if cy > ty {
            cy.saturating_sub(1).max(ty)
        } else {
            cy
        };

        self.avatar_pos = (new_x, new_y);
    }

    /// Render the UI
    fn render(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<()> {
        self.render_conversation();
        self.render_tasks();
        self.render_input();
        self.render_status();
        self.render_avatar();

        terminal.draw(|frame| {
            let output = self.compositor.composite();

            // Use Ratatui's Buffer::merge() instead of cell-by-cell cloning
            // This leverages the framework's bulk operations and avoids 10k+ Cell::clone() calls
            frame.buffer_mut().merge(output);
        })?;

        Ok(())
    }

    /// Render conversation layer
    fn render_conversation(&mut self) {
        let width = self.size.0.saturating_sub(2) as usize;
        let input_and_status_height = (INPUT_HEIGHT + 1) as usize;
        let height = self.size.1.saturating_sub(input_and_status_height as u16) as usize;

        if width < 10 || height < 3 {
            return;
        }

        // ✅ OPTIMIZATION (Sprint 2): Check if we can use cached lines
        // Only rebuild if conversation changed or width changed (terminal resize)
        let width_changed = self.last_render_width != width;
        if !self.conversation_dirty && !width_changed {
            // ✅ Cache hit! Use cached lines, skip expensive rebuild
            self.render_cached_conversation_lines(height);
            return;
        }

        // ❌ Cache miss - rebuild lines from messages
        let mut all_lines: Vec<LineMeta> = Vec::new();

        for msg in &self.display.messages {
            let (prefix, base_style) = match msg.role {
                DisplayRole::User => ("You: ", Style::default().fg(Color::Green)),
                DisplayRole::Assistant => ("Yollayah: ", Style::default().fg(YOLLAYAH_MAGENTA)),
                DisplayRole::System => ("", Style::default().fg(Color::DarkGray)),
            };

            let prefix_len = prefix.len();
            let content = if msg.streaming {
                format!("{}{}_", prefix, msg.content)
            } else {
                format!("{}{}", prefix, msg.content)
            };

            // ✅ Use cached wrapping - saves 1200+ textwrap::wrap() calls/sec
            let wrapped = msg.get_wrapped(&content, width);
            for (line_idx, line) in wrapped.iter().enumerate() {
                all_lines.push(LineMeta {
                    text: line.to_string(),
                    base_style,
                    // Only first line has the prefix
                    prefix_len: if line_idx == 0 { prefix_len } else { 0 },
                    role: if line_idx == 0 { Some(msg.role) } else { None },
                    is_streaming: msg.streaming,
                });
            }

            // Add metadata line for assistant messages (subtle, dim)
            if msg.role == DisplayRole::Assistant && !msg.streaming {
                if let Some(meta_text) = msg.format_metadata() {
                    let meta_style = Style::default().fg(METADATA_COLOR);
                    all_lines.push(LineMeta {
                        text: format!("  ⌁ {}", meta_text),
                        base_style: meta_style,
                        prefix_len: 0,
                        role: None,
                        is_streaming: false,
                    });
                }
            }

            all_lines.push(LineMeta {
                text: String::new(),
                base_style: Style::default(),
                prefix_len: 0,
                role: None,
                is_streaming: false,
            });
        }

        // ✅ OPTIMIZATION (Sprint 2): Cache the built lines for next frame
        self.cached_conversation_lines = all_lines;
        self.last_render_width = width;
        self.conversation_dirty = false;

        // Use cached lines for rendering
        self.render_cached_conversation_lines(height);
    }

    /// Render conversation using cached lines (Sprint 2 optimization)
    fn render_cached_conversation_lines(&mut self, height: usize) {
        self.total_lines = self.cached_conversation_lines.len();

        // Clamp scroll offset
        let max_scroll = self.total_lines.saturating_sub(height);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        // Calculate visible range
        let visible_end = self.total_lines.saturating_sub(self.scroll_offset);
        let visible_start = visible_end.saturating_sub(height);

        let has_content_above = visible_start > 0;
        let has_content_below = self.scroll_offset > 0;

        if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.conversation) {
            buf.reset();
            let area = buf.area;

            let visible_lines: Vec<_> = self
                .cached_conversation_lines
                .iter()
                .skip(visible_start)
                .take(height)
                .collect();

            // Number of lines to fade at edges for scroll indication
            const FADE_LINES: usize = 3;

            for (i, line_meta) in visible_lines.iter().enumerate() {
                let y = i as u16;
                if y >= area.height {
                    break;
                }

                // Calculate fade factor based on position and scroll state
                let fade =
                    scroll_fade_factor(i, height, FADE_LINES, has_content_above, has_content_below);

                // If faded, use fade color for entire line
                if fade < 1.0 {
                    let fade_color = scroll_fade_color(fade);
                    let fade_style = Style::default().fg(fade_color);
                    let display_line: String =
                        line_meta.text.chars().take(area.width as usize).collect();
                    buf.set_string(area.x, y, &display_line, fade_style);
                    continue;
                }

                // Render with breathing effects
                let display_line: String =
                    line_meta.text.chars().take(area.width as usize).collect();

                // Handle lines with role prefix (first line of each message)
                if line_meta.prefix_len > 0 && line_meta.role.is_some() {
                    let role = line_meta.role.unwrap();
                    let prefix_end = line_meta.prefix_len.min(display_line.len());

                    // Static colors for prefix (breathing removed for performance)
                    let prefix_color = match role {
                        DisplayRole::User => USER_PREFIX_COLOR,
                        DisplayRole::Assistant => {
                            if line_meta.is_streaming {
                                // Brighter for streaming messages
                                STREAMING_CURSOR_COLOR
                            } else {
                                ASSISTANT_PREFIX_COLOR
                            }
                        }
                        DisplayRole::System => Color::DarkGray,
                    };

                    // Render prefix with static color
                    let prefix_str: String = display_line.chars().take(prefix_end).collect();
                    let prefix_style = Style::default().fg(prefix_color);
                    buf.set_string(area.x, y, &prefix_str, prefix_style);

                    // Render rest of line with base style
                    if display_line.len() > prefix_end {
                        let rest: String = display_line.chars().skip(prefix_end).collect();
                        buf.set_string(area.x + prefix_end as u16, y, &rest, line_meta.base_style);
                    }
                } else {
                    // No prefix - render entire line with base style
                    buf.set_string(area.x, y, &display_line, line_meta.base_style);
                }
            }
        }

        // Mark layer as needing re-composite
        self.compositor.mark_layer_dirty(self.layers.conversation);
    }

    /// Render input layer
    fn render_input(&mut self) {
        // Check if input changed
        let input_changed = self.input_buffer != self.prev_input_buffer
            || self.cursor_pos != self.prev_cursor_pos;

        // Only render if input changed
        if input_changed {
            if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.input) {
                buf.reset();
                let area = buf.area;

                let separator = "-".repeat(area.width as usize);
                buf.set_string(
                    area.x,
                    area.y,
                    &separator,
                    Style::default().fg(Color::DarkGray),
                );

                let text_height = area.height.saturating_sub(1) as usize;
                let text_width = area.width.saturating_sub(1) as usize;

                if text_width < 5 || text_height < 1 {
                    return;
                }

                // Build input string with cursor at correct position
                let prefix = "You: ";
                let chars: Vec<char> = self.input_buffer.chars().collect();
                let cursor_pos = self.cursor_pos.min(chars.len());

                // Insert cursor indicator at position
                let (before, after) = chars.split_at(cursor_pos);
                let before_str: String = before.iter().collect();
                let after_str: String = after.iter().collect();

                // Use block cursor (▏) for insert mode feel
                let full_input = format!("{}{}▏{}", prefix, before_str, after_str);
                let wrapped_lines: Vec<String> = textwrap::wrap(&full_input, text_width)
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                let visible_lines: Vec<&String> = if wrapped_lines.len() > text_height {
                    wrapped_lines
                        .iter()
                        .skip(wrapped_lines.len() - text_height)
                        .collect()
                } else {
                    wrapped_lines.iter().collect()
                };

                // Static color for input text (breathing removed for performance)
                let input_style = Style::default().fg(INPUT_TEXT_COLOR);

                for (i, line) in visible_lines.iter().enumerate() {
                    let y = area.y + 1 + i as u16;
                    if y < area.y + area.height {
                        buf.set_string(area.x, y, line, input_style);
                    }
                }

                if wrapped_lines.len() > text_height {
                    buf.set_string(
                        area.x + area.width.saturating_sub(3),
                        area.y,
                        "^",
                        Style::default().fg(Color::Yellow),
                    );
                }
            }

            // Mark layer as needing re-composite
            self.compositor.mark_layer_dirty(self.layers.input);

            // Update previous state
            self.prev_input_buffer = self.input_buffer.clone();
            self.prev_cursor_pos = self.cursor_pos;
        }
    }

    /// Render status bar with activity indicators
    fn render_status(&mut self) {
        // Count active sub-agent tasks
        let active_task_count = self
            .display
            .tasks
            .iter()
            .filter(|t| t.status.is_active())
            .count();

        // Check if status changed
        let status_changed = self.display.conductor_state != self.prev_conductor_state
            || active_task_count != self.prev_task_count
            || self.scroll_offset != self.prev_scroll_offset;

        // Only render if status changed
        if status_changed {
            if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.status) {
                buf.reset();
                let area = buf.area;

                let state_str = self.display.conductor_state.description();

            // Determine if we're doing complex work (thinking/responding)
            let is_processing = matches!(
                self.display.conductor_state,
                ConductorState::Thinking | ConductorState::Responding
            );

            // Build activity indicators string and render with colors
            let mut x_pos = area.x;

            // Leading space
            buf.set_string(x_pos, area.y, " ", Style::default());
            x_pos += 1;

            // Processing indicator: ⚡ when thinking/responding
            if is_processing {
                buf.set_string(
                    x_pos,
                    area.y,
                    "⚡",
                    Style::default().fg(INDICATOR_PROCESSING_ACTIVE),
                );
                x_pos += 1;
            }

            // Agent work indicator: show diamonds for active tasks
            // ◆ = active agent, ◇ = idle slot (max 3 shown)
            if active_task_count > 0 || is_processing {
                if is_processing || active_task_count > 0 {
                    buf.set_string(x_pos, area.y, " ", Style::default());
                    x_pos += 1;
                }

                // Agent diamonds - static colors (breathing removed for performance)
                let agent_color = if active_task_count > 0 {
                    INDICATOR_AGENT_ACTIVE
                } else {
                    INDICATOR_AGENT_IDLE
                };

                // Show up to 3 diamonds based on task count
                let filled = active_task_count.min(3);
                let empty = 3 - filled;

                for _ in 0..filled {
                    buf.set_string(x_pos, area.y, "◆", Style::default().fg(agent_color));
                    x_pos += 1;
                }
                for _ in 0..empty {
                    buf.set_string(
                        x_pos,
                        area.y,
                        "◇",
                        Style::default().fg(INDICATOR_AGENT_IDLE),
                    );
                    x_pos += 1;
                }
            }

            // Separator if we had indicators
            if is_processing || active_task_count > 0 {
                buf.set_string(x_pos, area.y, " ", Style::default());
                x_pos += 1;
            }

            // State description with static colors (breathing removed for performance)
            let status_style = match self.display.conductor_state {
                ConductorState::Initializing => {
                    Style::default().fg(YOLLAYAH_MAGENTA)
                }
                ConductorState::Ready => Style::default().fg(STATUS_READY_COLOR),
                ConductorState::Thinking | ConductorState::Responding => {
                    Style::default().fg(STATUS_THINKING_COLOR)
                }
                _ => Style::default().fg(Color::DarkGray),
            };

            buf.set_string(x_pos, area.y, state_str, status_style);
            x_pos += state_str.len() as u16;

            // Rest of status bar
            let scroll_info = if self.scroll_offset > 0 {
                format!(" [^{} lines]", self.scroll_offset)
            } else {
                String::new()
            };

            let suffix = format!(
                " | Esc | PgUp/Dn{}{}",
                scroll_info,
                if self.dev_mode { " [DEV]" } else { "" }
            );

                buf.set_string(x_pos, area.y, &suffix, Style::default().fg(Color::DarkGray));
            }

            // Mark layer as needing re-composite
            self.compositor.mark_layer_dirty(self.layers.status);

            // Update previous state
            self.prev_conductor_state = self.display.conductor_state;
            self.prev_task_count = active_task_count;
            self.prev_scroll_offset = self.scroll_offset;
        }
    }

    /// Compute a simple hash of active tasks for dirty tracking
    fn compute_tasks_hash(tasks: &[crate::display::DisplayTask]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for task in tasks {
            task.id.0.hash(&mut hasher);
            task.progress.hash(&mut hasher);
            task.status.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Render task panel layer
    fn render_tasks(&mut self) {
        let tasks: Vec<_> = self
            .display
            .tasks
            .iter()
            .filter(|t| t.status.is_active())
            .cloned()
            .collect();

        // Compute hash of current tasks
        let tasks_hash = Self::compute_tasks_hash(&tasks);

        // Check if tasks changed
        let tasks_changed = tasks_hash != self.prev_tasks_hash;

        // Only render if tasks changed
        if tasks_changed {
            if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.tasks) {
                buf.reset();
                if !tasks.is_empty() {
                    // Render tasks from display state
                    Self::render_display_tasks_to_buffer(buf, &tasks);
                }
            }

            // Mark layer as needing re-composite
            self.compositor.mark_layer_dirty(self.layers.tasks);

            // Update previous hash
            self.prev_tasks_hash = tasks_hash;
        }
    }

    /// Render tasks from display state to a buffer
    fn render_display_tasks_to_buffer(
        buf: &mut ratatui::buffer::Buffer,
        tasks: &[crate::display::DisplayTask],
    ) {
        let area = buf.area;
        if area.width < 10 || area.height < 3 {
            return;
        }

        // Header
        buf.set_string(
            area.x + 1,
            area.y,
            "--- Tasks ---",
            Style::default().fg(Color::Yellow),
        );

        let mut y = area.y + 2;
        for task in tasks {
            if y >= area.y + area.height - 1 {
                break;
            }

            // Task name
            let name: String = task
                .display_name()
                .chars()
                .take(area.width as usize - 2)
                .collect();
            buf.set_string(area.x + 1, y, &name, Style::default().fg(YOLLAYAH_MAGENTA));
            y += 1;

            // Progress bar
            let bar_width = (area.width as usize).saturating_sub(4).min(20);
            let progress_bar = task.progress_bar(bar_width);
            let progress_str = format!("[{}] {}%", progress_bar, task.progress);
            buf.set_string(
                area.x + 2,
                y,
                &progress_str,
                Style::default().fg(Color::DarkGray),
            );
            y += 2;
        }
    }

    /// Render avatar layer
    fn render_avatar(&mut self) {
        // Only render and mark dirty if avatar actually changed
        if self.avatar_changed {
            if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.avatar) {
                buf.reset();
                self.avatar.render(buf);
            }

            // Mark layer as needing re-composite
            self.compositor.mark_layer_dirty(self.layers.avatar);
        }
    }

    /// Generate a quick goodbye message
    fn generate_goodbye(&mut self) {
        let idx = rand::random::<usize>() % QUICK_GOODBYES.len();
        self.goodbye_message = Some(QUICK_GOODBYES[idx].to_string());
    }

    /// Get the goodbye message for display after TUI closes
    pub fn goodbye(&self) -> Option<&str> {
        self.goodbye_message.as_deref()
    }
}
