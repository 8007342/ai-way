//! Activity Indicators and Overlays
//!
//! Provides visual indicators for what Yollayah is doing:
//! - Thinking: thought bubbles, question marks, gears turning
//! - Construction: hard hat, tools, building
//! - Study: glasses, books, graduation cap
//! - Engineering: gears, circuits, wrenches
//! - Gardening: watering can, flowers, leaves
//! - Cooking: chef hat, spatula, steam
//!
//! These overlay on top of the base avatar sprite to show activity context.
//! Designed for extensibility - new activities can be added easily.

use std::time::Duration;

use ratatui::style::Color;

use super::sprites::{build_frame, ColoredCell};

/// Types of activities Yollayah can be doing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Activity {
    /// No specific activity (base sprite only)
    None,
    /// Thinking/processing - thought bubbles, pondering
    Thinking,
    /// Construction work - building something
    Construction,
    /// Studying/learning - reading, researching
    Study,
    /// Engineering - technical problem solving
    Engineering,
    /// Gardening - nurturing, growing
    Gardening,
    /// Cooking - preparing, mixing
    Cooking,
    /// Security - checking, protecting (for Cousin Rita)
    Security,
    /// Database - data work (for Tia Rosa)
    Database,
    /// Design - creative work (for Cousin Lucia)
    Design,
    /// Testing - QA work (for The Intern)
    Testing,
}

impl Activity {
    /// Get a friendly name for the activity
    pub fn name(&self) -> &'static str {
        match self {
            Activity::None => "idle",
            Activity::Thinking => "thinking",
            Activity::Construction => "building",
            Activity::Study => "studying",
            Activity::Engineering => "engineering",
            Activity::Gardening => "gardening",
            Activity::Cooking => "cooking",
            Activity::Security => "securing",
            Activity::Database => "querying",
            Activity::Design => "designing",
            Activity::Testing => "testing",
        }
    }

    /// Get activity from specialist agent type
    pub fn from_agent(agent: &str) -> Self {
        match agent {
            "ethical-hacker" => Activity::Security,
            "backend-engineer" => Activity::Engineering,
            "frontend-specialist" => Activity::Design,
            "senior-full-stack-developer" => Activity::Engineering,
            "solutions-architect" => Activity::Construction,
            "ux-ui-designer" => Activity::Design,
            "qa-engineer" => Activity::Testing,
            "privacy-researcher" => Activity::Security,
            "devops-engineer" => Activity::Construction,
            "relational-database-expert" => Activity::Database,
            _ => Activity::Thinking,
        }
    }
}

/// Size of overlay (matches AvatarSize concept)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OverlaySize {
    Tiny,
    Small,
    Medium,
    Large,
}

/// An animated overlay that floats near the avatar
pub struct ActivityOverlay {
    /// Frames of the overlay animation
    pub frames: Vec<OverlayFrame>,
    /// Current frame index
    current_frame: usize,
    /// Time on current frame
    frame_time: Duration,
    /// Whether the overlay loops
    pub looping: bool,
    /// Position offset from avatar (x, y) - can be negative
    pub offset: (i16, i16),
}

/// A single frame of an overlay
pub struct OverlayFrame {
    /// The cells to draw
    pub cells: Vec<Vec<ColoredCell>>,
    /// Width
    pub width: u16,
    /// Height
    pub height: u16,
    /// Duration in ms
    pub duration_ms: u64,
}

impl OverlayFrame {
    /// Get cell at position
    pub fn get(&self, x: u16, y: u16) -> Option<&ColoredCell> {
        self.cells
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .filter(|c| !c.is_empty())
    }
}

impl ActivityOverlay {
    /// Update the overlay animation
    pub fn update(&mut self, delta: Duration) {
        if self.frames.is_empty() {
            return;
        }

        let frame = &self.frames[self.current_frame];
        let frame_duration = Duration::from_millis(frame.duration_ms);

        self.frame_time += delta;

        if self.frame_time >= frame_duration {
            self.frame_time = Duration::ZERO;
            self.current_frame += 1;

            if self.current_frame >= self.frames.len() {
                if self.looping {
                    self.current_frame = 0;
                } else {
                    self.current_frame = self.frames.len() - 1;
                }
            }
        }
    }

