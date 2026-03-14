//! Shell hook logic for automatic time tracking.
//!
//! The hook fires on every shell prompt render. It detects the current project
//! from the working directory, manages sessions, and starts/stops timers.

use std::path::Path;

use time::OffsetDateTime;

use crate::error::StintError;
use crate::models::entry::{EntrySource, TimeEntry};
use crate::models::project::ProjectStatus;
use crate::models::session::ShellSession;
use crate::models::types::{EntryId, SessionId};
use crate::storage::Storage;

/// Default idle threshold in seconds (5 minutes).
const IDLE_THRESHOLD_SECS: i64 = 300;

/// Stale session threshold in seconds (1 hour).
const STALE_THRESHOLD_SECS: i64 = 3600;

/// What happened as a result of the hook firing.
#[derive(Debug, PartialEq, Eq)]
pub enum HookAction {
    /// Session heartbeat updated, no project change.
    Heartbeat,
    /// Started tracking a new project.
    Started { project_name: String },
    /// Switched from one project to another.
    Switched { from: String, to: String },
    /// Stopped tracking (left project directory).
    Stopped { project_name: String },
    /// New session created, no project detected.
    SessionCreated,
    /// New session created and started tracking.
    SessionStarted { project_name: String },
    /// Idle gap detected; previous entry stopped at last heartbeat, new one started.
    IdleResume { project_name: String },
}

/// Handles a shell hook invocation.
///
/// Called on every prompt render. Detects the current project from `cwd`,
/// manages the session lifecycle, and starts/stops/switches timers.
pub fn handle_hook(
    storage: &impl Storage,
    pid: u32,
    cwd: &Path,
    shell: Option<&str>,
) -> Result<HookAction, StintError> {
    let now = OffsetDateTime::now_utc();

    match storage.get_session_by_pid(pid)? {
        None => handle_cold_start(storage, pid, cwd, shell, now),
        Some(session) => handle_warm_path(storage, session, cwd, now),
    }
}

/// Handles the first hook call in a new shell session.
fn handle_cold_start(
    storage: &impl Storage,
    pid: u32,
    cwd: &Path,
    shell: Option<&str>,
    now: OffsetDateTime,
) -> Result<HookAction, StintError> {
    // Reap stale sessions opportunistically
    let _ = reap_stale_sessions(storage, now);

    // Detect project from cwd
    let project = storage.get_project_by_path(cwd)?;
    let active_project = project.filter(|p| p.status == ProjectStatus::Active);

    let project_id = active_project.as_ref().map(|p| p.id.clone());

    let session = ShellSession {
        id: SessionId::new(),
        pid,
        shell: shell.map(|s| s.to_string()),
        cwd: cwd.to_path_buf(),
        current_project_id: project_id,
        started_at: now,
        last_heartbeat: now,
        ended_at: None,
    };
    storage.upsert_session(&session)?;

    match active_project {
        Some(project) => {
            // Merge mode: only create entry if none is running for this project
            if storage.get_running_entry(&project.id)?.is_none() {
                let entry = new_hook_entry(&project.id, &session.id, now);
                storage.create_entry(&entry)?;
            }
            Ok(HookAction::SessionStarted {
                project_name: project.name,
            })
        }
        None => Ok(HookAction::SessionCreated),
    }
}

