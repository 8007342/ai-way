//! Layered Compositor
//!
//! Manages z-ordered layers for rendering. Each layer has its own buffer
//! and can be positioned, resized, and reordered independently.
//!
//! The compositor composites all visible layers into a final output buffer.

mod layer;

use std::collections::HashMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

pub use layer::Layer;

/// Unique identifier for a layer
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayerId(u32);

/// The compositor manages all layers and composites them together
pub struct Compositor {
    /// All layers by ID
    layers: HashMap<LayerId, Layer>,
    /// Layers sorted by z-index for rendering
    render_order: Vec<LayerId>,
    /// Next layer ID to assign
    next_id: u32,
    /// Output buffer (composited result)
    output: Buffer,
    /// Total area
    area: Rect,
}

impl Compositor {
    /// Create a new compositor for the given area
    pub fn new(area: Rect) -> Self {
        Self {
            layers: HashMap::new(),
            render_order: Vec::new(),
            next_id: 0,
            output: Buffer::empty(area),
            area,
        }
    }

    /// Create a new layer and return its ID
    pub fn create_layer(&mut self, bounds: Rect, z_index: i32) -> LayerId {
        let id = LayerId(self.next_id);
        self.next_id += 1;

        let layer = Layer::new(id, bounds, z_index);
        self.layers.insert(id, layer);
        self.update_render_order();

        id
    }

    /// Get mutable access to a layer's buffer for rendering
    pub fn layer_buffer_mut(&mut self, id: LayerId) -> Option<&mut Buffer> {
        self.layers.get_mut(&id).map(|l| &mut l.buffer)
    }

    /// Set a layer's z-index (for avatar popping to front)
    pub fn set_z_index(&mut self, id: LayerId, z_index: i32) {
        if let Some(layer) = self.layers.get_mut(&id) {
            if layer.z_index != z_index {
                layer.z_index = z_index;
                self.update_render_order();
            }
        }
    }

    /// Move a layer to a new position
    pub fn move_layer(&mut self, id: LayerId, x: u16, y: u16) {
        if let Some(layer) = self.layers.get_mut(&id) {
            layer.bounds.x = x;
            layer.bounds.y = y;
        }
    }

    /// Resize a layer
    pub fn resize_layer(&mut self, id: LayerId, width: u16, height: u16) {
        if let Some(layer) = self.layers.get_mut(&id) {
            layer.bounds.width = width;
            layer.bounds.height = height;
            // Buffer uses origin coordinates
            layer.buffer = Buffer::empty(Rect::new(0, 0, width, height));
        }
    }

    /// Set layer visibility
    pub fn set_visible(&mut self, id: LayerId, visible: bool) {
        if let Some(layer) = self.layers.get_mut(&id) {
            layer.visible = visible;
        }
    }

    /// Resize the entire compositor
    pub fn resize(&mut self, area: Rect) {
        self.area = area;
        self.output = Buffer::empty(area);
    }

    /// Composite all visible layers into the output buffer
    pub fn composite(&mut self) -> &Buffer {
        // Clear output
        self.output.reset();

        // Render layers in z-order (back to front)
        for &id in &self.render_order.clone() {
            if let Some(layer) = self.layers.get(&id) {
                if layer.visible {
                    Self::blit_layer(&mut self.output, &self.area, layer);
                }
            }
        }

        &self.output
    }

    /// Blit a layer onto the output buffer (solid occlusion)
    fn blit_layer(output: &mut Buffer, area: &Rect, layer: &Layer) {
        let lb = &layer.bounds;

        for ly in 0..lb.height {
            for lx in 0..lb.width {
                let src_x = lx;
                let src_y = ly;
                let dst_x = lb.x + lx;
                let dst_y = lb.y + ly;

                // Bounds check
                if dst_x >= area.width || dst_y >= area.height {
                    continue;
                }

                let src_idx = layer.buffer.index_of(src_x, src_y);
                if src_idx >= layer.buffer.content.len() {
                    continue;
                }

                let src_cell = &layer.buffer.content[src_idx];

                // Solid occlusion: non-space cells overwrite
                // This allows transparent "holes" in layers
                if src_cell.symbol() != " " {
                    let dst_idx = output.index_of(dst_x, dst_y);
                    if dst_idx < output.content.len() {
                        output.content[dst_idx] = src_cell.clone();
                    }
                }
            }
        }
    }

    /// Find the topmost layer at a given position (for mouse events)
    pub fn layer_at(&self, x: u16, y: u16) -> Option<LayerId> {
        // Iterate in reverse render order (front to back)
        for &id in self.render_order.iter().rev() {
            if let Some(layer) = self.layers.get(&id) {
                if layer.visible && layer.contains(x, y) {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Update render order based on z-indices
    fn update_render_order(&mut self) {
        self.render_order = self.layers.keys().copied().collect();
        self.render_order
            .sort_by_key(|id| self.layers.get(id).map(|l| l.z_index).unwrap_or(0));
    }
}
