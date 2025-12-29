//! Main Application
//!
//! The App struct manages the entire TUI lifecycle:
//! - Event loop (keyboard, mouse, resize)
//! - Compositor orchestration
//! - Avatar state machine
//! - Backend communication

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::avatar::{Avatar, AvatarState, AvatarStateMachine, AvatarTrigger};
use crate::avatar::commands::{AvatarCommand, CommandParser, Position, Mood, Size as AvatarSize, Gesture, Reaction};
use crate::backend::{BackendClient, BackendConnection, StreamingToken};
use crate::compositor::{Compositor, LayerId};
use crate::theme::YOLLAYAH_MAGENTA;

/// Main application state
pub struct App {
    /// Is the app still running?
    running: bool,
    /// The layered compositor
    compositor: Compositor,
    /// The avatar (heart of the UX)
    avatar: Avatar,
    /// Avatar state machine
    state_machine: AvatarStateMachine,
    /// Layer assignments
    layers: AppLayers,
    /// User input buffer
    input_buffer: String,
    /// Conversation messages
    messages: Vec<Message>,
    /// Currently streaming response
    streaming: Option<String>,
    /// Stream receiver for tokens
    stream_rx: Option<mpsc::Receiver<StreamingToken>>,
    /// Last frame time (for animations)
    last_frame: Instant,
    /// Developer mode
    dev_mode: bool,
    /// Terminal size
    size: (u16, u16),
    /// Backend client
    backend: BackendClient,
    /// Model name
    model: String,
    /// Scroll offset (lines from bottom, 0 = latest)
    scroll_offset: usize,
    /// Total rendered lines (for scroll bounds)
    total_lines: usize,
    /// Avatar position (x, y)
    avatar_pos: (u16, u16),
    /// Avatar target position for smooth movement
    avatar_target: (u16, u16),
    /// Time until next wander (fallback when agent doesn't control)
    wander_timer: Duration,
    /// Command parser for agent control
    command_parser: CommandParser,
    /// Whether avatar is hidden
    avatar_hidden: bool,
    /// Current mood set by agent
    agent_mood: Option<Mood>,
    /// Whether agent enabled free wandering
    wander_enabled: bool,
    /// Current gesture/reaction being performed
    current_gesture: Option<String>,
    /// Gesture timer
    gesture_timer: Duration,
}

/// Layer IDs for UI regions
struct AppLayers {
    conversation: LayerId,
    input: LayerId,
    status: LayerId,
    avatar: LayerId,
}

/// A conversation message
#[derive(Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    User,
    Yollayah,
    System,
}

impl App {
    /// Create a new App instance
    pub async fn new() -> anyhow::Result<Self> {
        let size = crossterm::terminal::size()?;
        let area = Rect::new(0, 0, size.0, size.1);

        let mut compositor = Compositor::new(area);

        // Create layers with z-ordering
        // Lower z = further back, higher z = in front
        let conversation = compositor.create_layer(
            Rect::new(0, 0, area.width, area.height.saturating_sub(3)),
            0, // Background
        );

        let input = compositor.create_layer(
            Rect::new(0, area.height.saturating_sub(3), area.width, 2),
            10,
        );

        let status = compositor.create_layer(
            Rect::new(0, area.height.saturating_sub(1), area.width, 1),
            10,
        );

        // Avatar layer - starts in bottom right, medium size
        let avatar_bounds = Rect::new(
            area.width.saturating_sub(26),
            area.height.saturating_sub(8),
            24,
            6,
        );
        let avatar_layer = compositor.create_layer(avatar_bounds, 50);

        let layers = AppLayers {
            conversation,
            input,
            status,
            avatar: avatar_layer,
        };

        let avatar = Avatar::new();
        let state_machine = AvatarStateMachine::new();

        // Get model and connection from environment (set by yollayah.sh)
        let model = std::env::var("YOLLAYAH_MODEL")
            .unwrap_or_else(|_| "yollayah".to_string());
        let host = std::env::var("YOLLAYAH_OLLAMA_HOST")
            .unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = std::env::var("YOLLAYAH_OLLAMA_PORT")
            .unwrap_or_else(|_| "11434".to_string())
            .parse()
            .unwrap_or(11434);

        let backend = BackendClient::new(BackendConnection::DirectOllama { host, port });

        // Initial avatar position (bottom right)
        let avatar_x = area.width.saturating_sub(26);
        let avatar_y = area.height.saturating_sub(10);

        Ok(Self {
            running: true,
            compositor,
            avatar,
            state_machine,
            layers,
            input_buffer: String::new(),
            messages: vec![Message {
                role: Role::System,
                content: "Welcome! Yollayah is ready to help.".to_string(),
            }],
            streaming: None,
            stream_rx: None,
            last_frame: Instant::now(),
            dev_mode: false,
            size: (size.0, size.1),
            backend,
            model,
            scroll_offset: 0,
            total_lines: 0,
            avatar_pos: (avatar_x, avatar_y),
            avatar_target: (avatar_x, avatar_y),
            wander_timer: Duration::from_secs(5),
            command_parser: CommandParser::new(),
            avatar_hidden: false,
            agent_mood: None,
            wander_enabled: true, // Default to wandering until agent takes control
            current_gesture: None,
            gesture_timer: Duration::ZERO,
        })
    }

