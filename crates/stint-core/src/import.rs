//! One-time data import from CSV files.
//!
//! Supports generic CSV import and Toggl/Clockify export formats.

use std::path::Path;

use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

use crate::error::StintError;
use crate::models::entry::{EntrySource, TimeEntry};
use crate::models::project::{Project, ProjectSource, ProjectStatus};
use crate::models::types::{EntryId, ProjectId};
use crate::storage::Storage;

/// ISO datetime format for CSV parsing: YYYY-MM-DD HH:MM:SS.
const DATETIME_FMT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

/// Result of an import operation.
#[derive(Debug)]
pub struct ImportResult {
    /// Number of entries imported.
    pub entries_imported: usize,
    /// Number of projects created.
    pub projects_created: usize,
    /// Number of rows skipped (errors or duplicates).
    pub rows_skipped: usize,
}

/// Imports time entries from a generic CSV file.
///
/// Expected columns: project, start, end, duration_secs, notes
/// (header row required). Missing optional fields are treated as empty.
pub fn import_csv(storage: &impl Storage, path: &Path) -> Result<ImportResult, StintError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| StintError::InvalidInput(format!("failed to read {}: {e}", path.display())))?;

    let mut lines = contents.lines();
    let header = lines
        .next()
        .ok_or_else(|| StintError::InvalidInput("CSV file is empty".to_string()))?;

    let columns: Vec<String> = header.split(',').map(|c| c.trim().to_lowercase()).collect();

    let col_idx = |name: &str| -> Option<usize> { columns.iter().position(|c| c == name) };

    let project_col = col_idx("project")
        .ok_or_else(|| StintError::InvalidInput("CSV missing 'project' column".to_string()))?;

    let start_col = col_idx("start")
        .ok_or_else(|| StintError::InvalidInput("CSV missing 'start' column".to_string()))?;
    let start_col = Some(start_col); // Keep as Option for consistent field access
    let end_col = col_idx("end");
    let duration_col = col_idx("duration_secs").or_else(|| col_idx("duration"));
    let notes_col = col_idx("notes").or_else(|| col_idx("description"));

    let now = OffsetDateTime::now_utc();
    let mut result = ImportResult {
        entries_imported: 0,
        projects_created: 0,
        rows_skipped: 0,
    };

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        let fields = split_csv_line(line);

        let project_name = match fields.get(project_col) {
            Some(name) if !name.is_empty() => name.as_str(),
            _ => {
                result.rows_skipped += 1;
                continue;
            }
        };

        // Validate row data BEFORE creating any project

        // Parse start time (skip row if missing/unparseable)
        let start = match start_col
            .and_then(|i| fields.get(i))
            .and_then(|s| parse_datetime(s))
        {
            Some(dt) => dt,
            None => {
                result.rows_skipped += 1;
                continue;
            }
        };

        // Parse end time
        let end = end_col
            .and_then(|i| fields.get(i))
            .and_then(|s| parse_datetime(s));

        // Parse duration (reject negative values)
        let duration_secs = duration_col
            .and_then(|i| fields.get(i))
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|&d| d >= 0)
            .or_else(|| end.map(|e| (e - start).whole_seconds()))
            .filter(|&d| d >= 0);

        // Ensure end is set (imported entries should always be completed)
        let end = end.or_else(|| duration_secs.map(|d| start + time::Duration::seconds(d)));

        // Skip rows that can't produce a completed entry or have inverted ranges
        match end {
            Some(e) if e >= start => {}
            _ => {
                result.rows_skipped += 1;
                continue;
            }
        }

        // Parse notes
        let notes = notes_col
            .and_then(|i| fields.get(i))
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        // Row is valid — now find or create the project
        let project = match storage.get_project_by_name(project_name)? {
            Some(p) => p,
            None => {
                let p = Project {
                    id: ProjectId::new(),
                    name: project_name.to_string(),
                    paths: vec![],
                    tags: vec![],
                    hourly_rate_cents: None,
                    status: ProjectStatus::Active,
                    source: ProjectSource::Manual,
                    created_at: now,
                    updated_at: now,
                };
                storage.create_project(&p)?;
                result.projects_created += 1;
                p
            }
        };

        let entry = TimeEntry {
            id: EntryId::new(),
            project_id: project.id.clone(),
            session_id: None,
            start,
            end,
            duration_secs,
            source: EntrySource::Added,
            notes,
            tags: vec![],
            created_at: now,
            updated_at: now,
        };

        match storage.create_entry(&entry) {
            Ok(()) => result.entries_imported += 1,
            Err(crate::storage::error::StorageError::Database(ref e))
                if e.to_string().contains("UNIQUE constraint") =>
            {
                // Duplicate entry (e.g., unique running-per-project constraint) — skip
                result.rows_skipped += 1;
            }
            Err(e) => return Err(e.into()), // Real storage failure — abort
        }
    }

    Ok(result)
}