/// Handles subsequent hook calls in an existing session.
fn handle_warm_path(
    storage: &impl Storage,
    mut session: ShellSession,
    cwd: &Path,
    now: OffsetDateTime,
) -> Result<HookAction, StintError> {
    let idle_gap = (now - session.last_heartbeat).whole_seconds();
    let is_idle = idle_gap > IDLE_THRESHOLD_SECS;

    // Detect current project from cwd
    let new_project = storage.get_project_by_path(cwd)?;
    let new_active = new_project.filter(|p| p.status == ProjectStatus::Active);
    let new_project_id = new_active.as_ref().map(|p| p.id.clone());

    let old_project_id = session.current_project_id.clone();
    let project_changed = new_project_id != old_project_id;

    // Handle idle gap: stop old entry at last_heartbeat time
    if is_idle {
        if let Some(ref old_pid) = old_project_id {
            stop_entry_for_project(storage, old_pid, session.last_heartbeat)?;
        }

        // Update session
        session.cwd = cwd.to_path_buf();
        session.current_project_id = new_project_id;
        session.last_heartbeat = now;
        storage.upsert_session(&session)?;

        // Start new entry if we're in a project
        if let Some(project) = new_active {
            if storage.get_running_entry(&project.id)?.is_none() {
                let entry = new_hook_entry(&project.id, &session.id, now);
                storage.create_entry(&entry)?;
            }
            return Ok(HookAction::IdleResume {
                project_name: project.name,
            });
        }
        return Ok(HookAction::Heartbeat);
    }

    // No idle gap, no project change — just heartbeat
    if !project_changed {
        session.cwd = cwd.to_path_buf();
        session.last_heartbeat = now;
        storage.upsert_session(&session)?;
        return Ok(HookAction::Heartbeat);
    }

    // Project changed — stop old, start new
    let old_name = if let Some(ref old_pid) = old_project_id {
        let old_project = storage.get_project(old_pid)?;
        stop_entry_for_project(storage, old_pid, now)?;
        old_project.map(|p| p.name)
    } else {
        None
    };

    session.cwd = cwd.to_path_buf();
    session.current_project_id = new_project_id;
    session.last_heartbeat = now;
    storage.upsert_session(&session)?;

    match (old_name, new_active) {
        (Some(from), Some(to_project)) => {
            if storage.get_running_entry(&to_project.id)?.is_none() {
                let entry = new_hook_entry(&to_project.id, &session.id, now);
                storage.create_entry(&entry)?;
            }
            Ok(HookAction::Switched {
                from,
                to: to_project.name,
            })
        }
        (Some(from), None) => Ok(HookAction::Stopped { project_name: from }),
        (None, Some(to_project)) => {
            if storage.get_running_entry(&to_project.id)?.is_none() {
                let entry = new_hook_entry(&to_project.id, &session.id, now);
                storage.create_entry(&entry)?;
            }
            Ok(HookAction::Started {
                project_name: to_project.name,
            })
        }
        (None, None) => Ok(HookAction::Heartbeat),
    }
}

/// Handles shell exit: ends the session and conditionally stops the timer.
///
/// In merge mode, the entry is only stopped if no other active sessions
/// share the same project.
pub fn handle_hook_exit(storage: &impl Storage, pid: u32) -> Result<(), StintError> {
    let session = match storage.get_session_by_pid(pid)? {
        Some(s) => s,
        None => return Ok(()), // No active session for this PID
    };

    let now = OffsetDateTime::now_utc();

    // End the session
    storage.end_session(&session.id, now)?;

    // In merge mode, only stop the entry if no other sessions share this project
    if let Some(ref project_id) = session.current_project_id {
        let other_sessions = storage.count_active_sessions_for_project(project_id, &session.id)?;
        if other_sessions == 0 {
            stop_entry_for_project(storage, project_id, now)?;
        }
    }

    Ok(())
}

/// Reaps stale sessions whose last heartbeat is older than the threshold.
///
/// Ends all stale sessions first, then stops hook entries only for projects
/// with no remaining active sessions (preserving merge mode invariant).
/// Returns the number of sessions reaped.
pub fn reap_stale_sessions(
    storage: &impl Storage,
    now: OffsetDateTime,
) -> Result<usize, StintError> {
    let threshold = now - time::Duration::seconds(STALE_THRESHOLD_SECS);
    let stale = storage.get_stale_sessions(threshold)?;
    let count = stale.len();

    if count == 0 {
        return Ok(0);
    }

    // Group by project_id, tracking the max last_heartbeat per project
    let mut project_max_heartbeat: std::collections::HashMap<
        String,
        (crate::models::types::ProjectId, OffsetDateTime),
    > = std::collections::HashMap::new();

    // End all stale sessions first
    for session in &stale {
        if let Some(ref project_id) = session.current_project_id {
            let key = project_id.as_str().to_owned();
            project_max_heartbeat
                .entry(key)
                .and_modify(|(_, max_hb)| {
                    if session.last_heartbeat > *max_hb {
                        *max_hb = session.last_heartbeat;
                    }
                })
                .or_insert((project_id.clone(), session.last_heartbeat));
        }
        storage.end_session(&session.id, session.last_heartbeat)?;
    }

    // Stop entries only for projects with no remaining active sessions
    for (project_id, max_heartbeat) in project_max_heartbeat.values() {
        // Use a dummy session ID that won't match anything to count all active sessions
        let dummy_id = SessionId::new();
        let active_count = storage.count_active_sessions_for_project(project_id, &dummy_id)?;
        if active_count == 0 {
            stop_entry_for_project(storage, project_id, *max_heartbeat)?;
        }
    }

    Ok(count)
}

/// Stops the running hook-sourced entry for a project.
///
/// Only stops entries with `source: Hook`. Manual entries are left untouched
/// so that `stint start`/`stint stop` are not interfered with by the hook.
fn stop_entry_for_project(
    storage: &impl Storage,
    project_id: &crate::models::types::ProjectId,
    end_time: OffsetDateTime,
) -> Result<(), StintError> {
    if let Some(mut entry) = storage.get_running_hook_entry(project_id)? {
        entry.end = Some(end_time);
        entry.duration_secs = Some((end_time - entry.start).whole_seconds());
        entry.updated_at = end_time;
        storage.update_entry(&entry)?;
    }
    Ok(())
}

