//! Task Management Types
//!
//! Types for managing background tasks (specialist agents working in parallel).
//! This module defines the data structures; the Conductor handles orchestration.
//!
//! # Design Philosophy
//!
//! Yollayah can delegate work to specialist agents ("family members") who work
//! in the background. Tasks track this work and let the UI show progress.
//! The Conductor owns task state; UI surfaces just render what they're told.

use serde::{Deserialize, Serialize};

/// Task identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

impl TaskId {
    /// Create a new task ID from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique task ID
    pub fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        Self(format!("task_{timestamp}_{count}"))
    }

    /// Get the string value
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a background task
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task created but not started
    Pending,
    /// Task is actively running
    Running,
    /// Task completed successfully
    Done,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
}

impl TaskStatus {
    /// Parse status from a string (for filesystem state)
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "done" | "complete" | "completed" => Self::Done,
            "failed" | "error" => Self::Failed,
            "cancelled" | "canceled" => Self::Cancelled,
            _ => Self::Pending,
        }
    }

    /// Get a status icon (for UI display)
    #[must_use]
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "...",
            Self::Running => ">>>",
            Self::Done => "[+]",
            Self::Failed => "[!]",
            Self::Cancelled => "[x]",
        }
    }

    /// Unicode icon variant
    #[must_use]
    pub fn icon_unicode(&self) -> &'static str {
        match self {
            Self::Pending => "\u{23f3}",   // hourglass
            Self::Running => "\u{1f504}",  // counterclockwise arrows
            Self::Done => "\u{2705}",      // check mark
            Self::Failed => "\u{274c}",    // cross mark
            Self::Cancelled => "\u{26d4}", // no entry
        }
    }

    /// Human-readable label
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Done => "Done",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    /// Whether this status indicates the task is complete (done, failed, or cancelled)
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }

    /// Whether this status indicates the task is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A background task
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,
    /// Agent handling this task
    pub agent: String,
    /// Human-readable agent name (e.g., "Cousin Rita")
    pub agent_display_name: String,
    /// Task description
    pub description: String,
    /// Current status
    pub status: TaskStatus,
    /// Progress percentage (0-100)
    pub progress: u8,
    /// Status message (optional)
    pub status_message: Option<String>,
    /// Result output (when complete)
    pub output: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// When the task was created (Unix timestamp ms)
    pub created_at: u64,
    /// When the task was last updated (Unix timestamp ms)
    pub updated_at: u64,
}

impl Task {
    /// Create a new pending task
    #[must_use]
    pub fn new(id: TaskId, agent: String, description: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            id,
            agent_display_name: agent_to_family_name(&agent),
            agent,
            description,
            status: TaskStatus::Pending,
            progress: 0,
            status_message: None,
            output: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update task status
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.touch();
    }

    /// Update task progress
    pub fn set_progress(&mut self, progress: u8, message: Option<String>) {
        self.progress = progress.min(100);
        self.status_message = message;
        if progress > 0 && self.status == TaskStatus::Pending {
            self.status = TaskStatus::Running;
        }
        self.touch();
    }

    /// Mark task as completed
    pub fn complete(&mut self, output: Option<String>) {
        self.status = TaskStatus::Done;
        self.progress = 100;
        self.output = output;
        self.touch();
    }

    /// Mark task as failed
    pub fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.error = Some(error);
        self.touch();
    }

    /// Update the `updated_at` timestamp
    fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }

    /// Generate a progress bar string
    #[must_use]
    pub fn progress_bar(&self, width: usize) -> String {
        let filled = (self.progress as usize * width) / 100;
        let empty = width.saturating_sub(filled);

        format!("{}{}", "#".repeat(filled), "-".repeat(empty))
    }

    /// Unicode progress bar
    #[must_use]
    pub fn progress_bar_unicode(&self, width: usize) -> String {
        let filled = (self.progress as usize * width) / 100;
        let empty = width.saturating_sub(filled);

        format!(
            "{}{}",
            "\u{2588}".repeat(filled), // full block
            "\u{2591}".repeat(empty)   // light shade
        )
    }
}