    /// Main event loop
    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<()> {
        // Target ~10 FPS for terminal-style animations (100ms per frame)
        let frame_duration = Duration::from_millis(100);

        while self.running {
            let frame_start = Instant::now();

            // Poll for terminal events (non-blocking)
            if event::poll(Duration::from_millis(1))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key).await,
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    Event::Resize(w, h) => self.handle_resize(w, h),
                    _ => {}
                }
            }

            // Poll for streaming tokens (non-blocking)
            self.poll_stream();

            // Update animations
            self.update();

            // Render
            self.render(terminal)?;

            // Frame rate limiting
            let elapsed = frame_start.elapsed();
            if elapsed < frame_duration {
                tokio::time::sleep(frame_duration - elapsed).await;
            }
        }

        Ok(())
    }

    /// Poll for streaming tokens from the backend
    fn poll_stream(&mut self) {
        if let Some(ref mut rx) = self.stream_rx {
            // Try to receive tokens without blocking
            while let Ok(token) = rx.try_recv() {
                match token {
                    StreamingToken::Token(text) => {
                        // Parse for avatar commands, get cleaned text
                        let clean_text = self.command_parser.parse(&text);

                        // Append cleaned text to streaming buffer
                        if !clean_text.is_empty() {
                            if let Some(ref mut s) = self.streaming {
                                s.push_str(&clean_text);
                            } else {
                                self.streaming = Some(clean_text);
                            }
                        }
                    }
                    StreamingToken::Complete { message } => {
                        // Move completed message to history
                        self.messages.push(Message {
                            role: Role::Yollayah,
                            content: message,
                        });
                        self.streaming = None;
                        self.stream_rx = None;
                        self.scroll_offset = 0; // Jump to latest
                        self.state_machine.trigger(AvatarTrigger::ResponseCompleted { success: true });
                        break;
                    }
                    StreamingToken::Error(err) => {
                        // Show error
                        self.messages.push(Message {
                            role: Role::System,
                            content: format!("Error: {}", err),
                        });
                        self.streaming = None;
                        self.stream_rx = None;
                        self.state_machine.trigger(AvatarTrigger::ErrorOccurred { message: err });
                        break;
                    }
                }
            }
        }
    }

    /// Handle keyboard input
    async fn handle_key(&mut self, key: event::KeyEvent) {
        match key.code {
            // Quit
            KeyCode::Esc => self.running = false,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false
            }

            // Submit message
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    self.submit_message().await;
                }
            }

            // Typing
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                self.state_machine.trigger(AvatarTrigger::UserTyping);
            }

            KeyCode::Backspace => {
                self.input_buffer.pop();
            }

            // Toggle dev mode
            KeyCode::F(12) => {
                self.dev_mode = !self.dev_mode;
            }

            _ => {}
        }
    }

    /// Handle mouse input
    fn handle_mouse(&mut self, mouse: event::MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                // Scroll up (show older messages)
                if self.scroll_offset < self.total_lines.saturating_sub(1) {
                    self.scroll_offset += 3;
                }
                self.state_machine.trigger(AvatarTrigger::UserScrolled);
            }
            MouseEventKind::ScrollDown => {
                // Scroll down (show newer messages)
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
                self.state_machine.trigger(AvatarTrigger::UserScrolled);
            }
            _ => {}
        }
    }

    /// Handle terminal resize
    fn handle_resize(&mut self, width: u16, height: u16) {
        self.size = (width, height);
        // Recreate compositor with new size
        let area = Rect::new(0, 0, width, height);
        self.compositor.resize(area);
        self.state_machine.trigger(AvatarTrigger::WindowResized);
    }

    /// Submit user message
    async fn submit_message(&mut self) {
        let message = std::mem::take(&mut self.input_buffer);

        // Add to history
        self.messages.push(Message {
            role: Role::User,
            content: message.clone(),
        });

        // Trigger thinking state
        self.state_machine.trigger(AvatarTrigger::QuerySubmitted);

        // Send to backend and get streaming receiver
        match self.backend.send_message(&message, &self.model).await {
            Ok(rx) => {
                self.stream_rx = Some(rx);
                self.streaming = Some(String::new());
                self.state_machine.trigger(AvatarTrigger::ResponseStarted);
            }
            Err(e) => {
                self.messages.push(Message {
                    role: Role::System,
                    content: format!("Failed to send message: {}", e),
                });
                self.state_machine.trigger(AvatarTrigger::ErrorOccurred {
                    message: e.to_string(),
                });
            }
        }
    }

    /// Update animations and state
    fn update(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_frame;
        self.last_frame = now;

        // Process any pending avatar commands from the agent
        let mut agent_moved = false;
        while let Some(cmd) = self.command_parser.next_command() {
            agent_moved |= self.apply_avatar_command(cmd);
        }

        // Update avatar animation
        self.avatar.update(delta);

        // Handle gesture timer
        if self.gesture_timer > Duration::ZERO {
            self.gesture_timer = self.gesture_timer.saturating_sub(delta);
            if self.gesture_timer.is_zero() {
                self.current_gesture = None;
            }
        }

        // Sync avatar state: gesture > mood > state machine
        let recommended_anim = if let Some(ref gesture) = self.current_gesture {
            gesture.as_str()
        } else if let Some(mood) = &self.agent_mood {
            match mood {
                Mood::Happy | Mood::Excited => "happy",
                Mood::Thinking => "thinking",
                Mood::Playful => "swimming",
                Mood::Shy => "idle",
                Mood::Confused => "error",
                Mood::Calm => "idle",
                Mood::Curious => "waiting",
            }
        } else {
            self.state_machine.recommended_animation()
        };
        self.avatar.play(recommended_anim);

        let recommended_size = self.state_machine.recommended_size();
        self.avatar.set_size(recommended_size);

        // Update z-index based on state
        let should_foreground = self.state_machine.should_be_foreground();
        let new_z = if should_foreground { 100 } else { 50 };
        self.compositor.set_z_index(self.layers.avatar, new_z);

        // Wandering - only when enabled and idle
        if self.wander_enabled && !agent_moved {
            let is_idle = matches!(
                self.state_machine.state(),
                AvatarState::Idle | AvatarState::WaitingForInput { .. } | AvatarState::Playful { .. }
            );

            if is_idle {
                // Decrease wander timer
                self.wander_timer = self.wander_timer.saturating_sub(delta);

                if self.wander_timer.is_zero() {
                    // Pick a new random target position
                    self.pick_new_wander_target();
                    // Reset timer (random 4-10 seconds)
                    let secs = 4 + (rand::random::<u64>() % 7);
                    self.wander_timer = Duration::from_secs(secs);
                }
            }
        } else if agent_moved {
            // Agent gave movement commands, reset wander timer
            self.wander_timer = Duration::from_secs(6);
        }

        // Smoothly move towards target
        self.move_towards_target();

        // Update layer position and visibility
        self.compositor.set_visible(self.layers.avatar, !self.avatar_hidden);
        self.compositor.move_layer(self.layers.avatar, self.avatar_pos.0, self.avatar_pos.1);
    }

    /// Apply an avatar command from the agent
    fn apply_avatar_command(&mut self, cmd: AvatarCommand) -> bool {
        match cmd {
            AvatarCommand::MoveTo(pos) => {
                let (bounds_w, bounds_h) = self.avatar.bounds();
                let max_x = self.size.0.saturating_sub(bounds_w + 2);
                let max_y = self.size.1.saturating_sub(bounds_h + 4);

                let (x, y) = match pos {
                    Position::TopLeft => (2, 1),
                    Position::TopRight => (max_x, 1),
                    Position::BottomLeft => (2, max_y),
                    Position::BottomRight => (max_x, max_y),
                    Position::Center => (max_x / 2, max_y / 2),
                    Position::Follow => {
                        // Near the bottom of visible text
                        (max_x, max_y.saturating_sub(3))
                    }
                };
                self.avatar_target = (x, y);
                self.wander_enabled = false; // Agent took control
                true
            }
            AvatarCommand::MoveToPercent(x_pct, y_pct) => {
                let (bounds_w, bounds_h) = self.avatar.bounds();
                let max_x = self.size.0.saturating_sub(bounds_w + 2);
                let max_y = self.size.1.saturating_sub(bounds_h + 4);

                let x = (max_x as u32 * x_pct as u32 / 100) as u16;
                let y = (max_y as u32 * y_pct as u32 / 100) as u16;
                self.avatar_target = (x.max(2), y.max(1));
                self.wander_enabled = false;
                true
            }
            AvatarCommand::PointAt(x_pct, y_pct) => {
                // Move near the target location to "point" at it
                let (bounds_w, bounds_h) = self.avatar.bounds();
                let max_x = self.size.0.saturating_sub(bounds_w + 2);
                let max_y = self.size.1.saturating_sub(bounds_h + 4);

                let target_x = (self.size.0 as u32 * x_pct as u32 / 100) as u16;
                let target_y = (self.size.1 as u32 * y_pct as u32 / 100) as u16;

                // Position avatar near target but offset
                let x = if target_x > max_x / 2 { target_x.saturating_sub(bounds_w + 2) } else { target_x + 2 };
                let y = target_y.min(max_y);

                self.avatar_target = (x.max(2).min(max_x), y.max(1));
                self.wander_enabled = false;
                true
            }
            AvatarCommand::Wander(enabled) => {
                self.wander_enabled = enabled;
                if enabled {
                    // Start wandering immediately
                    self.wander_timer = Duration::from_millis(500);
                }
                false
            }
            AvatarCommand::Mood(mood) => {
                self.agent_mood = Some(mood);
                false
            }
            AvatarCommand::Size(size) => {
                let avatar_size = match size {
                    AvatarSize::Tiny => crate::avatar::AvatarSize::Tiny,
                    AvatarSize::Small => crate::avatar::AvatarSize::Small,
                    AvatarSize::Medium => crate::avatar::AvatarSize::Medium,
                    AvatarSize::Large => crate::avatar::AvatarSize::Large,
                };
                self.avatar.set_size(avatar_size);
                false
            }
            AvatarCommand::Gesture(gesture) => {
                // Gestures trigger specific animations with duration
                let (anim, duration) = match gesture {
                    Gesture::Wave => ("happy", 1500),
                    Gesture::Nod => ("talking", 800),
                    Gesture::Shake => ("error", 800),
                    Gesture::Bounce => ("happy", 1000),
                    Gesture::Spin => ("swimming", 1200),
                    Gesture::Dance => ("happy", 2000),
                    Gesture::Swim => ("swimming", 2000),
                    Gesture::Stretch => ("idle", 1500),
                    Gesture::Yawn => ("idle", 2000),
                    Gesture::Wiggle => ("swimming", 1000),
                    Gesture::Peek(_) => ("idle", 1000),
                };
                self.current_gesture = Some(anim.to_string());
                self.gesture_timer = Duration::from_millis(duration);
                false
            }
            AvatarCommand::React(reaction) => {
                // Reactions are like gestures but more expressive
                let (anim, duration) = match reaction {
                    Reaction::Laugh => ("happy", 2000),
                    Reaction::Gasp => ("error", 1000),
                    Reaction::Hmm => ("thinking", 2000),
                    Reaction::Tada => ("happy", 2500),
                    Reaction::Oops => ("error", 1500),
                    Reaction::Love => ("happy", 2000),
                    Reaction::Blush => ("idle", 1500),
                    Reaction::Wink => ("wink", 1000),
                    Reaction::Cry => ("error", 2000),
                    Reaction::Angry => ("error", 1500),
                    Reaction::Sleepy => ("idle", 2500),
                    Reaction::Dizzy => ("swimming", 2000),
                };
                self.current_gesture = Some(anim.to_string());
                self.gesture_timer = Duration::from_millis(duration);
                false
            }
            AvatarCommand::Hide => {
                self.avatar_hidden = true;
                false
            }
            AvatarCommand::Show => {
                self.avatar_hidden = false;
                false
            }
            AvatarCommand::CustomSprite(_data) => {
                // Future: parse and apply custom sprite data
                // For now, just acknowledge the command
                false
            }
        }
    }

    /// Pick a new random position for Yollayah to wander to
    fn pick_new_wander_target(&mut self) {
        let (bounds_w, bounds_h) = self.avatar.bounds();
        let max_x = self.size.0.saturating_sub(bounds_w + 2);
        let max_y = self.size.1.saturating_sub(bounds_h + 4); // Leave room for input

        // Pick random position, but bias towards edges/corners for personality
        let corner_bias = rand::random::<f32>();

        let (x, y) = if corner_bias < 0.3 {
            // Go to a corner
            let corner = rand::random::<u8>() % 4;
            match corner {
                0 => (2, 1),                           // Top-left
                1 => (max_x, 1),                       // Top-right
                2 => (2, max_y),                       // Bottom-left
                _ => (max_x, max_y),                   // Bottom-right
            }
        } else if corner_bias < 0.5 {
            // Center (for attention)
            (max_x / 2, max_y / 2)
        } else {
            // Random position
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

        // Move 1-2 cells per frame towards target
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
        // Render each layer
        self.render_conversation();
        self.render_input();
        self.render_status();
        self.render_avatar();

        // Composite and draw
        terminal.draw(|frame| {
            let output = self.compositor.composite();

            // Copy compositor output to frame
            let area = frame.area();
            let buf = frame.buffer_mut();

            for y in 0..area.height.min(output.area.height) {
                for x in 0..area.width.min(output.area.width) {
                    let idx = output.index_of(x, y);
                    if idx < output.content.len() {
                        buf[(x, y)] = output.content[idx].clone();
                    }
                }
            }
        })?;

        Ok(())
    }

    /// Render conversation layer with text wrapping, scrolling, and edge shading
    fn render_conversation(&mut self) {
        // First pass: build all wrapped lines
        let width = self.size.0.saturating_sub(2) as usize; // Leave margin
        let height = self.size.1.saturating_sub(4) as usize; // Leave room for input/status

        if width < 10 || height < 3 {
            return;
        }

        // Build wrapped lines with their styles
        let mut all_lines: Vec<(String, Style)> = Vec::new();

        for msg in &self.messages {
            let (prefix, base_style) = match msg.role {
                Role::User => ("You: ", Style::default().fg(Color::Green)),
                Role::Yollayah => ("Yollayah: ", Style::default().fg(YOLLAYAH_MAGENTA)),
                Role::System => ("", Style::default().fg(Color::DarkGray)),
            };

            let full_text = format!("{}{}", prefix, msg.content);
            let wrapped = textwrap::wrap(&full_text, width);

            for line in wrapped {
                all_lines.push((line.to_string(), base_style));
            }

            // Blank line between messages
            all_lines.push((String::new(), Style::default()));
        }

        // Add streaming response if active
        if let Some(ref streaming) = self.streaming {
            let full_text = format!("Yollayah: {}_", streaming);
            let wrapped = textwrap::wrap(&full_text, width);

            for line in wrapped {
                all_lines.push((line.to_string(), Style::default().fg(YOLLAYAH_MAGENTA)));
            }
        }

        // Update total lines for scroll bounds
        self.total_lines = all_lines.len();

        // Clamp scroll offset
        let max_scroll = self.total_lines.saturating_sub(height);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        // Calculate visible range (scroll from bottom)
        let visible_end = self.total_lines.saturating_sub(self.scroll_offset);
        let visible_start = visible_end.saturating_sub(height);

        // Check if there's content above/below
        let has_content_above = visible_start > 0;
        let has_content_below = self.scroll_offset > 0;

        // Now render to buffer
        if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.conversation) {
            buf.reset();
            let area = buf.area;

            let visible_lines: Vec<_> = all_lines
                .iter()
                .skip(visible_start)
                .take(height)
                .collect();

            for (i, (line, style)) in visible_lines.iter().enumerate() {
                let y = i as u16;
                if y >= area.height {
                    break;
                }

                // Apply gradient shading at edges to indicate scroll
                let final_style = if has_content_above && i < 2 {
                    // Fade top lines
                    let shade = if i == 0 { Color::Rgb(80, 80, 80) } else { Color::Rgb(120, 120, 120) };
                    Style::default().fg(shade)
                } else if has_content_below && i >= height.saturating_sub(2) {
                    // Fade bottom lines
                    let dist_from_bottom = height.saturating_sub(1).saturating_sub(i);
                    let shade = if dist_from_bottom == 0 { Color::Rgb(80, 80, 80) } else { Color::Rgb(120, 120, 120) };
                    Style::default().fg(shade)
                } else {
                    *style
                };

                // Truncate line to width (should already be wrapped, but safety)
                let display_line: String = line.chars().take(area.width as usize).collect();
                buf.set_string(area.x, y, &display_line, final_style);
            }
        }
    }

    /// Render input layer
    fn render_input(&mut self) {
        if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.input) {
            buf.reset();
            let area = buf.area;

            // Draw separator
            let separator = "─".repeat(area.width as usize);
            buf.set_string(
                area.x,
                area.y,
                &separator,
                Style::default().fg(Color::DarkGray),
            );

            // Draw input prompt
            let prompt = format!("You: {}_", self.input_buffer);
            buf.set_string(
                area.x,
                area.y + 1,
                &prompt,
                Style::default().fg(Color::Green),
            );
        }
    }

    /// Render status bar
    fn render_status(&mut self) {
        if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.status) {
            buf.reset();
            let area = buf.area;

            let state_str = match self.state_machine.state() {
                AvatarState::Idle => "Ready",
                AvatarState::Thinking => "Thinking...",
                AvatarState::Responding => "Responding...",
                AvatarState::WaitingForInput { .. } => "Listening",
                AvatarState::Celebrating { .. } => "Celebrating!",
                AvatarState::Playful { .. } => "Playful",
                AvatarState::Error { .. } => "Error",
                _ => "...",
            };

            let status = format!(
                " {} | Esc to quit | F12 for dev mode{}",
                state_str,
                if self.dev_mode { " [DEV]" } else { "" }
            );

            buf.set_string(
                area.x,
                area.y,
                &status,
                Style::default().fg(Color::DarkGray),
            );
        }
    }

    /// Render avatar layer
    fn render_avatar(&mut self) {
        if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.avatar) {
            buf.reset();
            self.avatar.render(buf);
        }
    }
}
