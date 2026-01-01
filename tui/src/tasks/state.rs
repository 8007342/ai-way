//! Task State Management
//!
//! Reads task state from the .state/tasks directory created by the shell routing module.

use std::fs;
use std::path::PathBuf;

/// Status of a background task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Failed,
    Unknown,
}

impl TaskStatus {
    fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "pending" => TaskStatus::Pending,
            "running" => TaskStatus::Running,
            "done" => TaskStatus::Done,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Unknown,
        }
    }

    /// Get a status icon
    pub fn icon(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "⏳",
            TaskStatus::Running => "🔄",
            TaskStatus::Done => "✅",
            TaskStatus::Failed => "❌",
            TaskStatus::Unknown => "❓",
        }
    }
}

/// A background specialist task
#[derive(Debug, Clone)]
pub struct BackgroundTask {
    pub id: String,
    pub agent: String,
    pub family_name: String,
    pub description: String,
    pub status: TaskStatus,
    pub progress: u8,
}

impl BackgroundTask {
    /// Get a short display name for the agent
    pub fn display_name(&self) -> &str {
        if !self.family_name.is_empty() {
            &self.family_name
        } else {
            &self.agent
        }
    }

    /// Get a progress bar string
    pub fn progress_bar(&self, width: usize) -> String {
        let filled = (self.progress as usize * width) / 100;
        let empty = width.saturating_sub(filled);

        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }
}

/// Manager for reading task state
pub struct TaskState {
    tasks_dir: PathBuf,
    cached_tasks: Vec<BackgroundTask>,
}

impl TaskState {
    /// Create a new task state manager
    pub fn new() -> Self {
        // Get the state directory from environment or default
        let state_dir = std::env::var("YOLLAYAH_STATE_DIR").unwrap_or_else(|_| {
            // Try to find relative to script dir
            std::env::var("SCRIPT_DIR")
                .map(|s| format!("{}/.state", s))
                .unwrap_or_else(|_| ".state".to_string())
        });

        Self {
            tasks_dir: PathBuf::from(state_dir).join("tasks"),
            cached_tasks: Vec::new(),
        }
    }

    /// Refresh task list from filesystem
    pub fn refresh(&mut self) {
        self.cached_tasks.clear();

        if !self.tasks_dir.exists() {
            return;
        }

        // Read all task directories
        if let Ok(entries) = fs::read_dir(&self.tasks_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(task) = self.read_task(&path) {
                        self.cached_tasks.push(task);
                    }
                }
            }
        }

        // Sort by status (running first, then pending, then done)
        self.cached_tasks.sort_by_key(|t| match t.status {
            TaskStatus::Running => 0,
            TaskStatus::Pending => 1,
            TaskStatus::Done => 2,
            TaskStatus::Failed => 3,
            TaskStatus::Unknown => 4,
        });
    }

    /// Read a single task from its directory
    fn read_task(&self, task_dir: &PathBuf) -> Option<BackgroundTask> {
        let id = task_dir.file_name()?.to_string_lossy().to_string();

        let agent = fs::read_to_string(task_dir.join("agent"))
            .ok()?
            .trim()
            .to_string();

        let status_str =
            fs::read_to_string(task_dir.join("status")).unwrap_or_else(|_| "unknown".to_string());

        let progress: u8 = fs::read_to_string(task_dir.join("progress"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);

        let description = fs::read_to_string(task_dir.join("description"))
            .unwrap_or_else(|_| "Working...".to_string())
            .trim()
            .to_string();

        // Get family name from agent ID
        let family_name = Self::agent_to_family_name(&agent);

        Some(BackgroundTask {
            id,
            agent,
            family_name,
            description,
            status: TaskStatus::from_str(&status_str),
            progress,
        })
    }

    /// Get list of active tasks (running or pending)
    pub fn active_tasks(&self) -> Vec<&BackgroundTask> {
        self.cached_tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Running | TaskStatus::Pending))
            .collect()
    }

    /// Get all tasks
    pub fn all_tasks(&self) -> &[BackgroundTask] {
        &self.cached_tasks
    }

    /// Check if there are any active tasks
    pub fn has_active_tasks(&self) -> bool {
        self.cached_tasks
            .iter()
            .any(|t| matches!(t.status, TaskStatus::Running | TaskStatus::Pending))
    }

    /// Get status of a specific task by ID
    pub fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        self.cached_tasks
            .iter()
            .find(|t| t.id == task_id)
            .map(|t| t.status)
    }

    /// Map agent ID to family name
    fn agent_to_family_name(agent_id: &str) -> String {
        match agent_id {
            "ethical-hacker" => "Cousin Rita".to_string(),
            "backend-engineer" => "Uncle Marco".to_string(),
            "frontend-specialist" => "Prima Sofia".to_string(),
            "senior-full-stack-developer" => "Tío Miguel".to_string(),
            "solutions-architect" => "Tía Carmen".to_string(),
            "ux-ui-designer" => "Cousin Lucia".to_string(),
            "qa-engineer" => "The Intern".to_string(),
            "privacy-researcher" => "Abuelo Pedro".to_string(),
            "devops-engineer" => "Primo Carlos".to_string(),
            "relational-database-expert" => "Tía Rosa".to_string(),
            _ => {
                // Capitalize first letter
                let mut chars = agent_id.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                    None => agent_id.to_string(),
                }
            }
        }
    }
}

impl Default for TaskState {
    fn default() -> Self {
        Self::new()
    }
}
