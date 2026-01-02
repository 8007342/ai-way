//! Fallback Chain Management
//!
//! Manages ordered fallback sequences for models. When a primary model fails,
//! the fallback chain determines which alternative models to try and in what order.
//!
//! # Architecture
//!
//! ```text
//! Primary Model (gpt-4) fails
//!         |
//!         v
//! FallbackChainManager.get_next_fallback()
//!         |
//!         v
//! Fallback 1: claude-3 (if healthy)
//!         |
//!         v
//! Fallback 2: gpt-3.5 (if healthy)
//!         |
//!         v
//! Fallback 3: local-llama (last resort)
//! ```
//!
//! # Features
//!
//! - Ordered fallback chains per model
//! - Automatic chain generation based on task class affinity
//! - Health-aware fallback selection (skips unhealthy models)
//! - Cycle detection to prevent infinite loops
//! - Dynamic chain updates at runtime

use std::collections::{HashMap, HashSet};

use parking_lot::RwLock;

use super::config::{ModelProfile, TaskClass};

// ============================================================================
// Fallback Chain
// ============================================================================

/// A single fallback chain for a model
#[derive(Clone, Debug)]
pub struct FallbackChain {
    /// Primary model ID
    pub primary: String,

    /// Ordered list of fallback model IDs
    pub fallbacks: Vec<String>,

    /// Task class this chain is optimized for (if any)
    pub task_class: Option<TaskClass>,

    /// Whether this chain was auto-generated
    pub auto_generated: bool,
}

impl FallbackChain {
    /// Create a new fallback chain
    pub fn new(primary: impl Into<String>, fallbacks: Vec<String>) -> Self {
        Self {
            primary: primary.into(),
            fallbacks,
            task_class: None,
            auto_generated: false,
        }
    }

    /// Create a chain optimized for a specific task class
    pub fn for_task_class(
        primary: impl Into<String>,
        fallbacks: Vec<String>,
        task_class: TaskClass,
    ) -> Self {
        Self {
            primary: primary.into(),
            fallbacks,
            task_class: Some(task_class),
            auto_generated: false,
        }
    }

    /// Get the next fallback after the given model
    /// Returns None if there are no more fallbacks
    #[must_use]
    pub fn next_fallback(&self, current: &str) -> Option<&String> {
        if current == self.primary {
            return self.fallbacks.first();
        }

        let pos = self.fallbacks.iter().position(|m| m == current)?;
        self.fallbacks.get(pos + 1)
    }

    /// Get all remaining fallbacks after the given model
    #[must_use]
    pub fn remaining_fallbacks(&self, current: &str) -> Vec<&String> {
        if current == self.primary {
            return self.fallbacks.iter().collect();
        }

        if let Some(pos) = self.fallbacks.iter().position(|m| m == current) {
            self.fallbacks.iter().skip(pos + 1).collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a model is in this chain (either primary or fallback)
    #[must_use]
    pub fn contains(&self, model_id: &str) -> bool {
        self.primary == model_id || self.fallbacks.iter().any(|m| m == model_id)
    }

    /// Get the total depth of this chain (including primary)
    #[must_use]
    pub fn depth(&self) -> usize {
        1 + self.fallbacks.len()
    }
}

// ============================================================================
// Fallback Chain Manager
// ============================================================================

/// Manages fallback chains for all models
///
/// Thread-safe manager that maintains fallback sequences and provides
/// health-aware fallback selection.
pub struct FallbackChainManager {
    /// Explicit fallback chains (`model_id` -> chain)
    chains: RwLock<HashMap<String, FallbackChain>>,

    /// Task-specific fallback chains (`task_class` -> chain)
    task_chains: RwLock<HashMap<TaskClass, FallbackChain>>,

    /// Model profiles for auto-generation
    profiles: RwLock<HashMap<String, ModelProfile>>,

    /// Maximum chain depth to prevent excessive fallbacks
    max_chain_depth: usize,
}

impl FallbackChainManager {
    /// Create a new fallback chain manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            chains: RwLock::new(HashMap::new()),
            task_chains: RwLock::new(HashMap::new()),
            profiles: RwLock::new(HashMap::new()),
            max_chain_depth: 5,
        }
    }

