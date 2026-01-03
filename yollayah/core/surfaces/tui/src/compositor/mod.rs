//! Layered Compositor
//!
//! Manages z-ordered layers for rendering. Each layer has its own buffer
//! and can be positioned, resized, and reordered independently.
//!
//! The compositor composites all visible layers into a final output buffer.

mod layer;

use std::collections::{HashMap, HashSet};

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
    /// Track which layers have changed since last composite
    /// When non-empty, composite() will rebuild the output buffer
    dirty_layers: HashSet<LayerId>,
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
            dirty_layers: HashSet::new(),
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
    /// Note: Caller should mark the layer dirty after modifying the buffer
    pub fn layer_buffer_mut(&mut self, id: LayerId) -> Option<&mut Buffer> {
        self.layers.get_mut(&id).map(|l| &mut l.buffer)
    }

    /// Set a layer's z-index (for avatar popping to front)
    pub fn set_z_index(&mut self, id: LayerId, z_index: i32) {
        if let Some(layer) = self.layers.get_mut(&id) {
            if layer.z_index != z_index {
                layer.z_index = z_index;
                self.update_render_order();
                // Z-index change affects visual output, mark dirty
                self.mark_layer_dirty(id);
            }
        }
    }

    /// Move a layer to a new position
    pub fn move_layer(&mut self, id: LayerId, x: u16, y: u16) {
        if let Some(layer) = self.layers.get_mut(&id) {
            if layer.bounds.x != x || layer.bounds.y != y {
                layer.bounds.x = x;
                layer.bounds.y = y;
                // Position change affects visual output, mark dirty
                self.mark_layer_dirty(id);
            }
        }
    }

    /// Resize a layer
    pub fn resize_layer(&mut self, id: LayerId, width: u16, height: u16) {
        if let Some(layer) = self.layers.get_mut(&id) {
            if layer.bounds.width != width || layer.bounds.height != height {
                layer.bounds.width = width;
                layer.bounds.height = height;
                // Buffer uses origin coordinates
                layer.buffer = Buffer::empty(Rect::new(0, 0, width, height));
                // Size change affects visual output, mark dirty
                self.mark_layer_dirty(id);
            }
        }
    }

    /// Set layer visibility
    pub fn set_visible(&mut self, id: LayerId, visible: bool) {
        if let Some(layer) = self.layers.get_mut(&id) {
            if layer.visible != visible {
                layer.visible = visible;
                // Visibility change affects visual output, mark dirty
                self.mark_layer_dirty(id);
            }
        }
    }

    /// Resize the entire compositor
    pub fn resize(&mut self, area: Rect) {
        self.area = area;
        self.output = Buffer::empty(area);
        // Mark all layers dirty since the output buffer changed
        let layer_ids: Vec<LayerId> = self.layers.keys().copied().collect();
        for id in layer_ids {
            self.mark_layer_dirty(id);
        }
    }

    /// Mark a layer as dirty (needs re-compositing)
    /// This should be called whenever a layer's content changes
    pub fn mark_layer_dirty(&mut self, id: LayerId) {
        self.dirty_layers.insert(id);
    }

    /// Check if any layers are dirty
    /// Returns true if composite() needs to rebuild the output buffer
    pub fn is_dirty(&self) -> bool {
        !self.dirty_layers.is_empty()
    }

    /// Clear the dirty tracking state
    /// Called internally after composite() rebuilds the output
    fn clear_dirty(&mut self) {
        self.dirty_layers.clear();
    }

    /// Composite all visible layers into the output buffer
    ///
    /// This uses dirty tracking to avoid unnecessary work:
    /// - If no layers are dirty, returns the cached output buffer
    /// - If any layer is dirty, clears and re-composites all visible layers
    ///
    /// Note: We re-composite ALL layers when ANY layer is dirty to maintain
    /// correct z-ordering and layer interactions. Future optimization could
    /// implement partial updates for non-overlapping dirty regions.
    pub fn composite(&mut self) -> &Buffer {
        // Early return: nothing changed, use cached output
        if self.dirty_layers.is_empty() {
            return &self.output;
        }

        // At least one layer changed - rebuild entire composite
        // Clear output buffer
        self.output.reset();

        // Render all visible layers in z-order (back to front)
        // We render ALL layers (not just dirty ones) to maintain correct layering
        for &id in &self.render_order {
            if let Some(layer) = self.layers.get(&id) {
                if layer.visible {
                    Self::blit_layer(&mut self.output, &self.area, layer);
                }
            }
        }

        // Clear dirty state after successful composite
        self.clear_dirty();

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
