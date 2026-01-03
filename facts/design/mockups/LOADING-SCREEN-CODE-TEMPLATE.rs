//! Loading Screen - Code Template for Option B (Pulsing Axolotl Head)
//!
//! This template shows the structure for implementing the recommended
//! loading screen design. Copy to: tui/src/loading.rs
//!
//! Implementation: ~200 lines
//! Time: 2 hours
//! Complexity: Medium
//!
//! Dependencies:
//! - ratatui: For terminal rendering
//! - std::time: For elapsed time tracking
//!
//! Related modules:
//! - tui/src/avatar/sprites.rs: Frame, SpriteSheet types
//! - tui/src/theme/mod.rs: Color palette and breathing_color()

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::time::Duration;

use crate::avatar::sprites::{build_frame, ColoredCell, Frame as AvatarFrame};
use crate::theme::*;

/// Loading screen with pulsing Yollayah avatar
///
/// Displays a cute 6x2 axolotl sprite that "breathes" (gills pulse with color)
/// while showing status messages. Used during TUI startup while connecting
/// to Conductor.
pub struct LoadingScreen {
    /// Total elapsed time since screen creation
    elapsed: Duration,

    /// Current loading phase (affects status message)
    phase: LoadingPhase,

    /// Animation frame index (0-2, cycles every 333ms)
    /// Frame 0: dim gills, open eyes (oo)
    /// Frame 1: bright gills, happy eyes (^^)
    /// Frame 2: curious eyes (..)
    current_frame: usize,

    /// Status message index (cycles every 2 seconds)
    /// "Just starting..." → "Warming up..." → "Almost there!"
    message_index: usize,

    /// Cached animation frames (pre-built on creation)
    frames: [AvatarFrame; 3],
}

/// Current loading phase - determines status message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadingPhase {
    /// Initial connection phase
    Connecting,
    /// Model warm-up phase
    WarmingUp,
    /// Final initialization phase
    Ready,
}

impl LoadingScreen {
    /// Create a new loading screen
    ///
    /// Pre-builds animation frames and initializes state.
    pub fn new() -> Self {
        let frames = Self::build_frames();

        Self {
            elapsed: Duration::from_millis(0),
            phase: LoadingPhase::Connecting,
            current_frame: 0,
            message_index: 0,
            frames,
        }
    }

    /// Build the three animation frames using sprite system
    ///
    /// Uses the existing build_frame() helper from avatar/sprites.rs
    /// with the axolotl color palette.
    fn build_frames() -> [AvatarFrame; 3] {
        // Define color map for sprite pattern
        // Pattern characters:
        //   'T' = '▀' (top half) in BODY color
        //   'L' = '▄' (lower half) in BODY color
        //   'B' = '█' (full block) in BODY color
        //   'o' = 'o' (letter) in EYES color
        //   '^' = '^' (caret) in EYES color
        //   '.' = '.' (period) in EYES color
        let palette = vec![
            ('T', '▀', AXOLOTL_BODY),
            ('L', '▄', AXOLOTL_BODY),
            ('B', '█', AXOLOTL_BODY),
            ('o', 'o', AXOLOTL_EYES),
            ('^', '^', AXOLOTL_EYES),
            ('.', '.', AXOLOTL_EYES),
        ];

        [
            // Frame 1: Dim state with open eyes (oo)
            // Gills will be colored dim (base breathing color)
            build_frame(
                &[
                    " TLLT ",  // Head: top and bottom blocks
                    " BooBL",  // Face: open eyes (oo)
                ],
                &palette,
                100, // Duration: 100ms per frame
            ),

            // Frame 2: Bright state with happy eyes (^^)
            // Gills will be colored bright (peak breathing color)
            build_frame(
                &[
                    " TLLT ",  // Same head
                    " B^^BL",  // Face: happy eyes (^^)
                ],
                &palette,
                100,
            ),

            // Frame 3: Curious eyes (..)
            // Back to dim state
            build_frame(
                &[
                    " TLLT ",
                    " Bo.BL",  // Face: one open, one curious
                ],
                &palette,
                100,
            ),
        ]
    }

    /// Update animation state based on elapsed time
    ///
    /// Called every render tick (~100ms). Updates frame index and
    /// message index based on elapsed time.
    pub fn tick(&mut self, delta: Duration) {
        self.elapsed += delta;

        // Calculate frame index based on 333ms per frame (1 second total cycle)
        // 0-333ms: frame 0
        // 333-666ms: frame 1
        // 666-1000ms: frame 2
        // 1000+ms: repeats
        let elapsed_in_cycle = self.elapsed.as_millis() % 1000;
        self.current_frame = (elapsed_in_cycle / 333) as usize % 3;

        // Calculate message index based on 2000ms per message
        let elapsed_message = self.elapsed.as_millis() % 6000;
        self.message_index = (elapsed_message / 2000) as usize % 3;
    }

