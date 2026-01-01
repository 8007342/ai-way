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
use crate::theme::YOLLAYAH_MAGENTA;

/// Input box height (lines) for text wrapping
const INPUT_HEIGHT: u16 = 5;

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
    /// Scroll offset (lines from bottom, 0 = latest)
    scroll_offset: usize,
    /// Total rendered lines (for scroll bounds)
    total_lines: usize,

    // === Avatar Rendering State ===
    /// Avatar position (x, y)
    avatar_pos: (u16, u16),
    /// Avatar target position for smooth movement
    avatar_target: (u16, u16),
    /// Time until next wander
    wander_timer: Duration,

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

        Ok(Self {
            running: true,
            goodbye_message: None,
            conductor,
            display: DisplayState::new(),
            compositor,
            avatar,
            layers,
            input_buffer: String::new(),
            scroll_offset: 0,
            total_lines: 0,
            avatar_pos: (avatar_x, avatar_y),
            avatar_target: (avatar_x, avatar_y),
            wander_timer: Duration::from_secs(5),
            last_frame: Instant::now(),
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

                // Frame tick - do work and render
                _ = tokio::time::sleep(Duration::from_millis(16)) => {
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

            // Poll conductor for streaming tokens
            self.conductor.poll_streaming().await;

            // Receive and process messages from Conductor
            self.process_conductor_messages();

            // Update animations and display state
            self.update();

            // Render
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
        for msg in self.conductor.recv_all() {
            // Check for quit message before applying
            if let ConductorMessage::Quit { message } = &msg {
                self.goodbye_message = message.clone();
            }

            // Apply message to display state
            self.display.apply_message(msg);
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
                    let _ = self.conductor.send_message(message).await;
                    self.scroll_offset = 0;
                }
            }

            // Typing
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                let _ = self.conductor.user_typing(true).await;
            }

            KeyCode::Backspace => {
                self.input_buffer.pop();
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

        // Update avatar animation
        self.avatar.update(delta);

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

    /// Render conversation layer
    fn render_conversation(&mut self) {
        let width = self.size.0.saturating_sub(2) as usize;
        let input_and_status_height = (INPUT_HEIGHT + 1) as usize;
        let height = self.size.1.saturating_sub(input_and_status_height as u16) as usize;

        if width < 10 || height < 3 {
            return;
        }

        // Build wrapped lines from display messages
        let mut all_lines: Vec<(String, Style)> = Vec::new();

        for msg in &self.display.messages {
            let (prefix, base_style) = match msg.role {
                DisplayRole::User => ("You: ", Style::default().fg(Color::Green)),
                DisplayRole::Assistant => ("Yollayah: ", Style::default().fg(YOLLAYAH_MAGENTA)),
                DisplayRole::System => ("", Style::default().fg(Color::DarkGray)),
            };

            let content = if msg.streaming {
                format!("{}{}_", prefix, msg.content)
            } else {
                format!("{}{}", prefix, msg.content)
            };

            let wrapped = textwrap::wrap(&content, width);
            for line in wrapped {
                all_lines.push((line.to_string(), base_style));
            }
            all_lines.push((String::new(), Style::default()));
        }

        self.total_lines = all_lines.len();

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

            let visible_lines: Vec<_> = all_lines.iter().skip(visible_start).take(height).collect();

            for (i, (line, style)) in visible_lines.iter().enumerate() {
                let y = i as u16;
                if y >= area.height {
                    break;
                }

                let final_style = if has_content_above && i < 2 {
                    let shade = if i == 0 {
                        Color::Rgb(80, 80, 80)
                    } else {
                        Color::Rgb(120, 120, 120)
                    };
                    Style::default().fg(shade)
                } else if has_content_below && i >= height.saturating_sub(2) {
                    let dist_from_bottom = height.saturating_sub(1).saturating_sub(i);
                    let shade = if dist_from_bottom == 0 {
                        Color::Rgb(80, 80, 80)
                    } else {
                        Color::Rgb(120, 120, 120)
                    };
                    Style::default().fg(shade)
                } else {
                    *style
                };

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

            let full_input = format!("You: {}_", self.input_buffer);
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

            for (i, line) in visible_lines.iter().enumerate() {
                let y = area.y + 1 + i as u16;
                if y < area.y + area.height {
                    buf.set_string(area.x, y, line, Style::default().fg(Color::Green));
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
    }

    /// Render status bar
    fn render_status(&mut self) {
        if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.status) {
            buf.reset();
            let area = buf.area;

            let state_str = self.display.conductor_state.description();

            let status_style = match self.display.conductor_state {
                ConductorState::WarmingUp | ConductorState::Initializing => {
                    Style::default().fg(YOLLAYAH_MAGENTA)
                }
                _ => Style::default().fg(Color::DarkGray),
            };

            let scroll_info = if self.scroll_offset > 0 {
                format!(" [^{} lines - PgDn to scroll]", self.scroll_offset)
            } else {
                String::new()
            };

            let status = format!(
                " {} | Esc to quit | PgUp/PgDn scroll{}{}",
                state_str,
                scroll_info,
                if self.dev_mode { " [DEV]" } else { "" }
            );

            buf.set_string(area.x, area.y, &status, status_style);
        }
    }

    /// Render task panel layer
    fn render_tasks(&mut self) {
        let has_tasks = self.display.has_active_tasks();
        let tasks: Vec<_> = self
            .display
            .tasks
            .iter()
            .filter(|t| t.status.is_active())
            .cloned()
            .collect();

        if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.tasks) {
            buf.reset();
            if has_tasks {
                // Render tasks from display state
                Self::render_display_tasks_to_buffer(buf, &tasks);
            }
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
        if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.avatar) {
            buf.reset();
            self.avatar.render(buf);
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