    /// Get current frame for rendering
    pub fn current_frame(&self) -> Option<&OverlayFrame> {
        self.frames.get(self.current_frame)
    }

    /// Reset animation to start
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.frame_time = Duration::ZERO;
    }
}

// ============================================================================
// Overlay Colors
// ============================================================================

const THOUGHT_BUBBLE: Color = Color::Rgb(200, 200, 220);
const THOUGHT_DOT: Color = Color::Rgb(180, 180, 200);
const GEAR_COLOR: Color = Color::Rgb(150, 150, 170);
const SPARKLE: Color = Color::Rgb(255, 255, 200);
const TOOL_COLOR: Color = Color::Rgb(180, 140, 100);
const LEAF_COLOR: Color = Color::Rgb(100, 180, 100);
#[allow(dead_code)] // For future cooking activity
const FLAME_COLOR: Color = Color::Rgb(255, 150, 50);
const BOOK_COLOR: Color = Color::Rgb(139, 90, 43);
const SHIELD_COLOR: Color = Color::Rgb(100, 150, 200);

// ============================================================================
// Overlay Builders
// ============================================================================

/// Build overlay frame from pattern
fn build_overlay_frame(
    pattern: &[&str],
    palette: &[(char, char, Color)],
    duration_ms: u64,
) -> OverlayFrame {
    let frame = build_frame(pattern, palette, duration_ms);
    OverlayFrame {
        cells: frame.cells,
        width: frame.width,
        height: frame.height,
        duration_ms,
    }
}

/// Create thinking overlay - thought bubble with animated dots
pub fn thinking_overlay(size: OverlaySize) -> ActivityOverlay {
    let palette = vec![
        ('O', '(', THOUGHT_BUBBLE), // Bubble left
        ('C', ')', THOUGHT_BUBBLE), // Bubble right
        ('_', '_', THOUGHT_BUBBLE), // Bubble bottom
        ('^', '^', THOUGHT_BUBBLE), // Bubble top
        ('.', '.', THOUGHT_DOT),    // Small dot
        ('o', 'o', THOUGHT_DOT),    // Medium dot
        ('*', '*', SPARKLE),        // Sparkle
        ('?', '?', GEAR_COLOR),     // Question mark
    ];

    match size {
        OverlaySize::Tiny => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&[".", " "], &palette, 300),
                build_overlay_frame(&["o", "."], &palette, 300),
                build_overlay_frame(&[".", "o"], &palette, 300),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-2, -1),
        },
        OverlaySize::Small => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&[".  ", " . ", "  ."], &palette, 250),
                build_overlay_frame(&[" . ", "  .", ".  "], &palette, 250),
                build_overlay_frame(&["  .", ".  ", " . "], &palette, 250),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-3, -2),
        },
        OverlaySize::Medium => ActivityOverlay {
            frames: vec![
                build_overlay_frame(
                    &[" .o. ", ".   .", "o ? o", ".   .", " .o. "],
                    &palette,
                    400,
                ),
                build_overlay_frame(
                    &[" o.o ", "o   o", ". ? .", "o   o", " o.o "],
                    &palette,
                    400,
                ),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-6, -3),
        },
        OverlaySize::Large => ActivityOverlay {
            frames: vec![
                build_overlay_frame(
                    &[
                        "  . o .  ",
                        " .     . ",
                        "o   ?   o",
                        " .     . ",
                        "  . o .  ",
                    ],
                    &palette,
                    350,
                ),
                build_overlay_frame(
                    &[
                        "  o . o  ",
                        " o     o ",
                        ".   ?   .",
                        " o     o ",
                        "  o . o  ",
                    ],
                    &palette,
                    350,
                ),
                build_overlay_frame(
                    &[
                        "  * . *  ",
                        " .     . ",
                        "o   ?   o",
                        " .     . ",
                        "  * . *  ",
                    ],
                    &palette,
                    350,
                ),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-10, -4),
        },
    }
}