    /// Create with custom max chain depth
    #[must_use]
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            chains: RwLock::new(HashMap::new()),
            task_chains: RwLock::new(HashMap::new()),
            profiles: RwLock::new(HashMap::new()),
            max_chain_depth: max_depth,
        }
    }

    /// Register a model profile (used for auto-generation)
    pub fn register_profile(&self, profile: ModelProfile) {
        let mut profiles = self.profiles.write();
        profiles.insert(profile.model_id.clone(), profile);
    }

    /// Set an explicit fallback chain for a model
    pub fn set_chain(&self, model_id: impl Into<String>, fallbacks: Vec<String>) {
        let model_id = model_id.into();
        let chain = FallbackChain::new(model_id.clone(), fallbacks);
        let mut chains = self.chains.write();
        chains.insert(model_id, chain);
    }

    /// Set a task-specific fallback chain
    pub fn set_task_chain(
        &self,
        task_class: TaskClass,
        primary: impl Into<String>,
        fallbacks: Vec<String>,
    ) {
        let chain = FallbackChain::for_task_class(primary, fallbacks, task_class);
        let mut task_chains = self.task_chains.write();
        task_chains.insert(task_class, chain);
    }

    /// Get the fallback chain for a model
    pub fn get_chain(&self, model_id: &str) -> Option<FallbackChain> {
        let chains = self.chains.read();
        chains.get(model_id).cloned()
    }

    /// Get the next fallback for a model
    ///
    /// If the model has an explicit chain, uses that.
    /// Otherwise, attempts to auto-generate based on profiles.
    pub fn get_next_fallback(
        &self,
        current_model: &str,
        task_class: Option<TaskClass>,
    ) -> Option<String> {
        // First, check explicit chains
        let chains = self.chains.read();
        if let Some(chain) = chains.get(current_model) {
            return chain.next_fallback(current_model).cloned();
        }
        drop(chains);

        // Check if current model is in any chain as a fallback
        let chains = self.chains.read();
        for chain in chains.values() {
            if let Some(next) = chain.next_fallback(current_model) {
                return Some(next.clone());
            }
        }
        drop(chains);

        // Check task-specific chains
        if let Some(task) = task_class {
            let task_chains = self.task_chains.read();
            if let Some(chain) = task_chains.get(&task) {
                if let Some(next) = chain.next_fallback(current_model) {
                    return Some(next.clone());
                }
            }
        }

        // Auto-generate fallback based on profiles
        self.auto_generate_fallback(current_model, task_class)
    }

    /// Get all fallbacks for a model (including from chains it's part of)
    pub fn get_all_fallbacks(&self, model_id: &str) -> Vec<String> {
        let chains = self.chains.read();

        // Check explicit chain
        if let Some(chain) = chains.get(model_id) {
            return chain.fallbacks.clone();
        }

        // Check if model is in any chain
        for chain in chains.values() {
            if chain.primary == model_id {
                return chain.fallbacks.clone();
            }
            if let Some(pos) = chain.fallbacks.iter().position(|m| m == model_id) {
                return chain.fallbacks.iter().skip(pos + 1).cloned().collect();
            }
        }

        Vec::new()
    }

    /// Get health-aware fallbacks (filters out unhealthy models)
    ///
    /// Takes a closure that checks if a model is healthy.
    pub fn get_healthy_fallbacks<F>(&self, model_id: &str, is_healthy: F) -> Vec<String>
    where
        F: Fn(&str) -> bool,
    {
        self.get_all_fallbacks(model_id)
            .into_iter()
            .filter(|m| is_healthy(m))
            .collect()
    }

    /// Get the next healthy fallback
    pub fn get_next_healthy_fallback<F>(
        &self,
        current_model: &str,
        _task_class: Option<TaskClass>,
        is_healthy: F,
    ) -> Option<String>
    where
        F: Fn(&str) -> bool,
    {
        let fallbacks = self.get_all_fallbacks(current_model);

        // Find current position
        let start_idx = if let Some(pos) = fallbacks.iter().position(|m| m == current_model) {
            pos + 1
        } else {
            0
        };

        // Find next healthy
        fallbacks
            .into_iter()
            .skip(start_idx)
            .find(|m| is_healthy(m))
    }

    /// Auto-generate a fallback based on model profiles
    fn auto_generate_fallback(
        &self,
        current_model: &str,
        task_class: Option<TaskClass>,
    ) -> Option<String> {
        let profiles = self.profiles.read();

        let current_profile = profiles.get(current_model)?;

        // Find models that could serve as fallbacks
        let mut candidates: Vec<(&String, f32)> = profiles
            .iter()
            .filter(|(id, _)| *id != current_model)
            .map(|(id, profile)| {
                let mut score = 0.0_f32;

                // Same backend type is preferred for consistency
                if profile.backend_id == current_profile.backend_id {
                    score += 10.0;
                }

                // Similar strengths
                for strength in &profile.strengths {
                    if current_profile.strengths.contains(strength) {
                        score += 5.0;
                    }
                }

                // Task class affinity
                if let Some(task) = task_class {
                    score += profile.affinity_for(task) * 20.0;
                }

                // Lower cost tier is better for fallbacks
                score -= f32::from(profile.cost_tier as u8) * 2.0;

                // Faster models are better for fallbacks
                score -= (profile.avg_ttft_ms as f32) / 1000.0;

                (id, score)
            })
            .collect();

        // Sort by score descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        candidates.first().map(|(id, _)| (*id).clone())
    }

    /// Validate a chain for cycles and invalid references
    pub fn validate_chain(&self, chain: &FallbackChain) -> Result<(), FallbackChainError> {
        let mut seen = HashSet::new();
        seen.insert(chain.primary.clone());

        for fallback in &chain.fallbacks {
            // Check for cycles
            if !seen.insert(fallback.clone()) {
                return Err(FallbackChainError::CycleDetected {
                    model: fallback.clone(),
                });
            }

            // Check depth
            if seen.len() > self.max_chain_depth {
                return Err(FallbackChainError::ChainTooDeep {
                    depth: seen.len(),
                    max: self.max_chain_depth,
                });
            }
        }

        Ok(())
    }

    /// Build and validate a complete fallback chain
    pub fn build_chain(
        &self,
        primary: impl Into<String>,
        fallbacks: Vec<String>,
    ) -> Result<FallbackChain, FallbackChainError> {
        let chain = FallbackChain::new(primary, fallbacks);
        self.validate_chain(&chain)?;
        Ok(chain)
    }

    /// Remove a fallback chain
    pub fn remove_chain(&self, model_id: &str) {
        let mut chains = self.chains.write();
        chains.remove(model_id);
    }

    /// Get all registered chains
    pub fn all_chains(&self) -> Vec<FallbackChain> {
        let chains = self.chains.read();
        chains.values().cloned().collect()
    }

    /// Clear all chains
    pub fn clear(&self) {
        let mut chains = self.chains.write();
        chains.clear();
        let mut task_chains = self.task_chains.write();
        task_chains.clear();
    }

    /// Get statistics about fallback chains
    pub fn stats(&self) -> FallbackStats {
        let chains = self.chains.read();
        let task_chains = self.task_chains.read();
        let profiles = self.profiles.read();

        let total_chains = chains.len();
        let total_task_chains = task_chains.len();
        let avg_depth = if total_chains > 0 {
            chains.values().map(FallbackChain::depth).sum::<usize>() as f64 / total_chains as f64
        } else {
            0.0
        };
        let max_depth = chains.values().map(FallbackChain::depth).max().unwrap_or(0);

        FallbackStats {
            total_chains,
            total_task_chains,
            total_profiles: profiles.len(),
            average_chain_depth: avg_depth,
            max_chain_depth: max_depth,
        }
    }
}

