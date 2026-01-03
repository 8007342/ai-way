//! Dirty-Rect Tracking for Partial Rendering
//!
//! This module implements partial rendering optimization (P2.5) to reduce
//! CPU usage during idle states. Instead of re-rendering the entire avatar
//! every frame, we track which cells have changed and only update those.
//!
//! # Performance Benefits
//!
//! - Breathing animation only touches ~5-10 cells per frame
//! - Idle animations have minimal cell changes between frames
//! - Mood transitions can be optimized by tracking affected regions
//!
//! # Usage
//!
//! ```ignore
//! let mut tracker = DirtyTracker::new(20, 15);
//!
//! // Mark regions that changed
//! tracker.mark_dirty(5, 3, 4, 2);
//!
//! // Get dirty rects for rendering
//! for rect in tracker.get_dirty_rects() {
//!     // Only re-render cells in this region
//!     render_region(rect);
//! }
//!
//! // Clear after rendering
//! tracker.clear_dirty();
//! ```

use std::collections::HashSet;

/// A rectangular region that needs re-rendering
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DirtyRect {
    /// X position (column)
    pub x: u16,
    /// Y position (row)
    pub y: u16,
    /// Width in cells
    pub width: u16,
    /// Height in cells
    pub height: u16,
}

impl DirtyRect {
    /// Create a new dirty rect
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create a single-cell dirty rect
    pub const fn cell(x: u16, y: u16) -> Self {
        Self::new(x, y, 1, 1)
    }

    /// Check if this rect contains a point
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Check if this rect intersects with another
    pub fn intersects(&self, other: &DirtyRect) -> bool {
        !(self.x + self.width <= other.x
            || other.x + other.width <= self.x
            || self.y + self.height <= other.y
            || other.y + other.height <= self.y)
    }

    /// Merge two rects into their bounding box
    pub fn merge(&self, other: &DirtyRect) -> DirtyRect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);

        DirtyRect {
            x,
            y,
            width: x2 - x,
            height: y2 - y,
        }
    }

    /// Get the area in cells
    pub fn area(&self) -> u32 {
        u32::from(self.width) * u32::from(self.height)
    }

    /// Iterate over all cells in this rect
    pub fn cells(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        (self.y..self.y + self.height)
            .flat_map(move |y| (self.x..self.x + self.width).map(move |x| (x, y)))
    }
}

/// Tracks which regions of the avatar need re-rendering
///
/// Uses a cell-based approach for fine-grained tracking, with
/// rect merging to optimize rendering passes.
#[derive(Debug)]
pub struct DirtyTracker {
    /// Set of dirty cells (x, y coordinates)
    dirty_cells: HashSet<(u16, u16)>,
    /// Total width of the tracked area
    width: u16,
    /// Total height of the tracked area
    height: u16,
    /// Whether the entire area is dirty (optimization for full redraws)
    full_dirty: bool,
    /// Cached dirty rects (invalidated on new marks)
    cached_rects: Option<Vec<DirtyRect>>,
}

