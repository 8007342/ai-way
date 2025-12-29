//! Avatar Size Variants with Blocky Pixel Art
//!
//! Sprites use Unicode block elements (█▀▄▌▐) for chunky pixel-art aesthetic.
//! Colors convey expression; characters only for accents (eyes, etc).
//!
//! Block elements reference: https://en.wikipedia.org/wiki/Block_Elements

use std::collections::HashMap;

use super::sprites::{build_animation, build_frame, Animation, SpriteSheet};
use crate::theme::*;

/// Avatar size categories
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarSize {
    /// 6x2 - peeking, subtle presence
    Tiny,
    /// 10x3 - quick status, background
    Small,
    /// 16x5 - normal interactions (default)
    Medium,
    /// 24x8 - celebrations, jokes, big moments
    Large,
}

impl AvatarSize {
    /// Maximum bounds (width, height) for this size
    pub fn max_bounds(&self) -> (u16, u16) {
        match self {
            Self::Tiny => (8, 2),
            Self::Small => (12, 4),
            Self::Medium => (18, 6),
            Self::Large => (26, 10),
        }
    }

    /// Z-index modifier (larger = more prominent)
    pub fn z_modifier(&self) -> i32 {
        match self {
            Self::Tiny => 0,
            Self::Small => 10,
            Self::Medium => 20,
            Self::Large => 50,
        }
    }
}

/// Load all sprite sheets for all sizes
pub fn load_all_sprites() -> HashMap<AvatarSize, SpriteSheet> {
    let mut sheets = HashMap::new();

    sheets.insert(AvatarSize::Tiny, load_tiny());
    sheets.insert(AvatarSize::Small, load_small());
    sheets.insert(AvatarSize::Medium, load_medium());
    sheets.insert(AvatarSize::Large, load_large());

    sheets
}

// ============================================================================
// Palette Keys (used in sprite patterns)
// ============================================================================
// B = Body (main pink)
// b = Body shadow (darker pink)
// H = Body highlight (lighter pink)
// G = Gills (coral)
// g = Gills highlight
// E = Eyes (dark)
// S = Eye shine (white dot)
// M = Mouth
// V = Belly (warm light)
// W = Water effect
// U = Bubble
// ! = Happy/excited accent (yellow)
// ? = Thinking accent (blue)
// X = Error accent (red)

fn base_palette() -> Vec<(char, char, ratatui::style::Color)> {
    vec![
        // Body
        ('B', '█', AXOLOTL_BODY),
        ('b', '█', AXOLOTL_BODY_SHADOW),
        ('H', '█', AXOLOTL_BODY_HIGHLIGHT),
        // Body contours
        ('T', '▀', AXOLOTL_BODY),           // Top half
        ('L', '▄', AXOLOTL_BODY),           // Lower half
        ('t', '▀', AXOLOTL_BODY_SHADOW),
        ('l', '▄', AXOLOTL_BODY_SHADOW),
        ('[', '▌', AXOLOTL_BODY),           // Left half
        (']', '▐', AXOLOTL_BODY),           // Right half
        // Gills
        ('G', '█', AXOLOTL_GILLS),
        ('g', '█', AXOLOTL_GILLS_HIGHLIGHT),
        (')', '▐', AXOLOTL_GILLS),          // Right gill edge
        ('(', '▌', AXOLOTL_GILLS),          // Left gill edge
        ('/', '▀', AXOLOTL_GILLS),          // Gill top
        ('\\', '▄', AXOLOTL_GILLS),         // Gill bottom
        // Eyes (character accents)
        ('o', 'o', AXOLOTL_EYES),           // Open eye
        ('.', '.', AXOLOTL_EYES),           // Tiny eye
        ('-', '-', AXOLOTL_EYES),           // Closed/blink
        ('^', '^', AXOLOTL_EYES),           // Happy eye
        ('*', '*', AXOLOTL_EYE_SHINE),      // Sparkle
        // Mouth (character accents)
        ('w', 'w', AXOLOTL_MOUTH),          // Smile
        ('v', 'v', AXOLOTL_MOUTH),          // Small smile
        ('n', 'n', AXOLOTL_MOUTH),          // Uncertain
        ('O', 'O', AXOLOTL_MOUTH),          // Surprised
        // Belly
        ('V', '█', AXOLOTL_BELLY),
        ('v', '▀', AXOLOTL_BELLY),
        // Effects
        ('W', '~', WATER_BLUE),             // Water
        ('U', '°', BUBBLE),                 // Bubble
        ('!', '!', MOOD_HAPPY),             // Excited
        ('?', '?', MOOD_THINKING),          // Thinking
        ('X', 'x', MOOD_ERROR),             // Error
        ('#', '♪', MOOD_HAPPY),             // Music note
    ]
}