impl Default for FallbackChainManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Errors and Stats
// ============================================================================

/// Errors that can occur during fallback chain operations
#[derive(Clone, Debug)]
pub enum FallbackChainError {
    /// Cycle detected in fallback chain
    CycleDetected { model: String },

    /// Chain exceeds maximum depth
    ChainTooDeep { depth: usize, max: usize },

    /// Model not found
    ModelNotFound { model: String },

    /// Invalid chain configuration
    InvalidChain { reason: String },
}

impl std::fmt::Display for FallbackChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CycleDetected { model } => {
                write!(f, "Cycle detected in fallback chain at model: {model}")
            }
            Self::ChainTooDeep { depth, max } => {
                write!(f, "Chain depth {depth} exceeds maximum {max}")
            }
            Self::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            Self::InvalidChain { reason } => {
                write!(f, "Invalid fallback chain: {reason}")
            }
        }
    }
}

impl std::error::Error for FallbackChainError {}

/// Statistics about fallback chains
#[derive(Clone, Debug)]
pub struct FallbackStats {
    /// Total number of explicit chains
    pub total_chains: usize,

    /// Total number of task-specific chains
    pub total_task_chains: usize,

    /// Total number of registered profiles
    pub total_profiles: usize,

    /// Average chain depth
    pub average_chain_depth: f64,

