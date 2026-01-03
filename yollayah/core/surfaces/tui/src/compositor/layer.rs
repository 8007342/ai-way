//! Layer - A single compositable layer

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::LayerId;

/// A single layer in the compositor
pub struct Layer {
    /// Unique identifier
    pub id: LayerId,
    /// Z-order (higher = in front)
    pub z_index: i32,
    /// Position and size
    pub bounds: Rect,
    /// Whether the layer is visible
    pub visible: bool,
    /// The layer's render buffer
    pub buffer: Buffer,
    /// Future: opacity for true transparency (0.0 = invisible, 1.0 = solid)
    pub opacity: f32,
}

impl Layer {
    /// Create a new layer
    pub fn new(id: LayerId, bounds: Rect, z_index: i32) -> Self {
        // Buffer uses origin coordinates (0,0) internally
        // The bounds store the screen position for compositing
        let buffer_area = Rect::new(0, 0, bounds.width, bounds.height);
        Self {
            id,
            z_index,
            bounds,
            visible: true,
            buffer: Buffer::empty(buffer_area),
            opacity: 1.0,
        }
    }

    /// Check if a point is within this layer's bounds
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.bounds.x
            && x < self.bounds.x + self.bounds.width
            && y >= self.bounds.y
            && y < self.bounds.y + self.bounds.height
    }
}