/// Create construction overlay - hard hat and tools
pub fn construction_overlay(size: OverlaySize) -> ActivityOverlay {
    let palette = vec![
        ('H', '=', Color::Yellow), // Hard hat
        ('h', '█', Color::Yellow), // Hard hat body
        ('T', '/', TOOL_COLOR),    // Tool
        ('t', '\\', TOOL_COLOR),   // Tool
        ('.', '.', SPARKLE),       // Spark
    ];

    match size {
        OverlaySize::Tiny | OverlaySize::Small => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&["H"], &palette, 500),
                build_overlay_frame(&["H."], &palette, 200),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (0, -1),
        },
        OverlaySize::Medium | OverlaySize::Large => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&[" HHH ", " hhh ", "T   t"], &palette, 400),
                build_overlay_frame(&[" HHH.", " hhh ", "T . t"], &palette, 200),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-3, -3),
        },
    }
}

/// Create study overlay - glasses and book
pub fn study_overlay(size: OverlaySize) -> ActivityOverlay {
    let palette = vec![
        ('G', 'o', Color::Rgb(100, 100, 100)), // Glasses
        ('-', '-', Color::Rgb(100, 100, 100)), // Glasses bridge
        ('B', '█', BOOK_COLOR),                // Book
        ('b', '▀', BOOK_COLOR),                // Book top
        ('*', '*', SPARKLE),                   // Sparkle
    ];

    match size {
        OverlaySize::Tiny | OverlaySize::Small => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&["G-G"], &palette, 800),
                build_overlay_frame(&["G-G*"], &palette, 200),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (0, 0),
        },
        OverlaySize::Medium | OverlaySize::Large => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&["G-G  ", "     ", " bBb "], &palette, 600),
                build_overlay_frame(&["G-G *", "     ", " bBb "], &palette, 300),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-2, -1),
        },
    }
}

/// Create engineering overlay - gears turning
pub fn engineering_overlay(size: OverlaySize) -> ActivityOverlay {
    let palette = vec![
        ('*', '*', GEAR_COLOR), // Gear tooth
        ('o', 'o', GEAR_COLOR), // Gear center
        ('.', '.', SPARKLE),    // Spark
    ];

    match size {
        OverlaySize::Tiny | OverlaySize::Small => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&["*o*"], &palette, 200),
                build_overlay_frame(&["o*o"], &palette, 200),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-2, -1),
        },
        OverlaySize::Medium | OverlaySize::Large => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&[" *o* ", "*o o*", " *o* "], &palette, 200),
                build_overlay_frame(&["*o*o*", "o . o", "*o*o*"], &palette, 200),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-6, -3),
        },
    }
}

/// Create cooking overlay - chef hat and steam
pub fn cooking_overlay(size: OverlaySize) -> ActivityOverlay {
    let palette = vec![
        ('C', '█', Color::White),              // Chef hat
        ('c', '▀', Color::White),              // Chef hat top
        ('~', '~', Color::Rgb(200, 200, 200)), // Steam
        ('s', '°', Color::Rgb(180, 180, 180)), // Steam particle
    ];

    match size {
        OverlaySize::Tiny | OverlaySize::Small => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&["cC", " ~"], &palette, 300),
                build_overlay_frame(&["cC", "~ "], &palette, 300),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (0, -2),
        },
        OverlaySize::Medium | OverlaySize::Large => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&[" s ~ s ", "  ccc  ", "  CCC  "], &palette, 300),
                build_overlay_frame(&[" ~ s ~ ", "  ccc  ", "  CCC  "], &palette, 300),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-3, -3),
        },
    }
}

