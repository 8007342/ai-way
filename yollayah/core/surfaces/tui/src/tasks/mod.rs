//! Task Panel Module
//!
//! Displays background specialist tasks in a panel on the right side of the TUI.
//! Shows task progress, status, and which family member is working on it.

mod renderer;
mod state;

pub use renderer::TaskPanel;
pub use state::{BackgroundTask, TaskState, TaskStatus};
