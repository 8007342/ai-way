//! Block Text Icon Library
//!
//! Minimalist icons using Unicode block drawing characters.
//! These work in most terminal fonts and avoid bright emojis.
//!
//! # Design Philosophy
//!
//! - Use box drawing and block elements (U+2500-U+257F, U+2580-U+259F)
//! - Geometric shapes (U+25A0-U+25FF) for simple indicators
//! - Triangular forms for arrows and direction
//! - Consistent visual weight - not too heavy, not too light
//!
//! # Character Reference
//!
//! Blocks: ▀ ▁ ▂ ▃ ▄ ▅ ▆ ▇ █ ▉ ▊ ▋ ▌ ▍ ▎ ▏ ▐ ░ ▒ ▓
//! Triangles: ▲ △ ▴ ▵ ▶ ▷ ▸ ▹ ► ▻ ▼ ▽ ▾ ▿ ◀ ◁ ◂ ◃ ◄ ◅
//! Squares: ■ □ ▢ ▣ ▤ ▥ ▦ ▧ ▨ ▩ ◆ ◇ ◈ ◊
//! Circles: ● ○ ◐ ◑ ◒ ◓ ◔ ◕ ◖ ◗ ◉ ◌ ◍ ◎
//! Lines: ─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼ ╭ ╮ ╯ ╰

/// Speed/performance indicators
pub mod speed {
    /// Very fast (>50 tok/s) - double arrow
    pub const FAST: &str = "▸▸";
    /// Normal speed (20-50 tok/s) - single arrow
    pub const NORMAL: &str = "▸";
    /// Slow (<20 tok/s) - no indicator (just show number)
    pub const SLOW: &str = "";
    /// Streaming/active
    pub const STREAMING: &str = "▹";
}

/// Status indicators
pub mod status {
    /// Ready/idle
    pub const READY: &str = "◇";
    /// Active/working
    pub const ACTIVE: &str = "◆";
    /// Thinking/processing
    pub const THINKING: &str = "◈";
    /// Success/done
    pub const SUCCESS: &str = "▣";
    /// Error/failed
    pub const ERROR: &str = "▢";
    /// Warning
    pub const WARNING: &str = "△";
    /// Info
    pub const INFO: &str = "○";
}

/// Progress indicators
pub mod progress {
    /// Empty progress block
    pub const EMPTY: &str = "░";
    /// Partial progress block
    pub const PARTIAL: &str = "▒";
    /// Full progress block
    pub const FULL: &str = "▓";
    /// Progress bar left cap
    pub const BAR_LEFT: &str = "▐";
    /// Progress bar right cap
    pub const BAR_RIGHT: &str = "▌";
}

/// Navigation/direction
pub mod nav {
    /// Up arrow
    pub const UP: &str = "▴";
    /// Down arrow
    pub const DOWN: &str = "▾";
    /// Left arrow
    pub const LEFT: &str = "◂";
    /// Right arrow
    pub const RIGHT: &str = "▸";
    /// Enter/submit
    pub const ENTER: &str = "▶";
    /// Back/escape
    pub const BACK: &str = "◀";
}

/// Task/agent indicators
pub mod task {
    /// Task pending
    pub const PENDING: &str = "···";
    /// Task running
    pub const RUNNING: &str = "▸▸▸";
    /// Task done
    pub const DONE: &str = "[▣]";
    /// Task failed
    pub const FAILED: &str = "[△]";
    /// Agent working
    pub const AGENT: &str = "◆";
}

/// Decorative separators
pub mod sep {
    /// Dot separator
    pub const DOT: &str = "·";
    /// Diamond separator
    pub const DIAMOND: &str = "◇";
    /// Line separator
    pub const LINE: &str = "│";
    /// Dash separator
    pub const DASH: &str = "─";
}

/// Cursor indicators
pub mod cursor {
    /// Block cursor
    pub const BLOCK: &str = "█";
    /// Underscore cursor
    pub const UNDERSCORE: &str = "▁";
    /// Line cursor
    pub const LINE: &str = "│";
    /// Insert mode
    pub const INSERT: &str = "▏";
}

/// Build a simple progress bar using block characters
pub fn progress_bar(progress: u8, width: usize) -> String {
    let filled = (progress as usize * width) / 100;
    let empty = width.saturating_sub(filled);

    format!(
        "{}{}",
        progress::FULL.repeat(filled),
        progress::EMPTY.repeat(empty)
    )
}

/// Build a progress bar with percentage
pub fn progress_bar_with_percent(progress: u8, width: usize) -> String {
    let bar = progress_bar(progress, width);
    format!("{} {:>3}%", bar, progress)
}

/// Get appropriate speed indicator for tokens per second
pub fn speed_indicator(tps: f32) -> &'static str {
    if tps > 50.0 {
        speed::FAST
    } else if tps > 20.0 {
        speed::NORMAL
    } else {
        speed::SLOW
    }
}

/// Format tokens per second with appropriate indicator
pub fn format_tps(tps: f32) -> String {
    let indicator = speed_indicator(tps);
    if indicator.is_empty() {
        format!("{:.1} tok/s", tps)
    } else {
        format!("{} {:.0} tok/s", indicator, tps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(0, 10).chars().count(), 10);
        assert_eq!(progress_bar(50, 10).chars().count(), 10);
        assert_eq!(progress_bar(100, 10).chars().count(), 10);
    }

    #[test]
    fn test_speed_indicator() {
        assert_eq!(speed_indicator(60.0), speed::FAST);
        assert_eq!(speed_indicator(30.0), speed::NORMAL);
        assert_eq!(speed_indicator(10.0), speed::SLOW);
    }

    #[test]
    fn test_format_tps() {
        let fast = format_tps(60.0);
        assert!(fast.contains("▸▸"));

        let normal = format_tps(30.0);
        assert!(normal.contains("▸") && !normal.contains("▸▸"));

        let slow = format_tps(10.0);
        assert!(slow.contains("10.0"));
    }
}