/// Creates a new hook-sourced time entry.
fn new_hook_entry(
    project_id: &crate::models::types::ProjectId,
    session_id: &SessionId,
    now: OffsetDateTime,
) -> TimeEntry {
    TimeEntry {
        id: EntryId::new(),
        project_id: project_id.clone(),
        session_id: Some(session_id.clone()),
        start: now,
        end: None,
        duration_secs: None,
        source: EntrySource::Hook,
        notes: None,
        tags: vec![],
        created_at: now,
        updated_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::{Project, ProjectStatus};
    use crate::models::types::ProjectId;
    use crate::storage::sqlite::SqliteStorage;
    use crate::storage::Storage;
    use std::path::PathBuf;

    fn setup() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn create_project(storage: &SqliteStorage, name: &str, path: &str) {
        let now = OffsetDateTime::now_utc();
        let project = Project {
            id: ProjectId::new(),
            name: name.to_string(),
            paths: vec![PathBuf::from(path)],
            tags: vec![],
            hourly_rate_cents: None,
            status: ProjectStatus::Active,
            created_at: now,
            updated_at: now,
        };
        storage.create_project(&project).unwrap();
    }

    #[test]
    fn cold_start_in_project_creates_session_and_entry() {
        let storage = setup();
        create_project(&storage, "my-app", "/home/user/my-app");

        let action = handle_hook(&storage, 1234, Path::new("/home/user/my-app/src"), None).unwrap();

        assert!(matches!(action, HookAction::SessionStarted { .. }));

        // Session exists
        let session = storage.get_session_by_pid(1234).unwrap().unwrap();
        assert!(session.current_project_id.is_some());

        // Entry exists and is running
        let entry = storage.get_any_running_entry().unwrap().unwrap();
        assert_eq!(entry.source, EntrySource::Hook);
    }

    #[test]
    fn cold_start_outside_project_creates_session_only() {
        let storage = setup();
        create_project(&storage, "my-app", "/home/user/my-app");

        let action = handle_hook(&storage, 1234, Path::new("/home/user/other"), None).unwrap();

        assert_eq!(action, HookAction::SessionCreated);

        let session = storage.get_session_by_pid(1234).unwrap().unwrap();
        assert!(session.current_project_id.is_none());
        assert!(storage.get_any_running_entry().unwrap().is_none());
    }

    #[test]
    fn warm_path_same_cwd_is_heartbeat() {
        let storage = setup();
        create_project(&storage, "my-app", "/home/user/my-app");

        handle_hook(&storage, 1234, Path::new("/home/user/my-app"), None).unwrap();
        let action = handle_hook(&storage, 1234, Path::new("/home/user/my-app"), None).unwrap();

        assert_eq!(action, HookAction::Heartbeat);
    }

    #[test]
    fn cwd_change_to_different_project_switches() {
        let storage = setup();
        create_project(&storage, "app-1", "/home/user/app-1");
        create_project(&storage, "app-2", "/home/user/app-2");

        handle_hook(&storage, 1234, Path::new("/home/user/app-1"), None).unwrap();
        let action = handle_hook(&storage, 1234, Path::new("/home/user/app-2"), None).unwrap();

        assert!(
            matches!(action, HookAction::Switched { from, to } if from == "app-1" && to == "app-2")
        );

        // Old entry should be stopped
        let app1 = storage.get_project_by_name("app-1").unwrap().unwrap();
        assert!(storage.get_running_entry(&app1.id).unwrap().is_none());

        // New entry should be running
        let app2 = storage.get_project_by_name("app-2").unwrap().unwrap();
        assert!(storage.get_running_entry(&app2.id).unwrap().is_some());
    }

    #[test]
    fn cwd_change_to_non_project_stops() {
        let storage = setup();
        create_project(&storage, "my-app", "/home/user/my-app");

        handle_hook(&storage, 1234, Path::new("/home/user/my-app"), None).unwrap();
        let action = handle_hook(&storage, 1234, Path::new("/home/user/other"), None).unwrap();

        assert!(matches!(action, HookAction::Stopped { .. }));
        assert!(storage.get_any_running_entry().unwrap().is_none());
    }

    #[test]
    fn cwd_change_from_non_project_to_project_starts() {
        let storage = setup();
        create_project(&storage, "my-app", "/home/user/my-app");

        handle_hook(&storage, 1234, Path::new("/home/user/other"), None).unwrap();
        let action = handle_hook(&storage, 1234, Path::new("/home/user/my-app"), None).unwrap();

        assert!(matches!(action, HookAction::Started { .. }));
        assert!(storage.get_any_running_entry().unwrap().is_some());
    }

    #[test]
    fn manual_start_is_not_duplicated_by_hook() {
        let storage = setup();
        create_project(&storage, "my-app", "/home/user/my-app");

        // Manually start a timer
        let project = storage.get_project_by_name("my-app").unwrap().unwrap();
        let now = OffsetDateTime::now_utc();
        let manual_entry = TimeEntry {
            id: EntryId::new(),
            project_id: project.id.clone(),
            session_id: None,
            start: now,
            end: None,
            duration_secs: None,
            source: EntrySource::Manual,
            notes: None,
            tags: vec![],
            created_at: now,
            updated_at: now,
        };
        storage.create_entry(&manual_entry).unwrap();

        // Hook fires in the same project directory
        handle_hook(&storage, 1234, Path::new("/home/user/my-app"), None).unwrap();

        // Should still be only one running entry (the manual one)
        let filter = crate::models::entry::EntryFilter::default();
        let entries = storage.list_entries(&filter).unwrap();
        let running: Vec<_> = entries.iter().filter(|e| e.is_running()).collect();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].source, EntrySource::Manual);
    }

    #[test]
    fn archived_project_is_not_tracked() {
        let storage = setup();
        create_project(&storage, "old-app", "/home/user/old-app");

        // Archive the project
        let mut project = storage.get_project_by_name("old-app").unwrap().unwrap();
        project.status = ProjectStatus::Archived;
        project.updated_at = OffsetDateTime::now_utc();
        storage.update_project(&project).unwrap();

        let action = handle_hook(&storage, 1234, Path::new("/home/user/old-app"), None).unwrap();

        assert_eq!(action, HookAction::SessionCreated);
        assert!(storage.get_any_running_entry().unwrap().is_none());
    }

    #[test]
    fn exit_ends_session_and_stops_entry() {
        let storage = setup();
        create_project(&storage, "my-app", "/home/user/my-app");

        handle_hook(&storage, 1234, Path::new("/home/user/my-app"), None).unwrap();
        assert!(storage.get_any_running_entry().unwrap().is_some());

        handle_hook_exit(&storage, 1234).unwrap();

        // Session should be ended
        assert!(storage.get_session_by_pid(1234).unwrap().is_none());

        // Entry should be stopped
        assert!(storage.get_any_running_entry().unwrap().is_none());
    }

    #[test]
    fn exit_in_merge_mode_keeps_entry_if_other_sessions() {
        let storage = setup();
        create_project(&storage, "my-app", "/home/user/my-app");

        // Two shells in the same project
        handle_hook(&storage, 1111, Path::new("/home/user/my-app"), None).unwrap();
        handle_hook(&storage, 2222, Path::new("/home/user/my-app"), None).unwrap();

        // Only one running entry (merge mode)
        let filter = crate::models::entry::EntryFilter::default();
        let entries = storage.list_entries(&filter).unwrap();
        let running: Vec<_> = entries.iter().filter(|e| e.is_running()).collect();
        assert_eq!(running.len(), 1);

        // First shell exits
        handle_hook_exit(&storage, 1111).unwrap();

        // Entry should still be running (shell 2222 still active)
        assert!(storage.get_any_running_entry().unwrap().is_some());

        // Second shell exits
        handle_hook_exit(&storage, 2222).unwrap();

        // Now the entry should be stopped
        assert!(storage.get_any_running_entry().unwrap().is_none());
    }

    #[test]
    fn exit_with_no_session_is_noop() {
        let storage = setup();
        // Should not error
        handle_hook_exit(&storage, 9999).unwrap();
    }

    #[test]
    fn stale_session_reaping() {
        let storage = setup();
        create_project(&storage, "my-app", "/home/user/my-app");

        // Create a session with an old heartbeat
        let old_time = OffsetDateTime::now_utc() - time::Duration::hours(2);
        let project = storage.get_project_by_name("my-app").unwrap().unwrap();

        let session = ShellSession {
            id: SessionId::new(),
            pid: 5555,
            shell: Some("bash".to_string()),
            cwd: PathBuf::from("/home/user/my-app"),
            current_project_id: Some(project.id.clone()),
            started_at: old_time,
            last_heartbeat: old_time,
            ended_at: None,
        };
        storage.upsert_session(&session).unwrap();

        // Create a running entry for that session
        let entry = new_hook_entry(&project.id, &session.id, old_time);
        storage.create_entry(&entry).unwrap();

        // Reap stale sessions
        let now = OffsetDateTime::now_utc();
        let reaped = reap_stale_sessions(&storage, now).unwrap();
        assert_eq!(reaped, 1);

        // Session should be ended
        assert!(storage.get_session_by_pid(5555).unwrap().is_none());

        // Entry should be stopped at last_heartbeat time
        let stopped = storage.get_entry(&entry.id).unwrap().unwrap();
        assert!(!stopped.is_running());
    }
}