impl DirtyTracker {
    /// Create a new dirty tracker for an area
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            dirty_cells: HashSet::new(),
            width,
            height,
            full_dirty: true, // Start with full redraw needed
            cached_rects: None,
        }
    }

    /// Mark a rectangular region as dirty
    pub fn mark_dirty(&mut self, x: u16, y: u16, width: u16, height: u16) {
        self.cached_rects = None;

        // If marking the full area, just set the flag
        if x == 0 && y == 0 && width >= self.width && height >= self.height {
            self.full_dirty = true;
            return;
        }

        // Mark individual cells
        for cy in y..y.saturating_add(height).min(self.height) {
            for cx in x..x.saturating_add(width).min(self.width) {
                self.dirty_cells.insert((cx, cy));
            }
        }
    }

    /// Mark a single cell as dirty
    pub fn mark_cell_dirty(&mut self, x: u16, y: u16) {
        if x < self.width && y < self.height {
            self.cached_rects = None;
            self.dirty_cells.insert((x, y));
        }
    }

    /// Mark the entire area as dirty
    pub fn mark_all_dirty(&mut self) {
        self.cached_rects = None;
        self.full_dirty = true;
    }

    /// Get optimized dirty rects for rendering
    ///
    /// This merges adjacent dirty cells into larger rects to reduce
    /// the number of rendering operations needed.
    pub fn get_dirty_rects(&mut self) -> Vec<DirtyRect> {
        // Return cached rects if available
        if let Some(ref rects) = self.cached_rects {
            return rects.clone();
        }

        // If full area is dirty, return single rect
        if self.full_dirty {
            let rects = vec![DirtyRect::new(0, 0, self.width, self.height)];
            self.cached_rects = Some(rects.clone());
            return rects;
        }

        // If no dirty cells, return empty
        if self.dirty_cells.is_empty() {
            self.cached_rects = Some(vec![]);
            return vec![];
        }

        // Build rects from dirty cells using row-based merging
        let rects = self.build_optimized_rects();
        self.cached_rects = Some(rects.clone());
        rects
    }

    /// Build optimized dirty rects from individual cells
    fn build_optimized_rects(&self) -> Vec<DirtyRect> {
        if self.dirty_cells.is_empty() {
            return vec![];
        }

        // Simple approach: find bounding box of all dirty cells
        // For small numbers of cells, this is efficient enough
        let mut min_x = u16::MAX;
        let mut min_y = u16::MAX;
        let mut max_x = 0u16;
        let mut max_y = 0u16;

        for &(x, y) in &self.dirty_cells {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }

        // If the bounding box is small enough or cell count is high,
        // just return the bounding box
        let bounding_area = (max_x - min_x + 1) as usize * (max_y - min_y + 1) as usize;
        let cell_count = self.dirty_cells.len();

        // If cells cover more than 50% of bounding box, use single rect
        if cell_count * 2 >= bounding_area || bounding_area <= 16 {
            return vec![DirtyRect::new(
                min_x,
                min_y,
                max_x - min_x + 1,
                max_y - min_y + 1,
            )];
        }

        // For sparse dirty regions, build row-based rects
        self.build_row_rects()
    }

    /// Build rects by scanning rows for contiguous dirty regions
    fn build_row_rects(&self) -> Vec<DirtyRect> {
        let mut rects = Vec::new();

        // Sort cells by y then x for row-based processing
        let mut sorted_cells: Vec<_> = self.dirty_cells.iter().copied().collect();
        sorted_cells.sort_by_key(|&(x, y)| (y, x));

        let mut current_row = u16::MAX;
        let mut run_start_x = 0;
        let mut run_end_x = 0;

        for (x, y) in sorted_cells {
            if y != current_row {
                // Finish previous run
                if current_row != u16::MAX {
                    rects.push(DirtyRect::new(
                        run_start_x,
                        current_row,
                        run_end_x - run_start_x + 1,
                        1,
                    ));
                }
                // Start new run
                current_row = y;
                run_start_x = x;
                run_end_x = x;
            } else if x == run_end_x + 1 {
                // Extend current run
                run_end_x = x;
            } else {
                // Gap in row, finish current run and start new
                rects.push(DirtyRect::new(
                    run_start_x,
                    current_row,
                    run_end_x - run_start_x + 1,
                    1,
                ));
                run_start_x = x;
                run_end_x = x;
            }
        }

        // Finish last run
        if current_row != u16::MAX {
            rects.push(DirtyRect::new(
                run_start_x,
                current_row,
                run_end_x - run_start_x + 1,
                1,
            ));
        }

        // Merge vertically adjacent rects with same x bounds
        self.merge_vertical_rects(rects)
    }

    /// Merge vertically adjacent rects with matching x bounds
    fn merge_vertical_rects(&self, mut rects: Vec<DirtyRect>) -> Vec<DirtyRect> {
        if rects.len() <= 1 {
            return rects;
        }

        // Sort by x, then y for vertical merging
        rects.sort_by_key(|r| (r.x, r.y));

        let mut merged = Vec::new();
        let mut current = rects[0];

        for rect in rects.into_iter().skip(1) {
            // Check if we can merge vertically
            if rect.x == current.x
                && rect.width == current.width
                && rect.y == current.y + current.height
            {
                // Extend current rect downward
                current.height += rect.height;
            } else {
                // Can't merge, push current and start new
                merged.push(current);
                current = rect;
            }
        }

        merged.push(current);
        merged
    }

    /// Clear all dirty tracking (call after rendering)
    pub fn clear_dirty(&mut self) {
        self.dirty_cells.clear();
        self.full_dirty = false;
        self.cached_rects = None;
    }

    /// Check if any region is dirty
    pub fn is_dirty(&self) -> bool {
        self.full_dirty || !self.dirty_cells.is_empty()
    }

    /// Check if the entire area is marked dirty
    pub fn is_full_dirty(&self) -> bool {
        self.full_dirty
    }

    /// Get the number of dirty cells
    pub fn dirty_cell_count(&self) -> usize {
        if self.full_dirty {
            usize::from(self.width) * usize::from(self.height)
        } else {
            self.dirty_cells.len()
        }
    }

    /// Resize the tracked area
    ///
    /// This marks the entire new area as dirty.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.mark_all_dirty();
    }

    /// Get the tracked area dimensions
    pub fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Check if a specific cell is dirty
    pub fn is_cell_dirty(&self, x: u16, y: u16) -> bool {
        self.full_dirty || self.dirty_cells.contains(&(x, y))
    }
}