/// Map agent ID to family name
///
/// Yollayah's specialist agents are her "family members" with distinct personalities.
#[must_use]
pub fn agent_to_family_name(agent_id: &str) -> String {
    match agent_id {
        "ethical-hacker" => "Cousin Rita".to_string(),
        "backend-engineer" => "Uncle Marco".to_string(),
        "frontend-specialist" => "Prima Sofia".to_string(),
        "senior-full-stack-developer" => "Tio Miguel".to_string(),
        "solutions-architect" => "Tia Carmen".to_string(),
        "ux-ui-designer" => "Cousin Lucia".to_string(),
        "qa-engineer" => "The Intern".to_string(),
        "privacy-researcher" => "Abuelo Pedro".to_string(),
        "devops-engineer" => "Primo Carlos".to_string(),
        "relational-database-expert" => "Tia Rosa".to_string(),
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

/// Task manager state
///
/// Tracks all background tasks for the Conductor.
#[derive(Clone, Debug, Default)]
pub struct TaskManager {
    /// All tasks, keyed by ID
    tasks: std::collections::HashMap<TaskId, Task>,
    /// Order of task creation for consistent display
    task_order: Vec<TaskId>,
    /// Maximum number of active tasks (0 = unlimited)
    max_active_tasks: usize,
    /// Maximum total tasks (0 = unlimited)
    max_total_tasks: usize,
    /// Age after which completed tasks are cleaned up (0 = never)
    task_cleanup_age_ms: u64,
}

/// Error when task creation fails
#[derive(Clone, Debug)]
pub enum TaskCreationError {
    /// Too many active tasks
    TooManyActiveTasks {
        /// The configured limit
        limit: usize,
        /// Current active task count
        current: usize,
    },
    /// Too many total tasks
    TooManyTotalTasks {
        /// The configured limit
        limit: usize,
        /// Current total task count
        current: usize,
    },
}

impl std::fmt::Display for TaskCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyActiveTasks { limit, current } => {
                write!(f, "Too many active tasks: {current} (limit: {limit})")
            }
            Self::TooManyTotalTasks { limit, current } => {
                write!(f, "Too many total tasks: {current} (limit: {limit})")
            }
        }
    }
}

impl std::error::Error for TaskCreationError {}

impl TaskManager {
    /// Create a new task manager
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new task manager with limits
    #[must_use]
    pub fn new_with_limits(
        max_active_tasks: usize,
        max_total_tasks: usize,
        task_cleanup_age_ms: u64,
    ) -> Self {
        Self {
            tasks: std::collections::HashMap::new(),
            task_order: Vec::new(),
            max_active_tasks,
            max_total_tasks,
            task_cleanup_age_ms,
        }
    }

    /// Add a new task
    ///
    /// Note: This method does not check limits. Use `try_add_task` for limit checking.
    pub fn add_task(&mut self, task: Task) {
        // Auto-cleanup old tasks if configured
        if self.task_cleanup_age_ms > 0 {
            self.cleanup_old_tasks(self.task_cleanup_age_ms);
        }

        let id = task.id.clone();
        self.tasks.insert(id.clone(), task);
        self.task_order.push(id);
    }

    /// Try to add a new task, checking limits first
    pub fn try_add_task(&mut self, task: Task) -> Result<(), TaskCreationError> {
        self.check_limits()?;
        self.add_task(task);
        Ok(())
    }

    /// Check if adding a task would exceed limits
    fn check_limits(&self) -> Result<(), TaskCreationError> {
        // Check active task limit
        if self.max_active_tasks > 0 {
            let active = self.active_count();
            if active >= self.max_active_tasks {
                return Err(TaskCreationError::TooManyActiveTasks {
                    limit: self.max_active_tasks,
                    current: active,
                });
            }
        }

        // Check total task limit
        if self.max_total_tasks > 0 {
            let total = self.total_count();
            if total >= self.max_total_tasks {
                return Err(TaskCreationError::TooManyTotalTasks {
                    limit: self.max_total_tasks,
                    current: total,
                });
            }
        }

        Ok(())
    }

