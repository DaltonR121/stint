//! Time entry domain model.

use time::OffsetDateTime;

use super::types::{EntryId, ProjectId, SessionId};

/// How a time entry was created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntrySource {
    /// Created via `stint start` / `stint stop`.
    Manual,
    /// Created automatically by a shell hook.
    Hook,
    /// Added retroactively via `stint add`.
    Added,
}

impl EntrySource {
    /// Returns the source as a lowercase string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Hook => "hook",
            Self::Added => "added",
        }
    }

    /// Parses a source from a stored string value.
    pub fn from_str_value(s: &str) -> Option<Self> {
        match s {
            "manual" => Some(Self::Manual),
            "hook" => Some(Self::Hook),
            "added" => Some(Self::Added),
            _ => None,
        }
    }
}

/// A single time tracking entry.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeEntry {
    /// Unique identifier.
    pub id: EntryId,
    /// The project this entry belongs to.
    pub project_id: ProjectId,
    /// The shell session that created this entry (None for manual/retroactive).
    pub session_id: Option<SessionId>,
    /// When tracking started.
    pub start: OffsetDateTime,
    /// When tracking stopped (None means currently running).
    pub end: Option<OffsetDateTime>,
    /// Duration in seconds. Computed on stop, or set directly for `stint add`.
    pub duration_secs: Option<i64>,
    /// How this entry was created.
    pub source: EntrySource,
    /// Optional notes.
    pub notes: Option<String>,
    /// User-defined tags.
    pub tags: Vec<String>,
    /// When this entry was created.
    pub created_at: OffsetDateTime,
    /// When this entry was last updated.
    pub updated_at: OffsetDateTime,
}

impl TimeEntry {
    /// Returns true if this entry is currently running (no end time).
    pub fn is_running(&self) -> bool {
        self.end.is_none()
    }

    /// Computes the duration from start and end timestamps.
    ///
    /// Returns `duration_secs` if set, otherwise computes from `end - start`.
    /// Returns None if the entry is still running and has no explicit duration.
    pub fn computed_duration_secs(&self) -> Option<i64> {
        if let Some(d) = self.duration_secs {
            return Some(d);
        }
        self.end.map(|end| (end - self.start).whole_seconds())
    }
}

/// Filters for querying time entries.
#[derive(Debug, Default)]
pub struct EntryFilter {
    /// Filter by project.
    pub project_id: Option<ProjectId>,
    /// Include entries starting at or after this time.
    pub from: Option<OffsetDateTime>,
    /// Include entries starting before this time.
    pub to: Option<OffsetDateTime>,
    /// Filter by tags (all must match).
    pub tags: Vec<String>,
    /// Filter by entry source.
    pub source: Option<EntrySource>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn make_entry(
        start: OffsetDateTime,
        end: Option<OffsetDateTime>,
        duration_secs: Option<i64>,
    ) -> TimeEntry {
        TimeEntry {
            id: EntryId::new(),
            project_id: ProjectId::new(),
            session_id: None,
            start,
            end,
            duration_secs,
            source: EntrySource::Manual,
            notes: None,
            tags: vec![],
            created_at: start,
            updated_at: start,
        }
    }

    #[test]
    fn running_entry_has_no_end() {
        let entry = make_entry(datetime!(2026-01-01 9:00 UTC), None, None);
        assert!(entry.is_running());
    }

    #[test]
    fn stopped_entry_is_not_running() {
        let entry = make_entry(
            datetime!(2026-01-01 9:00 UTC),
            Some(datetime!(2026-01-01 10:30 UTC)),
            None,
        );
        assert!(!entry.is_running());
    }

    #[test]
    fn computed_duration_from_timestamps() {
        let entry = make_entry(
            datetime!(2026-01-01 9:00 UTC),
            Some(datetime!(2026-01-01 10:30 UTC)),
            None,
        );
        assert_eq!(entry.computed_duration_secs(), Some(5400)); // 1.5 hours
    }

    #[test]
    fn explicit_duration_takes_precedence() {
        let entry = make_entry(
            datetime!(2026-01-01 0:00 UTC),
            None,
            Some(9000), // 2.5 hours
        );
        assert_eq!(entry.computed_duration_secs(), Some(9000));
    }

    #[test]
    fn running_entry_without_duration_returns_none() {
        let entry = make_entry(datetime!(2026-01-01 9:00 UTC), None, None);
        assert_eq!(entry.computed_duration_secs(), None);
    }
}
