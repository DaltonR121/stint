//! High-level business logic for Stint operations.
//!
//! The service layer wraps the `Storage` trait with validation and business rules,
//! keeping CLI handlers thin.

use time::OffsetDateTime;

use crate::error::StintError;
use crate::models::entry::{EntryFilter, EntrySource, TimeEntry};
use crate::models::project::{Project, ProjectStatus};
use crate::models::types::{EntryId, ProjectId};
use crate::storage::Storage;

/// High-level operations for Stint, wrapping a storage backend.
pub struct StintService<S: Storage> {
    storage: S,
}

impl<S: Storage> StintService<S> {
    /// Creates a new service wrapping the given storage backend.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Returns a reference to the underlying storage.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Looks up an active project by name, returning an error if not found or archived.
    fn resolve_active_project(&self, name: &str) -> Result<Project, StintError> {
        let project = self
            .storage
            .get_project_by_name(name)?
            .ok_or_else(|| StintError::InvalidInput(format!("project '{name}' not found")))?;

        if project.status == ProjectStatus::Archived {
            return Err(StintError::ProjectNotActive(name.to_string()));
        }

        Ok(project)
    }

    /// Starts a timer for the named project.
    ///
    /// Fails if a timer is already running or the project is not found/archived.
    pub fn start_timer(&self, project_name: &str) -> Result<(TimeEntry, Project), StintError> {
        let project = self.resolve_active_project(project_name)?;

        // Check for any running timer across all projects
        if let Some(running) = self.storage.get_any_running_entry()? {
            let running_project = self.storage.get_project(&running.project_id)?;
            let name = running_project
                .map(|p| p.name)
                .unwrap_or_else(|| "unknown".to_string());
            return Err(StintError::TimerAlreadyRunning(name));
        }

        let now = OffsetDateTime::now_utc();
        let entry = TimeEntry {
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

        self.storage.create_entry(&entry)?;
        Ok((entry, project))
    }

    /// Stops the currently running timer.
    ///
    /// Fails if no timer is running.
    pub fn stop_timer(&self) -> Result<(TimeEntry, Project), StintError> {
        let mut entry = self
            .storage
            .get_any_running_entry()?
            .ok_or(StintError::NoRunningTimer)?;

        let now = OffsetDateTime::now_utc();
        entry.end = Some(now);
        entry.duration_secs = Some((now - entry.start).whole_seconds());
        entry.updated_at = now;

        self.storage.update_entry(&entry)?;

        let project = self
            .storage
            .get_project(&entry.project_id)?
            .ok_or_else(|| StintError::InvalidInput("project not found".to_string()))?;

        Ok((entry, project))
    }

    /// Adds a completed time entry retroactively.
    ///
    /// If no date is provided, uses today. The entry is created with `source: Added`.
    pub fn add_time(
        &self,
        project_name: &str,
        duration_secs: i64,
        date: Option<OffsetDateTime>,
        notes: Option<&str>,
    ) -> Result<(TimeEntry, Project), StintError> {
        if duration_secs <= 0 {
            return Err(StintError::InvalidInput(
                "duration must be greater than zero".to_string(),
            ));
        }

        let project = self.resolve_active_project(project_name)?;

        let start = date.unwrap_or_else(|| {
            let now = OffsetDateTime::now_utc();
            now.date().midnight().assume_utc()
        });
        let end = start + time::Duration::seconds(duration_secs);
        let now = OffsetDateTime::now_utc();

        let entry = TimeEntry {
            id: EntryId::new(),
            project_id: project.id.clone(),
            session_id: None,
            start,
            end: Some(end),
            duration_secs: Some(duration_secs),
            source: EntrySource::Added,
            notes: notes.map(|s| s.to_string()),
            tags: vec![],
            created_at: now,
            updated_at: now,
        };

        self.storage.create_entry(&entry)?;
        Ok((entry, project))
    }

    /// Returns the currently running timer and its project, if any.
    pub fn get_status(&self) -> Result<Option<(TimeEntry, Project)>, StintError> {
        let entry = self.storage.get_any_running_entry()?;
        match entry {
            Some(e) => {
                let project = self
                    .storage
                    .get_project(&e.project_id)?
                    .ok_or_else(|| StintError::InvalidInput("project not found".to_string()))?;
                Ok(Some((e, project)))
            }
            None => Ok(None),
        }
    }

    /// Archives a project, hiding it from default listings.
    ///
    /// Stops any running timer for the project first.
    pub fn archive_project(&self, name: &str) -> Result<Project, StintError> {
        let mut project = self
            .storage
            .get_project_by_name(name)?
            .ok_or_else(|| StintError::InvalidInput(format!("project '{name}' not found")))?;

        if project.status == ProjectStatus::Archived {
            return Err(StintError::InvalidInput(format!(
                "project '{name}' is already archived"
            )));
        }

        // Stop any running timer for this project
        if let Some(mut entry) = self.storage.get_running_entry(&project.id)? {
            let now = OffsetDateTime::now_utc();
            entry.end = Some(now);
            entry.duration_secs = Some((now - entry.start).whole_seconds());
            entry.updated_at = now;
            self.storage.update_entry(&entry)?;
        }

        project.status = ProjectStatus::Archived;
        project.updated_at = OffsetDateTime::now_utc();
        self.storage.update_project(&project)?;

        Ok(project)
    }

    /// Deletes a project and all its entries.
    pub fn delete_project(&self, name: &str) -> Result<String, StintError> {
        let project = self
            .storage
            .get_project_by_name(name)?
            .ok_or_else(|| StintError::InvalidInput(format!("project '{name}' not found")))?;

        self.storage.delete_project(&project.id)?;
        Ok(project.name)
    }

    /// Lists entries matching the given filter, enriched with project data.
    pub fn get_entries(
        &self,
        filter: &EntryFilter,
    ) -> Result<Vec<(TimeEntry, Project)>, StintError> {
        let entries = self.storage.list_entries(filter)?;
        let mut results = Vec::with_capacity(entries.len());

        // Cache projects to avoid repeated lookups
        let mut project_cache: std::collections::HashMap<String, Project> =
            std::collections::HashMap::new();

        for entry in entries {
            let pid_str = entry.project_id.as_str().to_owned();
            let project = if let Some(cached) = project_cache.get(&pid_str) {
                cached.clone()
            } else {
                let p = self
                    .storage
                    .get_project(&entry.project_id)?
                    .ok_or_else(|| {
                        StintError::InvalidInput(format!(
                            "project not found for entry {}",
                            entry.id
                        ))
                    })?;
                project_cache.insert(pid_str, p.clone());
                p
            };

            results.push((entry, project));
        }

        Ok(results)
    }

    /// Returns the most recent time entry with its project.
    pub fn get_last_entry(&self) -> Result<Option<(TimeEntry, Project)>, StintError> {
        let filter = EntryFilter::default();
        let entries = self.storage.list_entries(&filter)?;
        // list_entries returns DESC by start, so first is most recent
        match entries.into_iter().next() {
            Some(entry) => {
                let project = self
                    .storage
                    .get_project(&entry.project_id)?
                    .ok_or_else(|| {
                        StintError::InvalidInput("project not found for entry".to_string())
                    })?;
                Ok(Some((entry, project)))
            }
            None => Ok(None),
        }
    }

    /// Deletes a time entry by ID.
    pub fn delete_entry(&self, id: &EntryId) -> Result<(), StintError> {
        self.storage.delete_entry(id)?;
        Ok(())
    }

    /// Updates a time entry.
    pub fn update_entry(&self, entry: &TimeEntry) -> Result<(), StintError> {
        self.storage.update_entry(entry)?;
        Ok(())
    }

    /// Resolves a project name to its ID for use in filters.
    pub fn resolve_project_id(&self, name: &str) -> Result<ProjectId, StintError> {
        let project = self
            .storage
            .get_project_by_name(name)?
            .ok_or_else(|| StintError::InvalidInput(format!("project '{name}' not found")))?;
        Ok(project.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::{Project, ProjectSource};
    use crate::storage::sqlite::SqliteStorage;
    use std::path::PathBuf;

    fn setup() -> StintService<SqliteStorage> {
        let storage = SqliteStorage::open_in_memory().unwrap();
        StintService::new(storage)
    }

    fn create_project(service: &StintService<SqliteStorage>, name: &str) {
        let now = OffsetDateTime::now_utc();
        let project = Project {
            id: ProjectId::new(),
            name: name.to_string(),
            paths: vec![PathBuf::from(format!("/home/user/{name}"))],
            tags: vec![],
            hourly_rate_cents: None,
            status: ProjectStatus::Active,
            source: ProjectSource::Manual,
            created_at: now,
            updated_at: now,
        };
        service.storage().create_project(&project).unwrap();
    }

    #[test]
    fn start_and_stop_timer() {
        let service = setup();
        create_project(&service, "my-app");

        let (entry, project) = service.start_timer("my-app").unwrap();
        assert!(entry.is_running());
        assert_eq!(project.name, "my-app");

        let (stopped, _) = service.stop_timer().unwrap();
        assert!(!stopped.is_running());
        assert!(stopped.duration_secs.unwrap() >= 0);
    }

    #[test]
    fn start_while_running_errors() {
        let service = setup();
        create_project(&service, "app-1");
        create_project(&service, "app-2");

        service.start_timer("app-1").unwrap();
        let result = service.start_timer("app-2");
        assert!(matches!(result, Err(StintError::TimerAlreadyRunning(_))));
    }

    #[test]
    fn stop_without_running_errors() {
        let service = setup();
        let result = service.stop_timer();
        assert!(matches!(result, Err(StintError::NoRunningTimer)));
    }

    #[test]
    fn start_archived_project_errors() {
        let service = setup();
        create_project(&service, "old-app");
        service.archive_project("old-app").unwrap();

        let result = service.start_timer("old-app");
        assert!(matches!(result, Err(StintError::ProjectNotActive(_))));
    }

    #[test]
    fn start_nonexistent_project_errors() {
        let service = setup();
        let result = service.start_timer("no-such-project");
        assert!(matches!(result, Err(StintError::InvalidInput(_))));
    }

    #[test]
    fn add_time_zero_duration_errors() {
        let service = setup();
        create_project(&service, "my-app");

        let result = service.add_time("my-app", 0, None, None);
        assert!(matches!(result, Err(StintError::InvalidInput(_))));
    }

    #[test]
    fn add_time_negative_duration_errors() {
        let service = setup();
        create_project(&service, "my-app");

        let result = service.add_time("my-app", -3600, None, None);
        assert!(matches!(result, Err(StintError::InvalidInput(_))));
    }

    #[test]
    fn add_time() {
        let service = setup();
        create_project(&service, "my-app");

        let (entry, project) = service
            .add_time("my-app", 3600, None, Some("Retroactive"))
            .unwrap();

        assert!(!entry.is_running());
        assert_eq!(entry.duration_secs, Some(3600));
        assert_eq!(entry.source, EntrySource::Added);
        assert_eq!(entry.notes.as_deref(), Some("Retroactive"));
        assert_eq!(project.name, "my-app");
    }

    #[test]
    fn get_status_running() {
        let service = setup();
        create_project(&service, "my-app");
        service.start_timer("my-app").unwrap();

        let status = service.get_status().unwrap();
        assert!(status.is_some());
        let (entry, project) = status.unwrap();
        assert!(entry.is_running());
        assert_eq!(project.name, "my-app");
    }

    #[test]
    fn get_status_idle() {
        let service = setup();
        let status = service.get_status().unwrap();
        assert!(status.is_none());
    }

    #[test]
    fn archive_stops_running_timer() {
        let service = setup();
        create_project(&service, "my-app");
        service.start_timer("my-app").unwrap();

        service.archive_project("my-app").unwrap();

        // Timer should be stopped
        let status = service.get_status().unwrap();
        assert!(status.is_none());

        // Project should be archived
        let project = service
            .storage()
            .get_project_by_name("my-app")
            .unwrap()
            .unwrap();
        assert_eq!(project.status, ProjectStatus::Archived);
    }

    #[test]
    fn delete_project() {
        let service = setup();
        create_project(&service, "doomed");

        let name = service.delete_project("doomed").unwrap();
        assert_eq!(name, "doomed");
        assert!(service
            .storage()
            .get_project_by_name("doomed")
            .unwrap()
            .is_none());
    }

    #[test]
    fn get_entries_with_project() {
        let service = setup();
        create_project(&service, "my-app");
        service.add_time("my-app", 3600, None, None).unwrap();
        service.add_time("my-app", 1800, None, None).unwrap();

        let entries = service.get_entries(&EntryFilter::default()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].1.name, "my-app");
    }
}