/// Create security overlay - shield
pub fn security_overlay(size: OverlaySize) -> ActivityOverlay {
    let palette = vec![
        ('S', '█', SHIELD_COLOR), // Shield body
        ('s', '▀', SHIELD_COLOR), // Shield top
        ('v', 'v', SHIELD_COLOR), // Shield bottom
        ('*', '*', SPARKLE),      // Sparkle
    ];

    match size {
        OverlaySize::Tiny | OverlaySize::Small => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&["sS"], &palette, 500),
                build_overlay_frame(&["sS*"], &palette, 200),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-2, -1),
        },
        OverlaySize::Medium | OverlaySize::Large => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&[" sSs ", " SSS ", "  v  "], &palette, 400),
                build_overlay_frame(&["*sSs*", " SSS ", "  v  "], &palette, 200),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-3, -3),
        },
    }
}

/// Create gardening overlay - leaf and water drops
pub fn gardening_overlay(size: OverlaySize) -> ActivityOverlay {
    let palette = vec![
        ('L', '♣', LEAF_COLOR),  // Leaf
        ('l', '~', LEAF_COLOR),  // Leaf small
        ('.', '.', Color::Cyan), // Water drop
        ('o', 'o', Color::Cyan), // Water drop large
    ];

    match size {
        OverlaySize::Tiny | OverlaySize::Small => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&["L.", ".L"], &palette, 400),
                build_overlay_frame(&[".L", "L."], &palette, 400),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-2, -1),
        },
        OverlaySize::Medium | OverlaySize::Large => ActivityOverlay {
            frames: vec![
                build_overlay_frame(&[" L.L ", ".   .", " l l "], &palette, 350),
                build_overlay_frame(&[".L L.", " . . ", "l   l"], &palette, 350),
            ],
            current_frame: 0,
            frame_time: Duration::ZERO,
            looping: true,
            offset: (-3, -3),
        },
    }
}

// ============================================================================
// Activity Overlay Manager
// ============================================================================

/// Manages activity overlays for the avatar
pub struct ActivityManager {
    /// Current activity
    current_activity: Activity,
    /// Current overlay (if any)
    overlay: Option<ActivityOverlay>,
    /// Overlay size to use
    size: OverlaySize,
}

impl ActivityManager {
    /// Create a new activity manager
    pub fn new() -> Self {
        Self {
            current_activity: Activity::None,
            overlay: None,
            size: OverlaySize::Medium,
        }
    }

    /// Set the current activity
    pub fn set_activity(&mut self, activity: Activity) {
        if self.current_activity != activity {
            self.current_activity = activity;
            self.overlay = self.create_overlay(activity);
        }
    }

    /// Set overlay size
    pub fn set_size(&mut self, size: OverlaySize) {
        if self.size != size {
            self.size = size;
            // Recreate overlay with new size
            self.overlay = self.create_overlay(self.current_activity);
        }
    }

    /// Get current activity
    pub fn current_activity(&self) -> Activity {
        self.current_activity
    }

    /// Update overlay animation
    pub fn update(&mut self, delta: Duration) {
        if let Some(ref mut overlay) = self.overlay {
            overlay.update(delta);
        }
    }

    /// Get current overlay for rendering
    pub fn overlay(&self) -> Option<&ActivityOverlay> {
        self.overlay.as_ref()
    }

    /// Create overlay for activity
    fn create_overlay(&self, activity: Activity) -> Option<ActivityOverlay> {
        match activity {
            Activity::None => None,
            Activity::Thinking => Some(thinking_overlay(self.size)),
            Activity::Construction => Some(construction_overlay(self.size)),
            Activity::Study => Some(study_overlay(self.size)),
            Activity::Engineering => Some(engineering_overlay(self.size)),
            Activity::Gardening => Some(gardening_overlay(self.size)),
            Activity::Cooking => Some(cooking_overlay(self.size)),
            Activity::Security => Some(security_overlay(self.size)),
            Activity::Database => Some(engineering_overlay(self.size)), // Reuse gears for now
            Activity::Design => Some(study_overlay(self.size)),         // Reuse glasses for now
            Activity::Testing => Some(engineering_overlay(self.size)),  // Reuse gears for now
        }
    }
}

impl Default for ActivityManager {
    fn default() -> Self {
        Self::new()
    }
}
