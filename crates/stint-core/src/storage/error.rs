//! Storage layer error types.

/// Errors that can occur in the storage layer.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The requested project was not found.
    #[error("project not found: {0}")]
    ProjectNotFound(String),

    /// The requested time entry was not found.
    #[error("entry not found: {0}")]
    EntryNotFound(String),

    /// The requested session was not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// A project with this name already exists.
    #[error("project name already exists: {0}")]
    DuplicateProjectName(String),

    /// An error from the underlying database.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// A schema migration failed.
    #[error("migration error: {0}")]
    Migration(String),
}