    /// Set the current loading phase
    ///
    /// Phase determines which status message is shown if message_index == 0.
    /// In real implementation, this would be called as connection progresses.
    pub fn set_phase(&mut self, phase: LoadingPhase) {
        self.phase = phase;
    }

    /// Get the status message for current phase and message index
    fn get_message(&self) -> &'static str {
        match self.message_index {
            0 => "Just starting...",
            1 => "Warming up...",
            2 => "Almost there...",
            _ => "Just starting...",
        }
    }

    /// Calculate gills color based on breathing animation
    ///
    /// Uses the existing breathing_color() function to smoothly
    /// oscillate between dim and bright coral colors over 1 second.
    fn get_gills_color(&self) -> ratatui::style::Color {
        breathing_color(
            AXOLOTL_GILLS,              // Dim state (base coral)
            AXOLOTL_GILLS_HIGHLIGHT,    // Bright state (bright coral)
            1000,                        // 1 second cycle
            self.elapsed,
        )
    }

    /// Calculate text color based on breathing animation
    ///
    /// Text color pulses in sync with gills, transitioning from
    /// dim gray to bright magenta.
    fn get_text_color(&self) -> ratatui::style::Color {
        breathing_color(
            DIM_GRAY,          // Dim baseline
            YOLLAYAH_MAGENTA,  // Bright accent
            1000,              // Same cycle as gills
            self.elapsed,
        )
    }

    /// Render the loading screen
    ///
    /// Draws:
    /// 1. Status message (centered, with breathing color)
    /// 2. Axolotl sprite (centered below message, with breathing gills)
    /// 3. "Press Ctrl+C to cancel" footer
    pub fn render(&self, f: &mut Frame) {
        let area = f.area();

        // Create layout: top/center/bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(40),  // Top spacer
                Constraint::Max(10),         // Message + sprite area
                Constraint::Percentage(40),  // Bottom spacer with footer
            ])
            .split(area);

        // ===== RENDER STATUS MESSAGE =====
        let message = self.get_message();
        let text_color = self.get_text_color();

        let message_widget = Paragraph::new(
            Line::from(vec![
                Span::styled(message, ratatui::style::Style::default().fg(text_color))
            ])
        )
        .alignment(Alignment::Center);

        f.render_widget(message_widget, chunks[1]);

        // ===== RENDER AXOLOTL SPRITE =====
        // Current frame with breathing colors applied
        let frame = &self.frames[self.current_frame];

        // Calculate sprite position (centered)
        let sprite_y = chunks[1].y + 1; // Below message
        let sprite_x = (chunks[1].width.saturating_sub(frame.width)) / 2 + chunks[1].x;

        let gills_color = self.get_gills_color();

        // Render each cell of the sprite with appropriate colors
        for (y, row) in frame.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if cell.is_transparent() {
                    continue; // Skip empty cells
                }

                // Apply breathing color to gills
                // In a real implementation, we'd check cell colors and apply
                // breathing only to gill-colored cells
                let render_color = if cell.fg == AXOLOTL_GILLS {
                    gills_color // Breathing color for gills
                } else {
                    cell.fg // Normal color for body/eyes
                };

                let cell_x = sprite_x + x as u16;
                let cell_y = sprite_y + y as u16;

                if cell_x < area.right() && cell_y < area.bottom() {
                    f.buffer_mut().get_mut(cell_x, cell_y).set_char(cell.ch);
                    f.buffer_mut()
                        .get_mut(cell_x, cell_y)
                        .set_fg(render_color);
                }
            }
        }

        // ===== RENDER FOOTER =====
        let footer_area = Rect {
            x: area.x,
            y: area.bottom().saturating_sub(2),
            width: area.width,
            height: 2,
        };

        let footer = Paragraph::new(
            Line::from(vec![
                Span::raw("Press "),
                Span::styled(
                    "Ctrl+C",
                    ratatui::style::Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(" to cancel"),
            ])
        )
        .alignment(Alignment::Center)
        .style(ratatui::style::Style::default().fg(DIM_GRAY));

        f.render_widget(footer, footer_area);
    }
}

