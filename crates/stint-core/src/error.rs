//! Top-level error types for Stint.

use crate::storage::error::StorageError;

/// Errors that can occur in Stint operations.
#[derive(Debug, thiserror::Error)]
pub enum StintError {
    /// An error originating from the storage layer.
    #[error(transparent)]
    Storage(#[from] StorageError),

    /// The caller provided invalid input.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// A timer is already running for the given project.
    #[error("timer already running for project {0}")]
    TimerAlreadyRunning(String),

    /// No timer is currently running.
    #[error("no timer is currently running")]
    NoRunningTimer,

    /// The project is archived and cannot be used for tracking.
    #[error("project '{0}' is archived")]
    ProjectNotActive(String),
}