    /// Create and add a new task
    ///
    /// This is the legacy method that doesn't return errors.
    /// For limit-aware creation, use `try_create_task`.
    pub fn create_task(&mut self, agent: String, description: String) -> TaskId {
        // Auto-cleanup old tasks if configured
        if self.task_cleanup_age_ms > 0 {
            self.cleanup_old_tasks(self.task_cleanup_age_ms);
        }

        let id = TaskId::generate();
        let task = Task::new(id.clone(), agent, description);
        self.add_task(task);
        id
    }

    /// Try to create a new task, checking limits first
    pub fn try_create_task(
        &mut self,
        agent: String,
        description: String,
    ) -> Result<TaskId, TaskCreationError> {
        // Auto-cleanup old tasks if configured
        if self.task_cleanup_age_ms > 0 {
            self.cleanup_old_tasks(self.task_cleanup_age_ms);
        }

        self.check_limits()?;

        let id = TaskId::generate();
        let task = Task::new(id.clone(), agent, description);
        self.add_task(task);
        Ok(id)
    }

    /// Get a task by ID
    #[must_use]
    pub fn get(&self, id: &TaskId) -> Option<&Task> {
        self.tasks.get(id)
    }

    /// Get a mutable task by ID
    pub fn get_mut(&mut self, id: &TaskId) -> Option<&mut Task> {
        self.tasks.get_mut(id)
    }