    /// Maximum chain depth
    pub max_chain_depth: usize,
}

// ============================================================================
// Fallback Context
// ============================================================================

/// Context for tracking fallback attempts during a request
#[derive(Clone, Debug)]
pub struct FallbackContext {
    /// Original model that was requested
    pub original_model: String,

    /// Models that have been tried (in order)
    pub tried_models: Vec<String>,

    /// Current model being used
    pub current_model: String,

    /// Whether we're currently on a fallback
    pub is_fallback: bool,

    /// Task class for this request
    pub task_class: Option<TaskClass>,
}

impl FallbackContext {
    /// Create a new fallback context
    pub fn new(model: impl Into<String>) -> Self {
        let model = model.into();
        Self {
            original_model: model.clone(),
            tried_models: vec![model.clone()],
            current_model: model,
            is_fallback: false,
            task_class: None,
        }
    }

    /// Create with task class
    pub fn with_task_class(model: impl Into<String>, task_class: TaskClass) -> Self {
        let mut ctx = Self::new(model);
        ctx.task_class = Some(task_class);
        ctx
    }

    /// Record a fallback to a new model
    pub fn fallback_to(&mut self, model: impl Into<String>) {
        let model = model.into();
        self.tried_models.push(model.clone());
        self.current_model = model;
        self.is_fallback = true;
    }

    /// Check if a model has already been tried
    #[must_use]
    pub fn has_tried(&self, model: &str) -> bool {
        self.tried_models.iter().any(|m| m == model)
    }

    /// Get the number of fallback attempts
    #[must_use]
    pub fn fallback_count(&self) -> usize {
        self.tried_models.len().saturating_sub(1)
    }