impl Default for DirtyTracker {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Extension trait for integrating dirty tracking with animation
pub trait DirtyTrackingExt {
    /// Mark the region affected by a frame change
    fn mark_frame_dirty(
        &mut self,
        x_offset: u16,
        y_offset: u16,
        frame_width: u16,
        frame_height: u16,
    );

    /// Mark the region affected by a mood transition
    fn mark_transition_dirty(&mut self, blend_factor: f32);
}

impl DirtyTrackingExt for DirtyTracker {
    fn mark_frame_dirty(
        &mut self,
        x_offset: u16,
        y_offset: u16,
        frame_width: u16,
        frame_height: u16,
    ) {
        self.mark_dirty(x_offset, y_offset, frame_width, frame_height);
    }

    fn mark_transition_dirty(&mut self, _blend_factor: f32) {
        // During transitions, we need to redraw the entire avatar
        // as colors are blending between moods
        self.mark_all_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_rect_creation() {
        let rect = DirtyRect::new(5, 10, 3, 4);
        assert_eq!(rect.x, 5);
        assert_eq!(rect.y, 10);
        assert_eq!(rect.width, 3);
        assert_eq!(rect.height, 4);
    }

    #[test]
    fn test_dirty_rect_contains() {
        let rect = DirtyRect::new(5, 5, 5, 5);

        assert!(rect.contains(5, 5));
        assert!(rect.contains(7, 7));
        assert!(rect.contains(9, 9));
        assert!(!rect.contains(10, 10));
        assert!(!rect.contains(4, 5));
    }

    #[test]
    fn test_dirty_rect_intersects() {
        let rect1 = DirtyRect::new(0, 0, 5, 5);
        let rect2 = DirtyRect::new(3, 3, 5, 5);
        let rect3 = DirtyRect::new(10, 10, 2, 2);

        assert!(rect1.intersects(&rect2));
        assert!(!rect1.intersects(&rect3));
    }

    #[test]
    fn test_dirty_rect_merge() {
        let rect1 = DirtyRect::new(0, 0, 3, 3);
        let rect2 = DirtyRect::new(2, 2, 3, 3);
        let merged = rect1.merge(&rect2);

        assert_eq!(merged.x, 0);
        assert_eq!(merged.y, 0);
        assert_eq!(merged.width, 5);
        assert_eq!(merged.height, 5);
    }

    #[test]
    fn test_dirty_rect_area() {
        let rect = DirtyRect::new(0, 0, 4, 5);
        assert_eq!(rect.area(), 20);
    }

    #[test]
    fn test_dirty_rect_cells() {
        let rect = DirtyRect::new(0, 0, 2, 2);
        let cells: Vec<_> = rect.cells().collect();
        assert_eq!(cells.len(), 4);
        assert!(cells.contains(&(0, 0)));
        assert!(cells.contains(&(1, 0)));
        assert!(cells.contains(&(0, 1)));
        assert!(cells.contains(&(1, 1)));
    }

    #[test]
    fn test_tracker_new_starts_dirty() {
        let tracker = DirtyTracker::new(20, 15);
        assert!(tracker.is_dirty());
        assert!(tracker.is_full_dirty());
    }

    #[test]
    fn test_tracker_clear_dirty() {
        let mut tracker = DirtyTracker::new(20, 15);
        tracker.clear_dirty();
        assert!(!tracker.is_dirty());
        assert!(!tracker.is_full_dirty());
    }

    #[test]
    fn test_tracker_mark_dirty() {
        let mut tracker = DirtyTracker::new(20, 15);
        tracker.clear_dirty();

        tracker.mark_dirty(5, 5, 3, 2);
        assert!(tracker.is_dirty());
        assert!(!tracker.is_full_dirty());
        assert_eq!(tracker.dirty_cell_count(), 6);
    }

    #[test]
    fn test_tracker_mark_cell_dirty() {
        let mut tracker = DirtyTracker::new(20, 15);
        tracker.clear_dirty();

        tracker.mark_cell_dirty(5, 5);
        assert!(tracker.is_dirty());
        assert!(tracker.is_cell_dirty(5, 5));
        assert!(!tracker.is_cell_dirty(6, 5));
    }

    #[test]
    fn test_tracker_mark_all_dirty() {
        let mut tracker = DirtyTracker::new(20, 15);
        tracker.clear_dirty();
        tracker.mark_cell_dirty(5, 5);

        tracker.mark_all_dirty();
        assert!(tracker.is_full_dirty());
        assert_eq!(tracker.dirty_cell_count(), 300); // 20 * 15
    }

    #[test]
    fn test_tracker_get_dirty_rects_full() {
        let mut tracker = DirtyTracker::new(20, 15);
        let rects = tracker.get_dirty_rects();

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], DirtyRect::new(0, 0, 20, 15));
    }