    /// Update task progress
    pub fn update_progress(&mut self, id: &TaskId, progress: u8, message: Option<String>) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.set_progress(progress, message);
        }
    }

    /// Mark task as completed
    pub fn complete_task(&mut self, id: &TaskId, output: Option<String>) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.complete(output);
        }
    }

    /// Mark task as failed
    pub fn fail_task(&mut self, id: &TaskId, error: String) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.fail(error);
        }
    }

    /// Get all tasks in creation order
    pub fn all_tasks(&self) -> impl Iterator<Item = &Task> {
        self.task_order.iter().filter_map(|id| self.tasks.get(id))
    }

    /// Get active tasks (pending or running)
    pub fn active_tasks(&self) -> impl Iterator<Item = &Task> {
        self.all_tasks().filter(|t| t.status.is_active())
    }

    /// Get completed tasks (done, failed, or cancelled)
    pub fn completed_tasks(&self) -> impl Iterator<Item = &Task> {
        self.all_tasks().filter(|t| t.status.is_terminal())
    }

    /// Check if there are any active tasks
    #[must_use]
    pub fn has_active_tasks(&self) -> bool {
        self.tasks.values().any(|t| t.status.is_active())
    }

    /// Get count of active tasks
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.tasks.values().filter(|t| t.status.is_active()).count()
    }

    /// Get total task count
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.tasks.len()
    }

    /// Remove completed tasks older than the given age
    pub fn cleanup_old_tasks(&mut self, max_age_ms: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let cutoff = now.saturating_sub(max_age_ms);

        // Find IDs of old completed tasks
        let old_ids: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|(_, t)| t.status.is_terminal() && t.updated_at < cutoff)
            .map(|(id, _)| id.clone())
            .collect();

        // Remove them
        for id in &old_ids {
            self.tasks.remove(id);
        }

        // Update order list
        self.task_order.retain(|id| !old_ids.contains(id));
    }

    /// Clear all tasks
    pub fn clear(&mut self) {
        self.tasks.clear();
        self.task_order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_generate() {
        let id1 = TaskId::generate();
        let id2 = TaskId::generate();
        assert_ne!(id1, id2);
        assert!(id1.0.starts_with("task_"));
    }

    #[test]
    fn test_task_status_parse() {
        assert_eq!(TaskStatus::parse("running"), TaskStatus::Running);
        assert_eq!(TaskStatus::parse("DONE"), TaskStatus::Done);
        assert_eq!(TaskStatus::parse("  failed  "), TaskStatus::Failed);
        assert_eq!(TaskStatus::parse("unknown"), TaskStatus::Pending);
    }

    #[test]
    fn test_task_progress() {
        let mut task = Task::new(
            TaskId::new("test"),
            "test-agent".to_string(),
            "Test task".to_string(),
        );

        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.progress, 0);

        task.set_progress(50, Some("Halfway".to_string()));
        assert_eq!(task.status, TaskStatus::Running);
        assert_eq!(task.progress, 50);
        assert_eq!(task.status_message, Some("Halfway".to_string()));

        task.complete(Some("Result".to_string()));
        assert_eq!(task.status, TaskStatus::Done);
        assert_eq!(task.progress, 100);
        assert_eq!(task.output, Some("Result".to_string()));
    }

    #[test]
    fn test_task_manager() {
        let mut manager = TaskManager::new();

        let id1 = manager.create_task("agent1".to_string(), "Task 1".to_string());
        let id2 = manager.create_task("agent2".to_string(), "Task 2".to_string());

        assert_eq!(manager.total_count(), 2);
        assert_eq!(manager.active_count(), 2);

        manager.update_progress(&id1, 50, None);
        manager.complete_task(&id2, None);

        assert_eq!(manager.active_count(), 1);
        assert!(manager.get(&id1).unwrap().status.is_active());
        assert!(manager.get(&id2).unwrap().status.is_terminal());
    }

    #[test]
    fn test_agent_to_family_name() {
        assert_eq!(agent_to_family_name("ethical-hacker"), "Cousin Rita");
        assert_eq!(agent_to_family_name("backend-engineer"), "Uncle Marco");
        assert_eq!(agent_to_family_name("unknown-agent"), "Unknown-agent");
    }

    #[test]
    fn test_task_manager_with_limits() {
        let mut manager = TaskManager::new_with_limits(2, 5, 0);

        // Can create tasks up to active limit
        assert!(manager
            .try_create_task("agent1".to_string(), "Task 1".to_string())
            .is_ok());
        assert!(manager
            .try_create_task("agent2".to_string(), "Task 2".to_string())
            .is_ok());

        // Third active task should fail
        let result = manager.try_create_task("agent3".to_string(), "Task 3".to_string());
        assert!(matches!(
            result,
            Err(TaskCreationError::TooManyActiveTasks { .. })
        ));

        // Complete one task
        for task in manager.tasks.values_mut() {
            if task.description == "Task 1" {
                task.complete(None);
                break;
            }
        }

        // Now we can create another
        assert!(manager
            .try_create_task("agent3".to_string(), "Task 3".to_string())
            .is_ok());
    }

    #[test]
    fn test_task_manager_total_limit() {
        let mut manager = TaskManager::new_with_limits(10, 3, 0);

        // Can create up to total limit
        assert!(manager
            .try_create_task("agent1".to_string(), "Task 1".to_string())
            .is_ok());
        assert!(manager
            .try_create_task("agent2".to_string(), "Task 2".to_string())
            .is_ok());
        assert!(manager
            .try_create_task("agent3".to_string(), "Task 3".to_string())
            .is_ok());

        // Fourth task should fail due to total limit
        let result = manager.try_create_task("agent4".to_string(), "Task 4".to_string());
        assert!(matches!(
            result,
            Err(TaskCreationError::TooManyTotalTasks { .. })
        ));
    }

    #[test]
    fn test_task_manager_no_limits() {
        let mut manager = TaskManager::new();

        // Should be able to create many tasks without limits
        for i in 0..100 {
            assert!(manager
                .try_create_task(format!("agent{}", i), format!("Task {}", i))
                .is_ok());
        }

        assert_eq!(manager.total_count(), 100);
    }
}
