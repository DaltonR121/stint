//! Report generation and formatting.

use serde::Serialize;

use crate::duration::format_duration_human;
use crate::models::entry::TimeEntry;
use crate::models::project::Project;

/// How to group report entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupBy {
    /// Group by project name.
    Project,
    /// Group by tag (entries with multiple tags appear in multiple groups).
    Tag,
}

impl GroupBy {
    /// Parses a group-by string.
    pub fn from_str_value(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "project" => Ok(Self::Project),
            "tag" => Ok(Self::Tag),
            _ => Err(format!("unknown group-by: '{s}' (use 'project' or 'tag')")),
        }
    }
}

/// Output format for reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportFormat {
    /// Markdown table.
    Markdown,
    /// Comma-separated values.
    Csv,
    /// JSON array.
    Json,
}

impl ReportFormat {
    /// Parses a format string.
    pub fn from_str_value(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(Self::Markdown),
            "csv" => Ok(Self::Csv),
            "json" => Ok(Self::Json),
            _ => Err(format!(
                "unknown format: '{s}' (use 'markdown', 'csv', or 'json')"
            )),
        }
    }
}

/// A single row in a report.
#[derive(Debug, Clone, Serialize)]
pub struct ReportRow {
    /// Group label (project name or tag).
    pub group: String,
    /// Total duration in seconds.
    pub total_secs: i64,
    /// Number of entries in this group.
    pub entry_count: usize,
    /// Earnings in cents (if hourly rate is set).
    pub earnings_cents: Option<i64>,
}

/// Generates grouped report rows from entries.
pub fn generate_report(entries: &[(TimeEntry, Project)], group_by: &GroupBy) -> Vec<ReportRow> {
    let mut groups: std::collections::BTreeMap<String, (i64, usize, Option<i64>)> =
        std::collections::BTreeMap::new();

    for (entry, project) in entries {
        let duration = entry.computed_duration_secs().unwrap_or(0);

        match group_by {
            GroupBy::Project => {
                let group =
                    groups
                        .entry(project.name.clone())
                        .or_insert((0, 0, project.hourly_rate_cents));
                group.0 += duration;
                group.1 += 1;
            }
            GroupBy::Tag => {
                if entry.tags.is_empty() {
                    let group = groups
                        .entry("(untagged)".to_string())
                        .or_insert((0, 0, None));
                    group.0 += duration;
                    group.1 += 1;
                } else {
                    for tag in &entry.tags {
                        let group = groups.entry(tag.clone()).or_insert((0, 0, None));
                        group.0 += duration;
                        group.1 += 1;
                    }
                }
            }
        }
    }

    groups
        .into_iter()
        .map(|(group, (total_secs, entry_count, rate))| {
            let earnings_cents = rate.map(|r| total_secs * r / 3600);
            ReportRow {
                group,
                total_secs,
                entry_count,
                earnings_cents,
            }
        })
        .collect()
}

/// Formats report rows into the specified output format.
pub fn format_report(rows: &[ReportRow], format: &ReportFormat) -> String {
    match format {
        ReportFormat::Markdown => format_markdown(rows),
        ReportFormat::Csv => format_csv(rows),
        ReportFormat::Json => format_json(rows),
    }
}

/// Renders rows as a Markdown table.
fn format_markdown(rows: &[ReportRow]) -> String {
    let mut out = String::new();
    out.push_str("| Group | Time | Entries | Earnings |\n");
    out.push_str("|-------|------|---------|----------|\n");

    let mut total_secs = 0i64;
    let mut total_entries = 0usize;

    for row in rows {
        let earnings = match row.earnings_cents {
            Some(c) => format!("${}.{:02}", c / 100, c % 100),
            None => "\u{2014}".to_string(), // em dash
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            row.group,
            format_duration_human(row.total_secs),
            row.entry_count,
            earnings,
        ));
        total_secs += row.total_secs;
        total_entries += row.entry_count;
    }

    out.push_str(&format!(
        "| **Total** | **{}** | **{}** | |\n",
        format_duration_human(total_secs),
        total_entries,
    ));

    out
}

/// Renders rows as CSV.
fn format_csv(rows: &[ReportRow]) -> String {
    let mut out = String::from("group,time_secs,time_human,entries,earnings_cents\n");
    for row in rows {
        let earnings = row
            .earnings_cents
            .map(|c| c.to_string())
            .unwrap_or_default();
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            row.group,
            row.total_secs,
            format_duration_human(row.total_secs),
            row.entry_count,
            earnings,
        ));
    }
    out
}