    /// Check if we're still on the original model
    #[must_use]
    pub fn is_original(&self) -> bool {
        self.current_model == self.original_model
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_chain_next() {
        let chain = FallbackChain::new(
            "primary",
            vec![
                "fallback1".to_string(),
                "fallback2".to_string(),
                "fallback3".to_string(),
            ],
        );

        assert_eq!(
            chain.next_fallback("primary"),
            Some(&"fallback1".to_string())
        );
        assert_eq!(
            chain.next_fallback("fallback1"),
            Some(&"fallback2".to_string())
        );
        assert_eq!(
            chain.next_fallback("fallback2"),
            Some(&"fallback3".to_string())
        );
        assert_eq!(chain.next_fallback("fallback3"), None);
        assert_eq!(chain.next_fallback("unknown"), None);
    }

    #[test]
    fn test_fallback_chain_remaining() {
        let chain = FallbackChain::new(
            "primary",
            vec!["fb1".to_string(), "fb2".to_string(), "fb3".to_string()],
        );

        let remaining = chain.remaining_fallbacks("primary");
        assert_eq!(remaining.len(), 3);

        let remaining = chain.remaining_fallbacks("fb1");
        assert_eq!(remaining.len(), 2);
        assert_eq!(*remaining[0], "fb2");

        let remaining = chain.remaining_fallbacks("fb3");
        assert_eq!(remaining.len(), 0);
    }

    #[test]
    fn test_fallback_chain_contains() {
        let chain = FallbackChain::new("primary", vec!["fb1".to_string(), "fb2".to_string()]);

        assert!(chain.contains("primary"));
        assert!(chain.contains("fb1"));
        assert!(chain.contains("fb2"));
        assert!(!chain.contains("unknown"));
    }

    #[test]
    fn test_manager_set_and_get_chain() {
        let manager = FallbackChainManager::new();

        manager.set_chain("gpt-4", vec!["claude-3".to_string(), "gpt-3.5".to_string()]);

        let chain = manager.get_chain("gpt-4").unwrap();
        assert_eq!(chain.primary, "gpt-4");
        assert_eq!(chain.fallbacks.len(), 2);
        assert_eq!(chain.fallbacks[0], "claude-3");
    }

    #[test]
    fn test_manager_get_next_fallback() {
        let manager = FallbackChainManager::new();

        manager.set_chain("gpt-4", vec!["claude-3".to_string(), "gpt-3.5".to_string()]);

        let next = manager.get_next_fallback("gpt-4", None);
        assert_eq!(next, Some("claude-3".to_string()));
    }

    #[test]
    fn test_manager_get_all_fallbacks() {
        let manager = FallbackChainManager::new();

        manager.set_chain(
            "model-a",
            vec![
                "model-b".to_string(),
                "model-c".to_string(),
                "model-d".to_string(),
            ],
        );

        let fallbacks = manager.get_all_fallbacks("model-a");
        assert_eq!(fallbacks.len(), 3);

        // When starting from a fallback, should get remaining
        let fallbacks = manager.get_all_fallbacks("model-b");
        assert_eq!(fallbacks.len(), 2);
        assert!(fallbacks.contains(&"model-c".to_string()));
        assert!(fallbacks.contains(&"model-d".to_string()));
    }

    #[test]
    fn test_manager_healthy_fallbacks() {
        let manager = FallbackChainManager::new();

        manager.set_chain(
            "primary",
            vec![
                "healthy1".to_string(),
                "unhealthy".to_string(),
                "healthy2".to_string(),
            ],
        );

        let is_healthy = |model: &str| model != "unhealthy";

        let healthy = manager.get_healthy_fallbacks("primary", is_healthy);
        assert_eq!(healthy.len(), 2);
        assert!(healthy.contains(&"healthy1".to_string()));
        assert!(healthy.contains(&"healthy2".to_string()));
        assert!(!healthy.contains(&"unhealthy".to_string()));
    }

    #[test]
    fn test_manager_next_healthy_fallback() {
        let manager = FallbackChainManager::new();

        manager.set_chain(
            "primary",
            vec![
                "unhealthy1".to_string(),
                "unhealthy2".to_string(),
                "healthy".to_string(),
            ],
        );

        let is_healthy = |model: &str| model == "healthy";

        let next = manager.get_next_healthy_fallback("primary", None, is_healthy);
        assert_eq!(next, Some("healthy".to_string()));
    }

    #[test]
    fn test_validate_chain_cycle_detection() {
        let manager = FallbackChainManager::new();

        let chain = FallbackChain::new(
            "model-a",
            vec!["model-b".to_string(), "model-a".to_string()], // Cycle!
        );

        let result = manager.validate_chain(&chain);
        assert!(matches!(
            result,
            Err(FallbackChainError::CycleDetected { .. })
        ));
    }

    #[test]
    fn test_validate_chain_depth() {
        let manager = FallbackChainManager::with_max_depth(3);

        let chain = FallbackChain::new(
            "model-a",
            vec![
                "model-b".to_string(),
                "model-c".to_string(),
                "model-d".to_string(),
                "model-e".to_string(),
            ],
        );

        let result = manager.validate_chain(&chain);
        assert!(matches!(
            result,
            Err(FallbackChainError::ChainTooDeep { .. })
        ));
    }

    #[test]
    fn test_fallback_context() {
        let mut ctx = FallbackContext::new("original-model");

        assert!(ctx.is_original());
        assert!(!ctx.is_fallback);
        assert_eq!(ctx.fallback_count(), 0);
        assert!(ctx.has_tried("original-model"));
        assert!(!ctx.has_tried("other-model"));

        ctx.fallback_to("fallback-1");

        assert!(!ctx.is_original());
        assert!(ctx.is_fallback);
        assert_eq!(ctx.fallback_count(), 1);
        assert_eq!(ctx.current_model, "fallback-1");
        assert!(ctx.has_tried("original-model"));
        assert!(ctx.has_tried("fallback-1"));

        ctx.fallback_to("fallback-2");
        assert_eq!(ctx.fallback_count(), 2);
        assert_eq!(ctx.tried_models.len(), 3);
    }

    #[test]
    fn test_task_specific_chains() {
        let manager = FallbackChainManager::new();

        manager.set_task_chain(
            TaskClass::CodeGeneration,
            "code-model",
            vec!["code-fallback-1".to_string(), "code-fallback-2".to_string()],
        );

        manager.set_task_chain(
            TaskClass::QuickResponse,
            "fast-model",
            vec!["fast-fallback".to_string()],
        );

        // Task chains are accessed via get_next_fallback with task_class
        // The behavior depends on how the model relates to the task chain
        let stats = manager.stats();
        assert_eq!(stats.total_task_chains, 2);
    }

    #[test]
    fn test_manager_stats() {
        let manager = FallbackChainManager::new();

        manager.set_chain("model-a", vec!["fb1".to_string(), "fb2".to_string()]);
        manager.set_chain("model-b", vec!["fb3".to_string()]);

        let stats = manager.stats();
        assert_eq!(stats.total_chains, 2);
        assert!(stats.average_chain_depth > 2.0); // (3 + 2) / 2 = 2.5
        assert_eq!(stats.max_chain_depth, 3);
    }

    #[test]
    fn test_manager_clear() {
        let manager = FallbackChainManager::new();

        manager.set_chain("model-a", vec!["fb1".to_string()]);
        manager.set_task_chain(TaskClass::General, "model-b", vec![]);

        manager.clear();

        let stats = manager.stats();
        assert_eq!(stats.total_chains, 0);
        assert_eq!(stats.total_task_chains, 0);
    }

    #[test]
    fn test_auto_generate_fallback() {
        let manager = FallbackChainManager::new();

        // Register some profiles
        let mut profile1 = ModelProfile::new("model-a", "backend-1");
        profile1.strengths = vec![TaskClass::CodeGeneration];
        profile1.avg_ttft_ms = 500;
        manager.register_profile(profile1);

        let mut profile2 = ModelProfile::new("model-b", "backend-1");
        profile2.strengths = vec![TaskClass::CodeGeneration];
        profile2.avg_ttft_ms = 300;
        manager.register_profile(profile2);

        let mut profile3 = ModelProfile::new("model-c", "backend-2");
        profile3.strengths = vec![TaskClass::QuickResponse];
        profile3.avg_ttft_ms = 100;
        manager.register_profile(profile3);

        // Auto-generate should prefer model-b for code tasks (same strength, faster)
        let fallback = manager.get_next_fallback("model-a", Some(TaskClass::CodeGeneration));
        assert!(fallback.is_some());
        // model-b should score higher due to same strengths and faster
        assert_eq!(fallback.unwrap(), "model-b");
    }
}
