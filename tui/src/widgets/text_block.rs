//! TextBlock Widget
//!
//! A borderless, scrollable text region.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::StatefulWidget;
use textwrap::wrap;

/// State for a scrollable text block
#[derive(Default)]
pub struct TextBlockState {
    /// Scroll offset (lines from top)
    pub scroll_offset: usize,
    /// Total content lines
    pub total_lines: usize,
}

impl TextBlockState {
    /// Scroll by delta (positive = down)
    pub fn scroll(&mut self, delta: i32) {
        let new_offset = self.scroll_offset as i32 + delta;
        self.scroll_offset = new_offset.max(0) as usize;
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.total_lines.saturating_sub(1);
    }
}

/// A borderless, scrollable text block
pub struct TextBlock<'a> {
    content: &'a str,
    style: Style,
}

impl<'a> TextBlock<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> StatefulWidget for TextBlock<'a> {
    type State = TextBlockState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Wrap text to width
        let wrapped: Vec<String> = self
            .content
            .lines()
            .flat_map(|line| {
                if line.is_empty() {
                    vec![String::new()]
                } else {
                    wrap(line, area.width as usize)
                        .into_iter()
                        .map(|cow| cow.to_string())
                        .collect()
                }
            })
            .collect();

        state.total_lines = wrapped.len();

        // Clamp scroll
        let max_scroll = state.total_lines.saturating_sub(area.height as usize);
        state.scroll_offset = state.scroll_offset.min(max_scroll);

        // Render visible lines
        for (i, line) in wrapped
            .iter()
            .skip(state.scroll_offset)
            .take(area.height as usize)
            .enumerate()
        {
            let y = area.y + i as u16;
            buf.set_string(area.x, y, line, self.style);
        }
    }
}
