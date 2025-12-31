//! Task Panel Renderer
//!
//! Renders the task panel showing background specialist work.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use super::state::{TaskState, TaskStatus, BackgroundTask};

/// Task panel widget
pub struct TaskPanel {
    state: TaskState,
}

impl TaskPanel {
    /// Create a new task panel
    pub fn new() -> Self {
        Self {
            state: TaskState::new(),
        }
    }

    /// Refresh task state from filesystem
    pub fn refresh(&mut self) {
        self.state.refresh();
    }

    /// Check if panel should be visible (has active tasks)
    pub fn should_show(&self) -> bool {
        self.state.has_active_tasks()
    }

    /// Get active task count
    pub fn active_count(&self) -> usize {
        self.state.active_tasks().len()
    }

    /// Get status of a specific task by ID
    pub fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        self.state.get_task_status(task_id)
    }

    /// Render the task panel to a buffer
    pub fn render(&self, buf: &mut Buffer, area: Rect) {
        if area.width < 10 || area.height < 3 {
            return;
        }

        let active_tasks = self.state.active_tasks();
        if active_tasks.is_empty() {
            return;
        }

        // Draw panel background and border
        self.draw_border(buf, area);

        // Draw header
        let header = " Background ";
        let header_x = area.x + (area.width.saturating_sub(header.len() as u16)) / 2;
        buf.set_string(
            header_x,
            area.y,
            header,
            Style::default().fg(Color::Magenta),
        );

        // Draw each task
        let task_area = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );

        let mut y = task_area.y;
        for task in active_tasks.iter().take((task_area.height / 3) as usize) {
            self.draw_task(buf, task, task_area.x, y, task_area.width);
            y += 3;
            if y >= task_area.y + task_area.height {
                break;
            }
        }
    }

    /// Draw panel border
    fn draw_border(&self, buf: &mut Buffer, area: Rect) {
        let border_style = Style::default().fg(Color::DarkGray);

        // Top border
        let top = format!(
            "╭{}╮",
            "─".repeat(area.width.saturating_sub(2) as usize)
        );
        buf.set_string(area.x, area.y, &top, border_style);

        // Side borders
        for y in (area.y + 1)..(area.y + area.height.saturating_sub(1)) {
            buf.set_string(area.x, y, "│", border_style);
            buf.set_string(area.x + area.width.saturating_sub(1), y, "│", border_style);
        }

        // Bottom border
        let bottom = format!(
            "╰{}╯",
            "─".repeat(area.width.saturating_sub(2) as usize)
        );
        buf.set_string(area.x, area.y + area.height.saturating_sub(1), &bottom, border_style);
    }

    /// Draw a single task
    fn draw_task(&self, buf: &mut Buffer, task: &BackgroundTask, x: u16, y: u16, width: u16) {
        let inner_width = width.saturating_sub(2) as usize;

        // Line 1: Status icon + family name
        let name_line = format!(
            "{} {}",
            task.status.icon(),
            task.display_name()
        );
        let name_style = match task.status {
            TaskStatus::Running => Style::default().fg(Color::Cyan),
            TaskStatus::Done => Style::default().fg(Color::Green),
            TaskStatus::Failed => Style::default().fg(Color::Red),
            _ => Style::default().fg(Color::Yellow),
        };
        buf.set_string(x, y, &name_line.chars().take(inner_width).collect::<String>(), name_style);

        // Line 2: Progress bar
        let bar_width = inner_width.saturating_sub(6);
        let progress_bar = task.progress_bar(bar_width);
        let progress_str = format!("{} {:>3}%", progress_bar, task.progress);
        buf.set_string(x, y + 1, &progress_str, Style::default().fg(Color::DarkGray));

        // Line 3: Description (truncated)
        let desc: String = task.description.chars().take(inner_width).collect();
        buf.set_string(x, y + 2, &desc, Style::default().fg(Color::Gray));
    }
}

impl Default for TaskPanel {
    fn default() -> Self {
        Self::new()
    }
}
