//! Security Module
//!
//! This module provides security validation for the Conductor to prevent:
//! - LLM command injection attacks
//! - Task agent injection
//! - Input validation bypass
//! - Resource exhaustion through unbounded data
//!
//! # Design Philosophy
//!
//! Security is enforced at the boundaries where untrusted input enters the system:
//! - User input from surfaces
//! - LLM-generated commands
//! - Task creation requests
//!
//! All validation is fail-safe: when in doubt, reject the input.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::avatar::AvatarCommand;

/// Configuration limits for the Conductor
///
/// These limits prevent resource exhaustion attacks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConductorLimits {
    /// Maximum size of a single message in bytes (default: 100KB)
    pub max_message_size: usize,
    /// Maximum messages per minute from a surface (default: 30)
    pub max_messages_per_minute: u32,
    /// Maximum command arguments (default: 10)
    pub max_command_args: usize,
    /// Maximum messages to keep in session history (default: 1000)
    pub max_session_messages: usize,
    /// Maximum total content bytes in session (default: 10MB)
    pub max_session_content_bytes: usize,
    /// Maximum active tasks (default: 20)
    pub max_active_tasks: usize,
    /// Maximum total tasks including completed (default: 100)
    pub max_total_tasks: usize,
    /// Age after which completed tasks are cleaned up (default: 1 hour)
    pub task_cleanup_age_ms: u64,
    /// Maximum commands per LLM response (default: 10)
    pub max_commands_per_response: usize,
    /// Maximum task description length (default: 1000)
    pub max_task_description_length: usize,
}

impl Default for ConductorLimits {
    fn default() -> Self {
        Self {
            max_message_size: 100 * 1024, // 100KB
            max_messages_per_minute: 30,
            max_command_args: 10,
            max_session_messages: 1000,
            max_session_content_bytes: 10 * 1024 * 1024, // 10MB
            max_active_tasks: 20,
            max_total_tasks: 100,
            task_cleanup_age_ms: 60 * 60 * 1000, // 1 hour
            max_commands_per_response: 10,
            max_task_description_length: 1000,
        }
    }
}

