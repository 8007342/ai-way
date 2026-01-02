#![allow(missing_docs)]
//! Multi-Model Query Routing
//!
//! High-performance routing infrastructure for concurrent model requests.
//! Supports multiple backend types, connection pooling, and intelligent
//! request routing based on task characteristics.
//!
//! # Architecture
//!
//! ```text
//! +------------------+
//! |   QueryRouter    |  <-- Entry point for all model requests
//! +--------+---------+
//!          |
//!          v
//! +------------------+
//! |  RoutingPolicy   |  <-- Decides which model/backend to use
//! +--------+---------+
//!          |
//!          v
//! +------------------+
//! |  ConnectionPool  |  <-- Manages backend connections
//! +--------+---------+
//!          |
//!    +-----+-----+
//!    |     |     |
//!    v     v     v
//! +----+ +----+ +----+
//! |HTTP| |gRPC| |Local|  <-- Backend transports
//! +----+ +----+ +----+
//! ```
//!
//! # Design Principles
//!
//! 1. **Latency-Aware Routing**: Different task types have different latency budgets
//! 2. **Resource Isolation**: Local models don't starve cloud API requests
//! 3. **Graceful Degradation**: Fallback chains when primary models fail
//! 4. **Observability**: Rich metrics for monitoring and debugging

pub mod backends;
pub mod config;
pub mod connection_pool;
pub mod fallback;
pub mod health;
pub mod metrics;
pub mod policy;
pub mod router;
pub mod semaphore;

#[cfg(test)]
pub mod test_utils;

pub use config::*;
pub use fallback::*;
pub use health::*;
pub use router::*;
