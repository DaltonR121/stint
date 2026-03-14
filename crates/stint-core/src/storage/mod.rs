//! Storage traits and implementations for Stint.

pub mod error;
pub mod sqlite;

use std::path::Path;
use time::OffsetDateTime;

use crate::models::entry::{EntryFilter, TimeEntry};
use crate::models::project::{Project, ProjectStatus};
use crate::models::session::ShellSession;
use crate::models::types::{EntryId, ProjectId, SessionId};

use self::error::StorageError;

pub use self::sqlite::SqliteStorage;

/// Pluggable storage backend for Stint.
pub trait Storage {
    // --- Projects ---

    /// Creates a new project. Returns error if name already exists.
    fn create_project(&self, project: &Project) -> Result<(), StorageError>;

    /// Retrieves a project by its ID.
    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>, StorageError>;

    /// Retrieves a project by its name (case-insensitive).
    fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, StorageError>;

    /// Finds the project whose registered path is the longest prefix of `path`.
    fn get_project_by_path(&self, path: &Path) -> Result<Option<Project>, StorageError>;

    /// Lists all projects, optionally filtered by status.
    fn list_projects(&self, status: Option<ProjectStatus>) -> Result<Vec<Project>, StorageError>;

    /// Updates an existing project.
    fn update_project(&self, project: &Project) -> Result<(), StorageError>;

    /// Deletes a project and all associated data.
    fn delete_project(&self, id: &ProjectId) -> Result<(), StorageError>;

    // --- Time Entries ---

    /// Creates a new time entry.
    fn create_entry(&self, entry: &TimeEntry) -> Result<(), StorageError>;

    /// Retrieves a time entry by its ID.
    fn get_entry(&self, id: &EntryId) -> Result<Option<TimeEntry>, StorageError>;

    /// Finds the currently running entry for a specific project.
    fn get_running_entry(&self, project_id: &ProjectId) -> Result<Option<TimeEntry>, StorageError>;

    /// Finds the currently running hook-sourced entry for a specific project.
    fn get_running_hook_entry(
        &self,
        project_id: &ProjectId,
    ) -> Result<Option<TimeEntry>, StorageError>;

    /// Finds any currently running entry across all projects.
    fn get_any_running_entry(&self) -> Result<Option<TimeEntry>, StorageError>;

    /// Lists entries matching the given filter.
    fn list_entries(&self, filter: &EntryFilter) -> Result<Vec<TimeEntry>, StorageError>;

    /// Updates an existing time entry.
    fn update_entry(&self, entry: &TimeEntry) -> Result<(), StorageError>;

    /// Deletes a time entry.
    fn delete_entry(&self, id: &EntryId) -> Result<(), StorageError>;

    // --- Sessions ---

    /// Creates or updates a shell session record.
    fn upsert_session(&self, session: &ShellSession) -> Result<(), StorageError>;

    /// Retrieves a session by its ID.
    fn get_session(&self, id: &SessionId) -> Result<Option<ShellSession>, StorageError>;

    /// Finds an active session by shell PID.
    fn get_session_by_pid(&self, pid: u32) -> Result<Option<ShellSession>, StorageError>;

    /// Marks a session as ended.
    fn end_session(&self, id: &SessionId, ended_at: OffsetDateTime) -> Result<(), StorageError>;

    /// Counts active sessions tracking a given project, excluding a specific session.
    fn count_active_sessions_for_project(
        &self,
        project_id: &ProjectId,
        exclude_session_id: &SessionId,
    ) -> Result<usize, StorageError>;

    /// Finds active sessions whose last heartbeat is older than the given time.
    fn get_stale_sessions(
        &self,
        older_than: OffsetDateTime,
    ) -> Result<Vec<ShellSession>, StorageError>;

    // --- Ignored Paths ---

    /// Adds a path to the ignore list for auto-discovery.
    fn add_ignored_path(&self, path: &Path) -> Result<(), StorageError>;

    /// Removes a path from the ignore list.
    fn remove_ignored_path(&self, path: &Path) -> Result<bool, StorageError>;

    /// Checks if a path (or any of its ancestors) is in the ignore list.
    fn is_path_ignored(&self, path: &Path) -> Result<bool, StorageError>;

    /// Lists all ignored paths.
    fn list_ignored_paths(&self) -> Result<Vec<std::path::PathBuf>, StorageError>;
}