    #[test]
    fn test_tracker_get_dirty_rects_empty() {
        let mut tracker = DirtyTracker::new(20, 15);
        tracker.clear_dirty();
        let rects = tracker.get_dirty_rects();

        assert!(rects.is_empty());
    }

    #[test]
    fn test_tracker_get_dirty_rects_small_region() {
        let mut tracker = DirtyTracker::new(20, 15);
        tracker.clear_dirty();
        tracker.mark_dirty(5, 5, 3, 2);

        let rects = tracker.get_dirty_rects();
        assert!(!rects.is_empty());

        // All dirty cells should be covered
        for y in 5..7 {
            for x in 5..8 {
                assert!(rects.iter().any(|r| r.contains(x, y)));
            }
        }
    }

    #[test]
    fn test_tracker_resize() {
        let mut tracker = DirtyTracker::new(10, 10);
        tracker.clear_dirty();

        tracker.resize(20, 15);
        assert!(tracker.is_full_dirty());
        assert_eq!(tracker.dimensions(), (20, 15));
    }

    #[test]
    fn test_tracker_clipping() {
        let mut tracker = DirtyTracker::new(10, 10);
        tracker.clear_dirty();

        // Mark region that extends past bounds
        tracker.mark_dirty(8, 8, 5, 5);

        // Should only mark cells within bounds
        assert!(tracker.is_cell_dirty(8, 8));
        assert!(tracker.is_cell_dirty(9, 9));
        assert!(!tracker.is_cell_dirty(10, 10)); // Out of bounds
    }

    #[test]
    fn test_dirty_tracking_ext() {
        let mut tracker = DirtyTracker::new(20, 15);
        tracker.clear_dirty();

        tracker.mark_frame_dirty(5, 3, 10, 8);
        assert!(tracker.is_dirty());
        assert!(tracker.is_cell_dirty(5, 3));
        assert!(tracker.is_cell_dirty(14, 10));
    }

    #[test]
    fn test_dirty_tracking_ext_transition() {
        let mut tracker = DirtyTracker::new(20, 15);
        tracker.clear_dirty();

        tracker.mark_transition_dirty(0.5);
        assert!(tracker.is_full_dirty());
    }

    #[test]
    fn test_rect_caching() {
        let mut tracker = DirtyTracker::new(20, 15);
        tracker.clear_dirty();
        tracker.mark_dirty(5, 5, 3, 3);

        // First call builds cache
        let rects1 = tracker.get_dirty_rects();
        // Second call uses cache
        let rects2 = tracker.get_dirty_rects();

        assert_eq!(rects1, rects2);

        // Marking dirty invalidates cache
        tracker.mark_cell_dirty(0, 0);
        let rects3 = tracker.get_dirty_rects();
        assert_ne!(rects1, rects3);
    }
}