/// Splits a CSV line respecting quoted fields (RFC 4180).
fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' if !in_quotes && current.is_empty() => {
                in_quotes = true;
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current.trim().to_string());
    fields
}

/// Parses a datetime string in common formats.
fn parse_datetime(s: &str) -> Option<OffsetDateTime> {
    // Try ISO 8601 / RFC 3339
    if let Ok(dt) = OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339) {
        return Some(dt);
    }
    // Try YYYY-MM-DD HH:MM:SS
    if let Ok(pdt) = time::PrimitiveDateTime::parse(s, DATETIME_FMT) {
        return Some(pdt.assume_utc());
    }
    // Try date only: YYYY-MM-DD
    let date_fmt: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]");
    if let Ok(d) = time::Date::parse(s, date_fmt) {
        return Some(d.midnight().assume_utc());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::sqlite::SqliteStorage;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn setup() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    #[test]
    fn import_basic_csv() {
        let storage = setup();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "project,start,end,duration_secs,notes\nmy-app,2026-01-15 09:00:00,2026-01-15 10:30:00,5400,Morning work"
        )
        .unwrap();

        let result = import_csv(&storage, file.path()).unwrap();
        assert_eq!(result.entries_imported, 1);
        assert_eq!(result.projects_created, 1);
        assert_eq!(result.rows_skipped, 0);

        let project = storage.get_project_by_name("my-app").unwrap().unwrap();
        assert_eq!(project.name, "my-app");
    }

    #[test]
    fn import_creates_projects_as_needed() {
        let storage = setup();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "project,start,end,duration_secs\napp-1,2026-01-01 09:00:00,2026-01-01 10:00:00,3600\napp-2,2026-01-01 11:00:00,2026-01-01 11:30:00,1800\napp-1,2026-01-01 14:00:00,2026-01-01 14:15:00,900"
        )
        .unwrap();

        let result = import_csv(&storage, file.path()).unwrap();
        assert_eq!(result.entries_imported, 3);
        assert_eq!(result.projects_created, 2);
    }

    #[test]
    fn import_skips_empty_project() {
        let storage = setup();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "project,start,duration_secs\n,2026-01-01 09:00:00,3600\nmy-app,2026-01-01 10:00:00,1800").unwrap();

        let result = import_csv(&storage, file.path()).unwrap();
        assert_eq!(result.entries_imported, 1);
        assert_eq!(result.rows_skipped, 1);
    }

    #[test]
    fn import_empty_file_errors() {
        let storage = setup();
        let file = NamedTempFile::new().unwrap();

        let result = import_csv(&storage, file.path());
        assert!(result.is_err());
    }

    #[test]
    fn parse_datetime_formats() {
        assert!(parse_datetime("2026-01-15 09:00:00").is_some());
        assert!(parse_datetime("2026-01-15T09:00:00Z").is_some());
        assert!(parse_datetime("2026-01-15").is_some());
        assert!(parse_datetime("garbage").is_none());
    }
}