impl Default for LoadingScreen {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// USAGE EXAMPLE (in main app startup loop)
// ============================================================================
//
// See STORY 4 in TODO-tui-initialization.md for integration pattern:
//
// ```rust
// #[tokio::main]
// async fn main() -> Result<()> {
//     // Setup terminal...
//     let mut terminal = setup_terminal()?;
//
//     // IMMEDIATE: Show loading screen
//     let mut loading = LoadingScreen::new();
//
//     // BACKGROUND: Connect to Conductor
//     let conductor_task = tokio::spawn(async {
//         Conductor::connect_with_retry(3, Duration::from_secs(5)).await
//     });
//
//     // LOOP: Animate loading while waiting
//     let mut last_render = Instant::now();
//     loop {
//         tokio::select! {
//             Ok(conductor) = &mut conductor_task => {
//                 // Connected! Break to main UI
//                 break;
//             }
//             _ = tokio::time::sleep(Duration::from_millis(100)) => {
//                 // Update animation
//                 loading.tick(Duration::from_millis(100));
//                 loading.set_phase(LoadingPhase::Connecting);
//
//                 // Render
//                 terminal.draw(|f| {
//                     loading.render(f);
//                 })?;
//             }
//             Some(event) = event_receiver.recv() => {
//                 // Handle Ctrl+C during loading
//                 if should_quit(&event) {
//                     return Ok(());
//                 }
//             }
//         }
//     }
//
//     // Continue with normal app...
// }
// ```

// ============================================================================
// TESTING CHECKLIST
// ============================================================================
//
// Visual Tests (manual):
// [ ] Launch on 80x24 terminal - sprite visible and centered
// [ ] Launch on 120x40 terminal - sprite still centered
// [ ] Watch 10 seconds - verify no color flickering
// [ ] Watch 10 seconds - verify smooth frame transitions
// [ ] Verify color breathing (gills dim → bright → dim)
// [ ] Verify text color breathing (gray → magenta → gray)
// [ ] Verify message rotation at ~2 second intervals
// [ ] Press Ctrl+C - verify clean exit (wired in app.rs later)
//
// Performance Tests:
// [ ] First render within 50ms
// [ ] Frame rate stable at 10fps (100ms per frame)
// [ ] CPU usage minimal (<1% on idle system)
// [ ] Memory footprint <500KB
//
// Edge Cases:
// [ ] Very small terminal (60x20) - still renders
// [ ] Very large terminal (200x60) - still centered
// [ ] Rapid phase changes - transitions smooth
// [ ] Long loading (5+ minutes) - no memory leaks

// ============================================================================
// IMPLEMENTATION NOTES
// ============================================================================
//
// 1. COLOR APPLICATION
//    The breathing_color() function returns an interpolated color
//    based on elapsed time and a sine wave. It handles the math
//    for smooth oscillation between base and bright colors.
//
//    No need to implement oscillation yourself - just call:
//    let color = breathing_color(base, bright, cycle_ms, elapsed);
//
// 2. SPRITE RENDERING
//    The build_frame() function is already proven in avatar/sizes.rs.
//    It takes a pattern string array and palette map, returns Frame.
//
//    Don't create custom sprite building - reuse this infrastructure.
//
// 3. FRAME TIMING
//    100ms per frame tick = 10fps animation
//    This is smooth enough for human perception without wasting CPU.
//
//    If smoother animation needed, can reduce to 50ms (20fps) but
//    unlikely to matter for a loading screen.
//
// 4. PHASE MANAGEMENT
//    Phase changes happen in the app.rs startup sequence as
//    connection progresses. Loading screen just reads current phase.
//
//    Don't implement phase logic here - just render it.
//
// 5. EVENT HANDLING
//    Ctrl+C handling happens in app.rs main event loop.
//    LoadingScreen is just a dumb rendering component.
//
//    Don't add event handling to LoadingScreen itself.

// ============================================================================
// FUTURE ENHANCEMENTS (not for initial implementation)
// ============================================================================
//
// These are possible improvements after Option B is working:
//
// 1. DYNAMIC EYE BLINKING
//    Random eye blinks every 3-5 seconds
//    Current: eyes fixed per frame (oo, ^^, ..)
//    Future: Add blink() method that briefly changes to (--)
//
// 2. DIFFERENT EXPRESSIONS PER PHASE
//    Phase::Connecting: neutral eyes
//    Phase::WarmingUp: happy eyes
//    Phase::Ready: very happy eyes
//
// 3. PROGRESS PERCENTAGE
//    Add estimated % progress based on phase
//    Not real progress, but gives user sense of forward movement
//
// 4. UPGRADE PATH TO OPTION C
//    Embed this LoadingScreen sprite as the swimmer in Option C
//    Use same color breathing and animation logic
//    No code waste - progressive enhancement