impl ConductorLimits {
    /// Create limits from environment variables with fallback to defaults
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            max_message_size: std::env::var("CONDUCTOR_MAX_MESSAGE_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_message_size),
            max_messages_per_minute: std::env::var("CONDUCTOR_MAX_MESSAGES_PER_MINUTE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_messages_per_minute),
            max_command_args: std::env::var("CONDUCTOR_MAX_COMMAND_ARGS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_command_args),
            max_session_messages: std::env::var("CONDUCTOR_MAX_SESSION_MESSAGES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_session_messages),
            max_session_content_bytes: std::env::var("CONDUCTOR_MAX_SESSION_CONTENT_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_session_content_bytes),
            max_active_tasks: std::env::var("CONDUCTOR_MAX_ACTIVE_TASKS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_active_tasks),
            max_total_tasks: std::env::var("CONDUCTOR_MAX_TOTAL_TASKS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_total_tasks),
            task_cleanup_age_ms: std::env::var("CONDUCTOR_TASK_CLEANUP_AGE_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.task_cleanup_age_ms),
            max_commands_per_response: std::env::var("CONDUCTOR_MAX_COMMANDS_PER_RESPONSE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_commands_per_response),
            max_task_description_length: std::env::var("CONDUCTOR_MAX_TASK_DESCRIPTION_LENGTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_task_description_length),
        }
    }
}

/// Result of input validation
#[derive(Clone, Debug)]
pub enum ValidationResult {
    /// Input is valid
    Valid,
    /// Input is invalid with reason
    Invalid(String),
    /// Input was rate limited
    RateLimited(String),
}

impl ValidationResult {
    /// Check if the result indicates valid input
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    /// Get the error message if invalid
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Valid => None,
            Self::Invalid(msg) | Self::RateLimited(msg) => Some(msg),
        }
    }
}

/// Input validator for surface events
///
/// Validates user input before processing to prevent:
/// - Oversized messages
/// - Rate limiting bypass
/// - Control character injection
pub struct InputValidator {
    limits: ConductorLimits,
    /// Message count for rate limiting
    message_count: AtomicU32,
    /// When the current rate limit window started
    window_start: std::sync::Mutex<Instant>,
}

impl InputValidator {
    /// Create a new input validator with the given limits
    pub fn new(limits: ConductorLimits) -> Self {
        Self {
            limits,
            message_count: AtomicU32::new(0),
            window_start: std::sync::Mutex::new(Instant::now()),
        }
    }

    /// Validate a user message
    pub fn validate_message(&self, content: &str) -> ValidationResult {
        // Check rate limit first
        if let result @ ValidationResult::RateLimited(_) = self.check_rate_limit() {
            return result;
        }

        // Check size
        if content.len() > self.limits.max_message_size {
            return ValidationResult::Invalid(format!(
                "Message too large: {} bytes (max: {})",
                content.len(),
                self.limits.max_message_size
            ));
        }

        // Check for control characters (except newline, tab)
        if content
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
        {
            return ValidationResult::Invalid(
                "Message contains invalid control characters".to_string(),
            );
        }

        ValidationResult::Valid
    }

    /// Validate a user command
    pub fn validate_command(&self, command: &str, args: &[String]) -> ValidationResult {
        // Check rate limit
        if let result @ ValidationResult::RateLimited(_) = self.check_rate_limit() {
            return result;
        }

        // Check command name
        if command.is_empty() {
            return ValidationResult::Invalid("Empty command name".to_string());
        }

        if command.len() > 50 {
            return ValidationResult::Invalid("Command name too long".to_string());
        }

        // Check for invalid characters in command
        if !command
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return ValidationResult::Invalid(
                "Command name contains invalid characters".to_string(),
            );
        }

        // Check argument count
        if args.len() > self.limits.max_command_args {
            return ValidationResult::Invalid(format!(
                "Too many command arguments: {} (max: {})",
                args.len(),
                self.limits.max_command_args
            ));
        }

        // Check each argument
        for (i, arg) in args.iter().enumerate() {
            if arg.len() > 1000 {
                return ValidationResult::Invalid(format!(
                    "Argument {} too long: {} bytes",
                    i,
                    arg.len()
                ));
            }
            if arg
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t')
            {
                return ValidationResult::Invalid(format!(
                    "Argument {} contains control characters",
                    i
                ));
            }
        }

        ValidationResult::Valid
    }

    /// Check and update rate limit
    fn check_rate_limit(&self) -> ValidationResult {
        let mut window_start = self.window_start.lock().unwrap();
        let now = Instant::now();

        // Reset window if it's been more than a minute
        if now.duration_since(*window_start) >= Duration::from_secs(60) {
            *window_start = now;
            self.message_count.store(1, Ordering::SeqCst);
            return ValidationResult::Valid;
        }

        // Increment and check count
        let count = self.message_count.fetch_add(1, Ordering::SeqCst) + 1;
        if count > self.limits.max_messages_per_minute {
            return ValidationResult::RateLimited(format!(
                "Rate limit exceeded: {} messages/minute (max: {})",
                count, self.limits.max_messages_per_minute
            ));
        }

        ValidationResult::Valid
    }

    /// Get the current limits
    pub fn limits(&self) -> &ConductorLimits {
        &self.limits
    }
}

/// Reason why a command was rejected
#[derive(Clone, Debug)]
pub enum CommandRejectionReason {
    /// Command is not in the allowlist
    NotAllowed(String),
    /// Too many commands in response
    RateLimitExceeded,
    /// Command not allowed in current state
    InvalidState(String),
    /// Invalid command arguments
    InvalidArguments(String),
    /// Agent not in allowlist
    UnknownAgent(String),
    /// Task description validation failed
    InvalidTaskDescription(String),
}

impl std::fmt::Display for CommandRejectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAllowed(cmd) => write!(f, "Command '{}' is not allowed", cmd),
            Self::RateLimitExceeded => write!(f, "Too many commands in response"),
            Self::InvalidState(msg) => write!(f, "Command not allowed in current state: {}", msg),
            Self::InvalidArguments(msg) => write!(f, "Invalid command arguments: {}", msg),
            Self::UnknownAgent(agent) => write!(f, "Unknown agent '{}' not in allowlist", agent),
            Self::InvalidTaskDescription(msg) => write!(f, "Invalid task description: {}", msg),
        }
    }
}

/// Command validator for LLM-generated commands
///
/// Validates commands extracted from LLM responses to prevent:
/// - Execution of unauthorized commands
/// - Command injection through jailbreaking
/// - Task agent injection
pub struct CommandValidator {
    /// Allowed command names (first word after yolla:)
    /// Currently used for configuration/extension, validation happens at enum level.
    #[allow(dead_code)]
    allowed_commands: HashSet<String>,
    /// Allowed agent names for task commands
    allowed_agents: HashSet<String>,
    /// Commands allowed per response
    max_commands_per_response: usize,
    /// Maximum task description length
    max_task_description_length: usize,
    /// Commands seen in current response
    commands_in_response: AtomicU32,
    /// Rejected commands log (for monitoring)
    rejected_commands: std::sync::Mutex<Vec<(String, CommandRejectionReason)>>,
}

impl CommandValidator {
    /// Create a new command validator with default allowlists
    pub fn new(limits: &ConductorLimits) -> Self {
        Self {
            allowed_commands: Self::default_allowed_commands(),
            allowed_agents: Self::default_allowed_agents(),
            max_commands_per_response: limits.max_commands_per_response,
            max_task_description_length: limits.max_task_description_length,
            commands_in_response: AtomicU32::new(0),
            rejected_commands: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Create a validator with custom allowlists
    pub fn with_allowlists(
        limits: &ConductorLimits,
        allowed_commands: HashSet<String>,
        allowed_agents: HashSet<String>,
    ) -> Self {
        Self {
            allowed_commands,
            allowed_agents,
            max_commands_per_response: limits.max_commands_per_response,
            max_task_description_length: limits.max_task_description_length,
            commands_in_response: AtomicU32::new(0),
            rejected_commands: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Default allowed commands - safe avatar controls
    fn default_allowed_commands() -> HashSet<String> {
        [
            // Movement commands
            "move", "wander", "stop", "point", "follow", // Expression commands
            "mood", "size", "hide", "show", // Gesture commands
            "wave", "nod", "shake", "bounce", "spin", "dance", "swim", "stretch", "yawn", "wiggle",
            "peek", // Reaction commands
            "react", "laugh", "gasp", "tada", "oops", "love", "wink",
            // Task commands (validated separately)
            "task",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Default allowed agents - known specialist agents
    fn default_allowed_agents() -> HashSet<String> {
        [
            "ethical-hacker",
            "backend-engineer",
            "frontend-specialist",
            "senior-full-stack-developer",
            "solutions-architect",
            "ux-ui-designer",
            "qa-engineer",
            "privacy-researcher",
            "devops-engineer",
            "relational-database-expert",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Reset the per-response command counter
    ///
    /// Call this at the start of processing each LLM response.
    pub fn reset_response_counter(&self) {
        self.commands_in_response.store(0, Ordering::SeqCst);
    }

    /// Validate an avatar command
    ///
    /// Returns Ok(()) if valid, Err with reason if rejected.
    pub fn validate_command(&self, cmd: &AvatarCommand) -> Result<(), CommandRejectionReason> {
        // Check rate limit
        let count = self.commands_in_response.fetch_add(1, Ordering::SeqCst) + 1;
        if count > self.max_commands_per_response as u32 {
            let reason = CommandRejectionReason::RateLimitExceeded;
            self.log_rejection(&format!("{:?}", cmd), reason.clone());
            return Err(reason);
        }

        // Validate based on command type
        match cmd {
            AvatarCommand::Task(task_cmd) => self.validate_task_command(task_cmd),
            AvatarCommand::CustomSprite(data) => {
                // CustomSprite could be used for injection, limit it
                if data.len() > 100 {
                    let reason = CommandRejectionReason::InvalidArguments(
                        "CustomSprite data too long".to_string(),
                    );
                    self.log_rejection("sprite", reason.clone());
                    return Err(reason);
                }
                // Only allow alphanumeric and basic punctuation
                if !data
                    .chars()
                    .all(|c| c.is_alphanumeric() || " -_.".contains(c))
                {
                    let reason = CommandRejectionReason::InvalidArguments(
                        "CustomSprite contains invalid characters".to_string(),
                    );
                    self.log_rejection("sprite", reason.clone());
                    return Err(reason);
                }
                Ok(())
            }
            AvatarCommand::PointAt {
                x_percent,
                y_percent,
            } => {
                // Already bounded to u8, but double-check
                if *x_percent > 100 || *y_percent > 100 {
                    let reason = CommandRejectionReason::InvalidArguments(
                        "Point coordinates out of range".to_string(),
                    );
                    self.log_rejection("point", reason.clone());
                    return Err(reason);
                }
                Ok(())
            }
            // All other commands are safe (they're enums with limited values)
            _ => Ok(()),
        }
    }

    /// Validate a task command specifically
    fn validate_task_command(
        &self,
        cmd: &crate::avatar::TaskCommand,
    ) -> Result<(), CommandRejectionReason> {
        use crate::avatar::TaskCommand;

        match cmd {
            TaskCommand::Start { agent, description } => {
                // Validate agent against allowlist
                if !self.allowed_agents.contains(agent) {
                    let reason = CommandRejectionReason::UnknownAgent(agent.clone());
                    self.log_rejection(&format!("task start {}", agent), reason.clone());
                    return Err(reason);
                }

                // Validate description
                if let Err(reason) = self.validate_task_description(description) {
                    self.log_rejection(&format!("task start {}", agent), reason.clone());
                    return Err(reason);
                }

                Ok(())
            }
            TaskCommand::Progress { task_id, percent } => {
                // Validate task_id format
                if let Err(reason) = self.validate_task_id(task_id) {
                    return Err(reason);
                }
                // percent is already bounded by u8 and min(100) in parser
                if *percent > 100 {
                    return Err(CommandRejectionReason::InvalidArguments(
                        "Progress percent out of range".to_string(),
                    ));
                }
                Ok(())
            }
            TaskCommand::Done { task_id }
            | TaskCommand::Focus { task_id }
            | TaskCommand::PointAt { task_id }
            | TaskCommand::Hover { task_id }
            | TaskCommand::Celebrate { task_id } => self.validate_task_id(task_id),
            TaskCommand::Fail { task_id, reason } => {
                self.validate_task_id(task_id)?;
                // Validate reason length
                if reason.len() > self.max_task_description_length {
                    return Err(CommandRejectionReason::InvalidTaskDescription(
                        "Failure reason too long".to_string(),
                    ));
                }
                Ok(())
            }
        }
    }

    /// Validate a task description
    fn validate_task_description(&self, description: &str) -> Result<(), CommandRejectionReason> {
        // Check length
        if description.len() > self.max_task_description_length {
            return Err(CommandRejectionReason::InvalidTaskDescription(format!(
                "Description too long: {} bytes (max: {})",
                description.len(),
                self.max_task_description_length
            )));
        }

        // Check for control characters
        if description.chars().any(|c| c.is_control() && c != ' ') {
            return Err(CommandRejectionReason::InvalidTaskDescription(
                "Description contains control characters".to_string(),
            ));
        }

        // Check for empty description
        if description.trim().is_empty() {
            return Err(CommandRejectionReason::InvalidTaskDescription(
                "Description is empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate a task ID format
    fn validate_task_id(&self, task_id: &str) -> Result<(), CommandRejectionReason> {
        // Task IDs should be reasonably short
        if task_id.len() > 100 {
            return Err(CommandRejectionReason::InvalidArguments(
                "Task ID too long".to_string(),
            ));
        }

        // Task IDs should be alphanumeric with underscores
        if !task_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(CommandRejectionReason::InvalidArguments(
                "Task ID contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Log a rejected command for monitoring
    fn log_rejection(&self, cmd: &str, reason: CommandRejectionReason) {
        tracing::warn!(
            command = cmd,
            reason = %reason,
            "Rejected LLM command"
        );

        let mut rejected = self.rejected_commands.lock().unwrap();
        // Keep only last 100 rejections
        if rejected.len() >= 100 {
            rejected.remove(0);
        }
        rejected.push((cmd.to_string(), reason));
    }

    /// Get rejected commands log
    pub fn rejected_commands(&self) -> Vec<(String, CommandRejectionReason)> {
        self.rejected_commands.lock().unwrap().clone()
    }

    /// Clear rejected commands log
    pub fn clear_rejected_log(&self) {
        self.rejected_commands.lock().unwrap().clear();
    }

    /// Check if an agent is allowed
    pub fn is_agent_allowed(&self, agent: &str) -> bool {
        self.allowed_agents.contains(agent)
    }

    /// Add an agent to the allowlist
    pub fn allow_agent(&mut self, agent: String) {
        self.allowed_agents.insert(agent);
    }

    /// Remove an agent from the allowlist
    pub fn disallow_agent(&mut self, agent: &str) {
        self.allowed_agents.remove(agent);
    }

    /// Get the current agent allowlist
    pub fn allowed_agents(&self) -> &HashSet<String> {
        &self.allowed_agents
    }
}

/// Security configuration combining all security settings
#[derive(Clone, Debug, Default)]
pub struct SecurityConfig {
    /// Resource limits
    pub limits: ConductorLimits,
    /// Additional allowed agents (beyond defaults)
    pub additional_agents: Vec<String>,
    /// Whether to log all rejected commands
    pub log_rejections: bool,
}

impl SecurityConfig {
    /// Create from environment variables
    pub fn from_env() -> Self {
        Self {
            limits: ConductorLimits::from_env(),
            additional_agents: std::env::var("CONDUCTOR_ADDITIONAL_AGENTS")
                .ok()
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
            log_rejections: std::env::var("CONDUCTOR_LOG_REJECTIONS")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avatar::{AvatarGesture, AvatarMood, AvatarPosition, TaskCommand};

    #[test]
    fn test_conductor_limits_default() {
        let limits = ConductorLimits::default();
        assert_eq!(limits.max_message_size, 100 * 1024);
        assert_eq!(limits.max_messages_per_minute, 30);
        assert_eq!(limits.max_session_messages, 1000);
    }

    #[test]
    fn test_input_validator_valid_message() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_message("Hello, world!");
        assert!(result.is_valid());
    }

    #[test]
    fn test_input_validator_oversized_message() {
        let mut limits = ConductorLimits::default();
        limits.max_message_size = 10;
        let validator = InputValidator::new(limits);
        let result = validator.validate_message("This message is too long!");
        assert!(!result.is_valid());
        assert!(result.error_message().unwrap().contains("too large"));
    }

    #[test]
    fn test_input_validator_control_characters() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_message("Hello\x00world");
        assert!(!result.is_valid());
        assert!(result.error_message().unwrap().contains("control"));
    }

    #[test]
    fn test_input_validator_newlines_allowed() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_message("Hello\nworld\ttab");
        assert!(result.is_valid());
    }

    #[test]
    fn test_input_validator_rate_limit() {
        let mut limits = ConductorLimits::default();
        limits.max_messages_per_minute = 2;
        let validator = InputValidator::new(limits);

        assert!(validator.validate_message("1").is_valid());
        assert!(validator.validate_message("2").is_valid());
        let result = validator.validate_message("3");
        assert!(!result.is_valid());
        assert!(matches!(result, ValidationResult::RateLimited(_)));
    }

    #[test]
    fn test_command_validator_valid_command() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Mood(AvatarMood::Happy);
        assert!(validator.validate_command(&cmd).is_ok());

        let cmd = AvatarCommand::Gesture(AvatarGesture::Wave);
        assert!(validator.validate_command(&cmd).is_ok());

        let cmd = AvatarCommand::MoveTo(AvatarPosition::Center);
        assert!(validator.validate_command(&cmd).is_ok());
    }

    #[test]
    fn test_command_validator_rate_limit() {
        let mut limits = ConductorLimits::default();
        limits.max_commands_per_response = 2;
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Mood(AvatarMood::Happy);
        assert!(validator.validate_command(&cmd).is_ok());
        assert!(validator.validate_command(&cmd).is_ok());

        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::RateLimitExceeded)
        ));

        // Reset and try again
        validator.reset_response_counter();
        assert!(validator.validate_command(&cmd).is_ok());
    }

    #[test]
    fn test_command_validator_task_allowed_agent() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Start {
            agent: "ethical-hacker".to_string(),
            description: "Test task".to_string(),
        });
        assert!(validator.validate_command(&cmd).is_ok());
    }

    #[test]
    fn test_command_validator_task_unknown_agent() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Start {
            agent: "malicious-agent".to_string(),
            description: "Test task".to_string(),
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::UnknownAgent(_))
        ));
    }

    #[test]
    fn test_command_validator_task_description_too_long() {
        let mut limits = ConductorLimits::default();
        limits.max_task_description_length = 10;
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Start {
            agent: "ethical-hacker".to_string(),
            description: "This description is way too long for the limit".to_string(),
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::InvalidTaskDescription(_))
        ));
    }

    #[test]
    fn test_command_validator_custom_sprite_validation() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        // Valid sprite
        let cmd = AvatarCommand::CustomSprite("custom-sprite-1".to_string());
        assert!(validator.validate_command(&cmd).is_ok());

        // Invalid characters
        let cmd = AvatarCommand::CustomSprite("sprite<script>".to_string());
        assert!(validator.validate_command(&cmd).is_err());

        // Too long
        let cmd = AvatarCommand::CustomSprite("a".repeat(200));
        assert!(validator.validate_command(&cmd).is_err());
    }

    #[test]
    fn test_validate_command_empty() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_command("", &[]);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validate_command_invalid_chars() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_command("test;rm -rf", &[]);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validate_command_too_many_args() {
        let mut limits = ConductorLimits::default();
        limits.max_command_args = 2;
        let validator = InputValidator::new(limits);
        let result = validator
            .validate_command("test", &["a".to_string(), "b".to_string(), "c".to_string()]);
        assert!(!result.is_valid());
    }

    // ========================================================================
    // SECURITY TESTS: Input Injection Prevention
    // ========================================================================

    #[test]
    fn test_input_null_byte_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        // Null byte injection attempt
        let result = validator.validate_message("Hello\x00World");
        assert!(!result.is_valid());
        assert!(result.error_message().unwrap().contains("control"));
    }

    #[test]
    fn test_input_bell_character_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        // Bell character (could be annoying in terminal)
        let result = validator.validate_message("Hello\x07World");
        assert!(!result.is_valid());
    }

    #[test]
    fn test_input_escape_sequence_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        // ANSI escape sequence start
        let result = validator.validate_message("Hello\x1b[31mRED\x1b[0m");
        assert!(!result.is_valid());
        assert!(result.error_message().unwrap().contains("control"));
    }

    #[test]
    fn test_input_backspace_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        // Backspace could manipulate terminal display
        let result = validator.validate_message("Safe\x08\x08\x08\x08EVIL");
        assert!(!result.is_valid());
    }