// ============================================================================
// TINY (6x2) - Just a peeking head
// ============================================================================

fn load_tiny() -> SpriteSheet {
    let mut animations = HashMap::new();
    let p = base_palette();

    // Idle - simple blink cycle
    animations.insert(
        "idle".to_string(),
        build_animation(
            "idle",
            &[
                (&[
                    " TLLT ",
                    " BooBL",
                ], 2000),
                (&[
                    " TLLT ",
                    " B--BL",
                ], 150),
            ],
            &p,
            true,
        ),
    );

    // Thinking
    animations.insert(
        "thinking".to_string(),
        build_animation(
            "thinking",
            &[
                (&[
                    "?TLLT ",
                    " BooBL",
                ], 400),
                (&[
                    " TLLT?",
                    " Bo.BL",
                ], 400),
            ],
            &p,
            true,
        ),
    );

    // Talking
    animations.insert(
        "talking".to_string(),
        build_animation(
            "talking",
            &[
                (&[
                    " TLLT ",
                    " BoOBL",
                ], 200),
                (&[
                    " TLLT ",
                    " BooBL",
                ], 200),
            ],
            &p,
            true,
        ),
    );

    // Happy
    animations.insert(
        "happy".to_string(),
        build_animation(
            "happy",
            &[
                (&[
                    " TLLT!",
                    " B^^BL",
                ], 300),
            ],
            &p,
            true,
        ),
    );

    SpriteSheet {
        animations,
        default: "idle".to_string(),
    }
}

// ============================================================================
// SMALL (10x3) - Head with gills
// ============================================================================

fn load_small() -> SpriteSheet {
    let mut animations = HashMap::new();
    let p = base_palette();

    // Idle
    animations.insert(
        "idle".to_string(),
        build_animation(
            "idle",
            &[
                (&[
                    "  /TLT\\  ",
                    " GBo oBG ",
                    "  \\LwL/  ",
                ], 2000),
                (&[
                    "  /TLT\\  ",
                    " GBo oBG ",
                    "  \\L-L/  ",  // blink + mouth change
                ], 150),
            ],
            &p,
            true,
        ),
    );

    // Thinking
    animations.insert(
        "thinking".to_string(),
        build_animation(
            "thinking",
            &[
                (&[
                    " ?/TLT\\  ",
                    " GBo oBG ",
                    "  \\LnL/  ",
                ], 500),
                (&[
                    "  /TLT\\? ",
                    " GBo.oBG ",
                    "  \\LnL/  ",
                ], 500),
            ],
            &p,
            true,
        ),
    );

    // Talking
    animations.insert(
        "talking".to_string(),
        build_animation(
            "talking",
            &[
                (&[
                    "  /TLT\\  ",
                    " GBo oBG ",
                    "  \\LOL/  ",
                ], 200),
                (&[
                    "  /TLT\\  ",
                    " GBo oBG ",
                    "  \\LwL/  ",
                ], 200),
            ],
            &p,
            true,
        ),
    );

    // Happy
    animations.insert(
        "happy".to_string(),
        build_animation(
            "happy",
            &[
                (&[
                    "! /TLT\\ !",
                    " GB^ ^BG ",
                    "  \\LwL/  ",
                ], 300),
                (&[
                    " !/TLT\\! ",
                    " GB^ ^BG ",
                    "  \\LwL/  ",
                ], 300),
            ],
            &p,
            true,
        ),
    );

    // Swimming
    animations.insert(
        "swimming".to_string(),
        build_animation(
            "swimming",
            &[
                (&[
                    "  /TLT\\WW",
                    " GBo oBG ",
                    "  \\LwL/  ",
                ], 300),
                (&[
                    " W/TLT\\ W",
                    " GBo oBG ",
                    "  \\LwL/WW",
                ], 300),
            ],
            &p,
            true,
        ),
    );

    SpriteSheet {
        animations,
        default: "idle".to_string(),
    }
}

