//! Shell session domain model.

use std::path::PathBuf;
use time::OffsetDateTime;

use super::types::{ProjectId, SessionId};

/// A shell session tracked by the hook.
///
/// Each terminal gets a session record. The hook updates `last_heartbeat`
/// on every prompt render and sets `ended_at` when the shell exits.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellSession {
    /// Unique identifier.
    pub id: SessionId,
    /// Shell process PID.
    pub pid: u32,
    /// Shell type (e.g., "bash", "zsh", "fish").
    pub shell: Option<String>,
    /// Last known working directory.
    pub cwd: PathBuf,
    /// The project currently active in this session.
    pub current_project_id: Option<ProjectId>,
    /// When this session started.
    pub started_at: OffsetDateTime,
    /// Last heartbeat from the shell hook.
    pub last_heartbeat: OffsetDateTime,
    /// When this session ended (None if still active).
    pub ended_at: Option<OffsetDateTime>,
}