/// Renders rows as JSON.
fn format_json(rows: &[ReportRow]) -> String {
    serde_json::to_string_pretty(rows).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::entry::{EntrySource, TimeEntry};
    use crate::models::project::{Project, ProjectStatus};
    use crate::models::types::{EntryId, ProjectId};
    use std::path::PathBuf;
    use time::OffsetDateTime;

    fn make_entry(project: &Project, duration: i64, tags: Vec<&str>) -> (TimeEntry, Project) {
        let now = OffsetDateTime::now_utc();
        let entry = TimeEntry {
            id: EntryId::new(),
            project_id: project.id.clone(),
            session_id: None,
            start: now,
            end: Some(now + time::Duration::seconds(duration)),
            duration_secs: Some(duration),
            source: EntrySource::Manual,
            notes: None,
            tags: tags.into_iter().map(String::from).collect(),
            created_at: now,
            updated_at: now,
        };
        (entry, project.clone())
    }

    fn make_project(name: &str, rate: Option<i64>) -> Project {
        let now = OffsetDateTime::now_utc();
        Project {
            id: ProjectId::new(),
            name: name.to_string(),
            paths: vec![PathBuf::from(format!("/home/user/{name}"))],
            tags: vec![],
            hourly_rate_cents: rate,
            status: ProjectStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn group_by_project() {
        let p1 = make_project("app-1", Some(15000));
        let p2 = make_project("app-2", None);

        let entries = vec![
            make_entry(&p1, 3600, vec![]),
            make_entry(&p1, 1800, vec![]),
            make_entry(&p2, 7200, vec![]),
        ];

        let rows = generate_report(&entries, &GroupBy::Project);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].group, "app-1");
        assert_eq!(rows[0].total_secs, 5400);
        assert_eq!(rows[0].entry_count, 2);
        assert_eq!(rows[0].earnings_cents, Some(22500)); // 1.5h * $150
        assert_eq!(rows[1].group, "app-2");
        assert_eq!(rows[1].earnings_cents, None);
    }

    #[test]
    fn group_by_tag() {
        let p = make_project("app", None);
        let entries = vec![
            make_entry(&p, 3600, vec!["frontend", "client"]),
            make_entry(&p, 1800, vec!["frontend"]),
            make_entry(&p, 900, vec![]),
        ];

        let rows = generate_report(&entries, &GroupBy::Tag);
        assert_eq!(rows.len(), 3); // (untagged), client, frontend

        let untagged = rows.iter().find(|r| r.group == "(untagged)").unwrap();
        assert_eq!(untagged.total_secs, 900);

        let frontend = rows.iter().find(|r| r.group == "frontend").unwrap();
        assert_eq!(frontend.total_secs, 5400); // 3600 + 1800

        let client = rows.iter().find(|r| r.group == "client").unwrap();
        assert_eq!(client.total_secs, 3600);
    }

    #[test]
    fn format_csv_output() {
        let rows = vec![ReportRow {
            group: "app".to_string(),
            total_secs: 5400,
            entry_count: 2,
            earnings_cents: Some(22500),
        }];
        let csv = format_report(&rows, &ReportFormat::Csv);
        assert!(csv.contains("group,time_secs,time_human,entries,earnings_cents"));
        assert!(csv.contains("app,5400,1h 30m,2,22500"));
    }

    #[test]
    fn format_json_output() {
        let rows = vec![ReportRow {
            group: "app".to_string(),
            total_secs: 3600,
            entry_count: 1,
            earnings_cents: None,
        }];
        let json = format_report(&rows, &ReportFormat::Json);
        assert!(json.contains("\"group\": \"app\""));
        assert!(json.contains("\"total_secs\": 3600"));
        assert!(json.contains("\"earnings_cents\": null"));
    }

    #[test]
    fn format_markdown_output() {
        let rows = vec![ReportRow {
            group: "app".to_string(),
            total_secs: 3600,
            entry_count: 1,
            earnings_cents: Some(15000),
        }];
        let md = format_report(&rows, &ReportFormat::Markdown);
        assert!(md.contains("| app | 1h | 1 | $150.00 |"));
        assert!(md.contains("| **Total**"));
    }
}