// ============================================================================
// MEDIUM (16x5) - Full body, main interaction size
// ============================================================================

fn load_medium() -> SpriteSheet {
    let mut animations = HashMap::new();
    let p = base_palette();

    // Idle - gentle breathing/blink
    animations.insert(
        "idle".to_string(),
        build_animation(
            "idle",
            &[
                (&[
                    "    /TTTT\\    ",
                    "  G BBo oBB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 2500),
                (&[
                    "    /TTTT\\    ",
                    "  G BB- -BB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 150),
            ],
            &p,
            true,
        ),
    );

    // Thinking - head tilt, question marks
    animations.insert(
        "thinking".to_string(),
        build_animation(
            "thinking",
            &[
                (&[
                    "   ?/TTTT\\    ",
                    "  G BBo.oBB G ",
                    " GG BB n BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 600),
                (&[
                    "    /TTTT\\?   ",
                    "  G BBo oBB G ",
                    " GG BB n BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 600),
            ],
            &p,
            true,
        ),
    );

    // Talking - mouth animation
    animations.insert(
        "talking".to_string(),
        build_animation(
            "talking",
            &[
                (&[
                    "    /TTTT\\    ",
                    "  G BBo oBB G ",
                    " GG BB O BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 200),
                (&[
                    "    /TTTT\\    ",
                    "  G BBo oBB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 200),
            ],
            &p,
            true,
        ),
    );

    // Happy - bouncy, sparkly
    animations.insert(
        "happy".to_string(),
        build_animation(
            "happy",
            &[
                (&[
                    "  ! /TTTT\\ !  ",
                    "  G BB^ ^BB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 300),
                (&[
                    " !  /TTTT\\  ! ",
                    "  G BB^ ^BB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 300),
            ],
            &p,
            true,
        ),
    );

    // Swimming - water effects, tail motion
    animations.insert(
        "swimming".to_string(),
        build_animation(
            "swimming",
            &[
                (&[
                    "    /TTTT\\ WW ",
                    "  G BBo oBB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    "   L  LL  LWW ",
                ], 300),
                (&[
                    "  WW/TTTT\\    ",
                    "  G BBo oBB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    " WW L  LL  L  ",
                ], 300),
            ],
            &p,
            true,
        ),
    );

    // Waiting - idle variant with occasional look around
    animations.insert(
        "waiting".to_string(),
        build_animation(
            "waiting",
            &[
                (&[
                    "   #/TTTT\\    ",
                    "  G BBo oBB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 800),
                (&[
                    "    /TTTT\\#   ",
                    "  G BBo oBB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 800),
            ],
            &p,
            true,
        ),
    );

    // Error - confused, X marks
    animations.insert(
        "error".to_string(),
        build_animation(
            "error",
            &[
                (&[
                    "  X /TTTT\\ X  ",
                    "  G BBo oBB G ",
                    " GG BB n BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 500),
                (&[
                    " X  /TTTT\\  X ",
                    "  G BB. .BB G ",
                    " GG BB n BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ], 500),
            ],
            &p,
            true,
        ),
    );

    // Wink - single frame non-looping
    animations.insert(
        "wink".to_string(),
        Animation {
            name: "wink".to_string(),
            frames: vec![build_frame(
                &[
                    "    /TTTT\\    ",
                    "  G BB^ oBB G ",
                    " GG BB w BB GG",
                    "    BVVVVB    ",
                    "   L  LL  L   ",
                ],
                &p,
                800,
            )],
            looping: false,
        },
    );

    SpriteSheet {
        animations,
        default: "idle".to_string(),
    }
}

// ============================================================================
// LARGE (24x8) - Big moments, celebrations
// ============================================================================

fn load_large() -> SpriteSheet {
    let mut animations = HashMap::new();
    let p = base_palette();

    // Idle - majestic presence
    animations.insert(
        "idle".to_string(),
        build_animation(
            "idle",
            &[
                (&[
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBBo  oBBBB  G   ",
                    "  GGG BBBB w  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                    "    WWW          WWW    ",
                ], 3000),
                (&[
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBB-  -BBBB  G   ",
                    "  GGG BBBB w  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                    "    WWW          WWW    ",
                ], 200),
            ],
            &p,
            true,
        ),
    );

    // Happy/Celebrating - bouncy with effects
    animations.insert(
        "happy".to_string(),
        build_animation(
            "happy",
            &[
                (&[
                    "  !     !    !     !    ",
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBB^  ^BBBB  G   ",
                    "  GGG BBBB w  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                ], 250),
                (&[
                    "    !      !      !     ",
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBB^  ^BBBB  G   ",
                    "  GGG BBBB w  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                ], 250),
            ],
            &p,
            true,
        ),
    );

    // Thinking
    animations.insert(
        "thinking".to_string(),
        build_animation(
            "thinking",
            &[
                (&[
                    "       ???              ",
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBBo .oBBBB  G   ",
                    "  GGG BBBB n  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                ], 700),
                (&[
                    "              ???       ",
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBBo  oBBBB  G   ",
                    "  GGG BBBB n  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                ], 700),
            ],
            &p,
            true,
        ),
    );

    // Waiting/Playful - music notes, wiggle
    animations.insert(
        "waiting".to_string(),
        build_animation(
            "waiting",
            &[
                (&[
                    "     #      #           ",
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBBo  oBBBB  G   ",
                    "  GGG BBBB w  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L  WWW  ",
                ], 500),
                (&[
                    "          #      #      ",
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBBo  oBBBB  G   ",
                    "  GGG BBBB O  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    " WWW L  L    L  L       ",
                ], 500),
            ],
            &p,
            true,
        ),
    );

    // Joking - wink with personality
    animations.insert(
        "joking".to_string(),
        Animation {
            name: "joking".to_string(),
            frames: vec![build_frame(
                &[
                    "                   *    ",
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBB^  -BBBB  G   ",
                    "  GGG BBBB w  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                ],
                &p,
                1500,
            )],
            looping: false,
        },
    );

    // Error
    animations.insert(
        "error".to_string(),
        build_animation(
            "error",
            &[
                (&[
                    "    X        X          ",
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBBo  oBBBB  G   ",
                    "  GGG BBBB n  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                ], 600),
                (&[
                    "         X        X     ",
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBB.  .BBBB  G   ",
                    "  GGG BBBB n  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                ], 600),
            ],
            &p,
            true,
        ),
    );

    // Talking
    animations.insert(
        "talking".to_string(),
        build_animation(
            "talking",
            &[
                (&[
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBBo  oBBBB  G   ",
                    "  GGG BBBB O  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                    "    WWW          WWW    ",
                ], 200),
                (&[
                    "        /TTTTTT\\        ",
                    "      /TTTTTTTTTT\\      ",
                    "   G  BBBBo  oBBBB  G   ",
                    "  GGG BBBB w  BBBB GGG  ",
                    "   G  BBBBBBBBBBBB  G   ",
                    "      BBVVVVVVVVBB      ",
                    "     L  L    L  L       ",
                    "    WWW          WWW    ",
                ], 200),
            ],
            &p,
            true,
        ),
    );

    SpriteSheet {
        animations,
        default: "idle".to_string(),
    }
}