    #[test]
    fn test_input_form_feed_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        // Form feed control character
        let result = validator.validate_message("Page1\x0cPage2");
        assert!(!result.is_valid());
    }

    #[test]
    fn test_input_carriage_return_allowed() {
        let validator = InputValidator::new(ConductorLimits::default());
        // Carriage return is allowed (Windows line endings)
        let result = validator.validate_message("Hello\r\nWorld");
        assert!(result.is_valid());
    }

    #[test]
    fn test_input_vertical_tab_rejected() {
        let validator = InputValidator::new(ConductorLimits::default());
        // Vertical tab is a control character
        let result = validator.validate_message("Hello\x0bWorld");
        assert!(!result.is_valid());
    }

    // ========================================================================
    // SECURITY TESTS: Command Injection Prevention
    // ========================================================================

    #[test]
    fn test_command_shell_semicolon_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_command("test;rm", &[]);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_command_shell_pipe_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_command("test|cat", &[]);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_command_shell_ampersand_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_command("test&bg", &[]);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_command_shell_backtick_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_command("test`id`", &[]);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_command_shell_dollar_injection() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_command("test$HOME", &[]);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_command_name_too_long() {
        let validator = InputValidator::new(ConductorLimits::default());
        let long_cmd = "a".repeat(51);
        let result = validator.validate_command(&long_cmd, &[]);
        assert!(!result.is_valid());
        assert!(result.error_message().unwrap().contains("too long"));
    }

    #[test]
    fn test_command_valid_with_hyphen_underscore() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_command("my-test_command", &[]);
        assert!(result.is_valid());
    }

    #[test]
    fn test_command_argument_control_char_rejected() {
        let validator = InputValidator::new(ConductorLimits::default());
        let result = validator.validate_command("test", &["arg\x00value".to_string()]);
        assert!(!result.is_valid());
        assert!(result.error_message().unwrap().contains("control"));
    }

    #[test]
    fn test_command_argument_too_long() {
        let validator = InputValidator::new(ConductorLimits::default());
        let long_arg = "a".repeat(1001);
        let result = validator.validate_command("test", &[long_arg]);
        assert!(!result.is_valid());
        assert!(result.error_message().unwrap().contains("too long"));
    }

    #[test]
    fn test_command_argument_newline_allowed() {
        let validator = InputValidator::new(ConductorLimits::default());
        // Newlines in args should be allowed (multi-line content)
        let result = validator.validate_command("test", &["line1\nline2".to_string()]);
        assert!(result.is_valid());
    }

    // ========================================================================
    // SECURITY TESTS: Task Agent Injection Prevention
    // ========================================================================

    #[test]
    fn test_task_agent_with_path_chars_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Start {
            agent: "../../../etc/passwd".to_string(),
            description: "Path traversal attempt".to_string(),
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::UnknownAgent(_))
        ));
    }

    #[test]
    fn test_task_agent_with_shell_chars_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Start {
            agent: "agent;rm -rf /".to_string(),
            description: "Shell injection attempt".to_string(),
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::UnknownAgent(_))
        ));
    }

    #[test]
    fn test_task_description_empty_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Start {
            agent: "ethical-hacker".to_string(),
            description: "   ".to_string(), // Only whitespace
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::InvalidTaskDescription(_))
        ));
    }

    #[test]
    fn test_task_description_control_chars_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Start {
            agent: "ethical-hacker".to_string(),
            description: "Task\x00with\x1bnull".to_string(),
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::InvalidTaskDescription(_))
        ));
    }

    #[test]
    fn test_task_id_too_long_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Done {
            task_id: "a".repeat(101),
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::InvalidArguments(_))
        ));
    }

    #[test]
    fn test_task_id_invalid_chars_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Done {
            task_id: "task;rm -rf /".to_string(),
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::InvalidArguments(_))
        ));
    }

    #[test]
    fn test_task_id_valid_format() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Done {
            task_id: "task_123_abc-def".to_string(),
        });
        assert!(validator.validate_command(&cmd).is_ok());
    }

    #[test]
    fn test_task_progress_over_100_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        // Note: percent is u8 so max is 255, but > 100 should be rejected
        let cmd = AvatarCommand::Task(TaskCommand::Progress {
            task_id: "task_1".to_string(),
            percent: 150,
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::InvalidArguments(_))
        ));
    }

    #[test]
    fn test_task_progress_boundary_values() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        // 0% is valid
        let cmd = AvatarCommand::Task(TaskCommand::Progress {
            task_id: "task_1".to_string(),
            percent: 0,
        });
        assert!(validator.validate_command(&cmd).is_ok());

        // Reset counter for next test
        validator.reset_response_counter();

        // 100% is valid
        let cmd = AvatarCommand::Task(TaskCommand::Progress {
            task_id: "task_1".to_string(),
            percent: 100,
        });
        assert!(validator.validate_command(&cmd).is_ok());
    }

    #[test]
    fn test_task_fail_reason_too_long() {
        let mut limits = ConductorLimits::default();
        limits.max_task_description_length = 50;
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::Task(TaskCommand::Fail {
            task_id: "task_1".to_string(),
            reason: "a".repeat(100),
        });
        let result = validator.validate_command(&cmd);
        assert!(matches!(
            result,
            Err(CommandRejectionReason::InvalidTaskDescription(_))
        ));
    }

    // ========================================================================
    // SECURITY TESTS: CustomSprite Injection Prevention
    // ========================================================================

    #[test]
    fn test_custom_sprite_path_traversal_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::CustomSprite("../../../etc/passwd".to_string());
        let result = validator.validate_command(&cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_sprite_html_injection_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::CustomSprite("<script>alert(1)</script>".to_string());
        let result = validator.validate_command(&cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_sprite_valid_names() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        // Valid sprite names
        let cmd = AvatarCommand::CustomSprite("sprite-name_v1".to_string());
        assert!(validator.validate_command(&cmd).is_ok());

        validator.reset_response_counter();

        let cmd = AvatarCommand::CustomSprite("MySprite.v2".to_string());
        assert!(validator.validate_command(&cmd).is_ok());
    }

    // ========================================================================
    // SECURITY TESTS: PointAt Boundary Validation
    // ========================================================================

    #[test]
    fn test_point_at_valid_boundaries() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        // Valid: 0,0
        let cmd = AvatarCommand::PointAt {
            x_percent: 0,
            y_percent: 0,
        };
        assert!(validator.validate_command(&cmd).is_ok());

        validator.reset_response_counter();

        // Valid: 100,100
        let cmd = AvatarCommand::PointAt {
            x_percent: 100,
            y_percent: 100,
        };
        assert!(validator.validate_command(&cmd).is_ok());

        validator.reset_response_counter();

        // Valid: 50,50
        let cmd = AvatarCommand::PointAt {
            x_percent: 50,
            y_percent: 50,
        };
        assert!(validator.validate_command(&cmd).is_ok());
    }

    #[test]
    fn test_point_at_over_100_rejected() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let cmd = AvatarCommand::PointAt {
            x_percent: 101,
            y_percent: 50,
        };
        let result = validator.validate_command(&cmd);
        assert!(result.is_err());
    }

    // ========================================================================
    // SECURITY TESTS: Allowlist Management
    // ========================================================================

    #[test]
    fn test_allow_agent_adds_to_set() {
        let limits = ConductorLimits::default();
        let mut validator = CommandValidator::new(&limits);

        assert!(!validator.is_agent_allowed("custom-agent"));
        validator.allow_agent("custom-agent".to_string());
        assert!(validator.is_agent_allowed("custom-agent"));
    }

    #[test]
    fn test_disallow_agent_removes_from_set() {
        let limits = ConductorLimits::default();
        let mut validator = CommandValidator::new(&limits);

        assert!(validator.is_agent_allowed("ethical-hacker"));
        validator.disallow_agent("ethical-hacker");
        assert!(!validator.is_agent_allowed("ethical-hacker"));
    }

    #[test]
    fn test_allowed_agents_returns_set() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let agents = validator.allowed_agents();
        assert!(agents.contains("ethical-hacker"));
        assert!(agents.contains("backend-engineer"));
        assert!(agents.contains("qa-engineer"));
    }

    // ========================================================================
    // SECURITY TESTS: Rejection Logging
    // ========================================================================

    #[test]
    fn test_rejected_commands_logged() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        // Trigger a rejection
        let cmd = AvatarCommand::Task(TaskCommand::Start {
            agent: "unknown-agent".to_string(),
            description: "Test".to_string(),
        });
        let _ = validator.validate_command(&cmd);

        let rejected = validator.rejected_commands();
        assert!(!rejected.is_empty());
    }

    #[test]
    fn test_rejected_commands_log_limit() {
        let mut limits = ConductorLimits::default();
        limits.max_commands_per_response = 200; // Allow many commands
        let validator = CommandValidator::new(&limits);

        // Trigger 150 rejections
        for i in 0..150 {
            let cmd = AvatarCommand::Task(TaskCommand::Start {
                agent: format!("unknown-{}", i),
                description: "Test".to_string(),
            });
            let _ = validator.validate_command(&cmd);
        }

        // Log should be capped at 100
        let rejected = validator.rejected_commands();
        assert_eq!(rejected.len(), 100);
    }

    #[test]
    fn test_clear_rejected_log() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        // Trigger a rejection
        let cmd = AvatarCommand::Task(TaskCommand::Start {
            agent: "unknown".to_string(),
            description: "Test".to_string(),
        });
        let _ = validator.validate_command(&cmd);
        assert!(!validator.rejected_commands().is_empty());

        validator.clear_rejected_log();
        assert!(validator.rejected_commands().is_empty());
    }

    // ========================================================================
    // SECURITY TESTS: ValidationResult
    // ========================================================================

    #[test]
    fn test_validation_result_is_valid_variants() {
        assert!(ValidationResult::Valid.is_valid());
        assert!(!ValidationResult::Invalid("test".to_string()).is_valid());
        assert!(!ValidationResult::RateLimited("test".to_string()).is_valid());
    }

    #[test]
    fn test_validation_result_error_messages() {
        assert!(ValidationResult::Valid.error_message().is_none());
        assert_eq!(
            ValidationResult::Invalid("error1".to_string()).error_message(),
            Some("error1")
        );
        assert_eq!(
            ValidationResult::RateLimited("error2".to_string()).error_message(),
            Some("error2")
        );
    }

    // ========================================================================
    // SECURITY TESTS: CommandRejectionReason Display
    // ========================================================================

    #[test]
    fn test_rejection_reason_display() {
        let reason = CommandRejectionReason::NotAllowed("test".to_string());
        assert!(format!("{}", reason).contains("not allowed"));

        let reason = CommandRejectionReason::RateLimitExceeded;
        assert!(format!("{}", reason).contains("Too many"));

        let reason = CommandRejectionReason::UnknownAgent("bad".to_string());
        assert!(format!("{}", reason).contains("Unknown agent"));

        let reason = CommandRejectionReason::InvalidArguments("bad args".to_string());
        assert!(format!("{}", reason).contains("Invalid"));
    }

    // ========================================================================
    // SECURITY TESTS: SecurityConfig
    // ========================================================================

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.additional_agents.is_empty());
        assert!(!config.log_rejections); // Default is false
        assert_eq!(config.limits.max_message_size, 100 * 1024);
    }

    // ========================================================================
    // SECURITY TESTS: All Default Allowed Agents Present
    // ========================================================================

    #[test]
    fn test_all_default_agents_allowed() {
        let limits = ConductorLimits::default();
        let validator = CommandValidator::new(&limits);

        let expected_agents = [
            "ethical-hacker",
            "backend-engineer",
            "frontend-specialist",
            "senior-full-stack-developer",
            "solutions-architect",
            "ux-ui-designer",
            "qa-engineer",
            "privacy-researcher",
            "devops-engineer",
            "relational-database-expert",
        ];

        for agent in expected_agents {
            assert!(
                validator.is_agent_allowed(agent),
                "Expected agent '{}' to be allowed",
                agent
            );
        }
    }

    // ========================================================================
    // SECURITY TESTS: Custom Allowlist Constructor
    // ========================================================================

    #[test]
    fn test_custom_allowlists() {
        let limits = ConductorLimits::default();
        let mut custom_commands = HashSet::new();
        custom_commands.insert("custom-cmd".to_string());

        let mut custom_agents = HashSet::new();
        custom_agents.insert("custom-agent".to_string());

        let validator = CommandValidator::with_allowlists(&limits, custom_commands, custom_agents);

        assert!(validator.is_agent_allowed("custom-agent"));
        assert!(!validator.is_agent_allowed("ethical-hacker")); // Not in custom list
    }

    // ========================================================================
    // SECURITY TESTS: InputValidator Limits Accessor
    // ========================================================================

    #[test]
    fn test_input_validator_limits_accessor() {
        let mut limits = ConductorLimits::default();
        limits.max_message_size = 12345;
        let validator = InputValidator::new(limits);

        assert_eq!(validator.limits().max_message_size, 12345);
    }
}
