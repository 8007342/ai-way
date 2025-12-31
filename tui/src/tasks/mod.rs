//! Task Panel Module
//!
//! Displays background specialist tasks in a panel on the right side of the TUI.
//! Shows task progress, status, and which family member is working on it.

mod state;
mod renderer;

pub use state::{TaskState, TaskStatus, BackgroundTask};
pub use renderer::TaskPanel;
