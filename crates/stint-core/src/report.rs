//! Report generation and formatting.

use std::collections::HashSet;

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
    /// Plain-text aligned table for terminal display.
    Table,
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
            "table" => Ok(Self::Table),
            "markdown" | "md" => Ok(Self::Markdown),
            "csv" => Ok(Self::Csv),
            "json" => Ok(Self::Json),
            _ => Err(format!(
                "unknown format: '{s}' (use 'table', 'markdown', 'csv', or 'json')"
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

/// Accumulator for a report group.
struct GroupAccum {
    total_secs: i64,
    entry_count: usize,
    /// Accumulated earnings in cents (None if any entry in the group lacks a rate).
    earnings_cents: Option<i64>,
}

impl GroupAccum {
    /// Creates a new accumulator.
    fn new() -> Self {
        Self {
            total_secs: 0,
            entry_count: 0,
            earnings_cents: Some(0),
        }
    }

    /// Adds an entry's contribution to this group.
    fn add(&mut self, duration: i64, hourly_rate_cents: Option<i64>) {
        self.total_secs += duration;
        self.entry_count += 1;
        match (self.earnings_cents, hourly_rate_cents) {
            (Some(acc), Some(rate)) => {
                self.earnings_cents = Some(acc + duration * rate / 3600);
            }
            _ => {
                // If any entry lacks a rate, earnings become indeterminate
                self.earnings_cents = None;
            }
        }
    }

    /// Converts to a report row.
    fn into_row(self, group: String) -> ReportRow {
        // If no entries had rates, show None rather than Some(0)
        let earnings = match self.earnings_cents {
            Some(0) if self.entry_count > 0 => None,
            other => other,
        };
        ReportRow {
            group,
            total_secs: self.total_secs,
            entry_count: self.entry_count,
            earnings_cents: earnings,
        }
    }
}

/// Result of report generation, including deduplicated totals.
pub struct ReportResult {
    /// Grouped rows.
    pub rows: Vec<ReportRow>,
    /// Total seconds across unique entries (not double-counted).
    pub unique_total_secs: i64,
    /// Total unique entries (not double-counted).
    pub unique_entry_count: usize,
}

/// Generates grouped report rows from entries, with deduplicated totals.
pub fn generate_report(entries: &[(TimeEntry, Project)], group_by: &GroupBy) -> ReportResult {
    let mut groups: std::collections::BTreeMap<String, GroupAccum> =
        std::collections::BTreeMap::new();

    // Track unique entries for accurate totals
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut unique_total_secs: i64 = 0;

    for (entry, project) in entries {
        let duration = entry.computed_duration_secs().unwrap_or(0);

        // Count each entry only once for totals
        let entry_id = entry.id.as_str().to_owned();
        if seen_ids.insert(entry_id) {
            unique_total_secs += duration;
        }

        match group_by {
            GroupBy::Project => {
                groups
                    .entry(project.name.clone())
                    .or_insert_with(GroupAccum::new)
                    .add(duration, project.hourly_rate_cents);
            }
            GroupBy::Tag => {
                if entry.tags.is_empty() {
                    groups
                        .entry("(untagged)".to_string())
                        .or_insert_with(GroupAccum::new)
                        .add(duration, project.hourly_rate_cents);
                } else {
                    for tag in &entry.tags {
                        groups
                            .entry(tag.clone())
                            .or_insert_with(GroupAccum::new)
                            .add(duration, project.hourly_rate_cents);
                    }
                }
            }
        }
    }

    let rows: Vec<ReportRow> = groups
        .into_iter()
        .map(|(group, accum)| accum.into_row(group))
        .collect();

    ReportResult {
        rows,
        unique_total_secs,
        unique_entry_count: seen_ids.len(),
    }
}

/// Formats report rows into the specified output format.
pub fn format_report(result: &ReportResult, format: &ReportFormat) -> String {
    match format {
        ReportFormat::Table => format_table(result),
        ReportFormat::Markdown => format_markdown(result),
        ReportFormat::Csv => format_csv(&result.rows),
        ReportFormat::Json => format_json(&result.rows),
    }
}

/// Escapes a string for use in a CSV field.
fn escape_csv(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Escapes a string for use in a Markdown table cell.
fn escape_markdown(field: &str) -> String {
    field.replace('|', "\\|").replace('\n', " ")
}

/// Renders rows as a plain-text aligned table for terminal display.
fn format_table(result: &ReportResult) -> String {
    if result.rows.is_empty() {
        return "No entries found.\n".to_string();
    }

    // Pre-format all values so we can measure widths
    let formatted: Vec<(String, String, String, String)> = result
        .rows
        .iter()
        .map(|row| {
            let earnings = match row.earnings_cents {
                Some(c) => format!("${}.{:02}", c / 100, c % 100),
                None => "\u{2014}".to_string(),
            };
            (
                row.group.clone(),
                format_duration_human(row.total_secs),
                row.entry_count.to_string(),
                earnings,
            )
        })
        .collect();

    let total_time = format_duration_human(result.unique_total_secs);
    let total_entries = result.unique_entry_count.to_string();

    // Compute dynamic column widths from headers, rows, and footer
    let gw = formatted
        .iter()
        .map(|(g, _, _, _)| g.len())
        .chain(std::iter::once("GROUP".len()))
        .chain(std::iter::once("Total".len()))
        .max()
        .unwrap_or(5);
    let tw = formatted
        .iter()
        .map(|(_, t, _, _)| t.len())
        .chain(std::iter::once("TIME".len()))
        .chain(std::iter::once(total_time.len()))
        .max()
        .unwrap_or(4);
    let ew = formatted
        .iter()
        .map(|(_, _, e, _)| e.len())
        .chain(std::iter::once("ENTRIES".len()))
        .chain(std::iter::once(total_entries.len()))
        .max()
        .unwrap_or(7);
    let rw = formatted
        .iter()
        .map(|(_, _, _, r)| r.len())
        .chain(std::iter::once("EARNINGS".len()))
        .max()
        .unwrap_or(8);

    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "  {:<gw$}  {:>tw$}  {:>ew$}  {:>rw$}\n",
        "GROUP", "TIME", "ENTRIES", "EARNINGS",
    ));

    // Rows
    for (group, time, entries, earnings) in &formatted {
        out.push_str(&format!(
            "  {:<gw$}  {:>tw$}  {:>ew$}  {:>rw$}\n",
            group, time, entries, earnings,
        ));
    }

    // Footer
    out.push_str(&format!(
        "  {:<gw$}  {:>tw$}  {:>ew$}  {:>rw$}\n",
        "Total", total_time, total_entries, "",
    ));

    out
}

/// Renders rows as a Markdown table with deduplicated totals.
fn format_markdown(result: &ReportResult) -> String {
    let mut out = String::new();
    out.push_str("| Group | Time | Entries | Earnings |\n");
    out.push_str("|-------|------|---------|----------|\n");

    for row in &result.rows {
        let earnings = match row.earnings_cents {
            Some(c) => format!("${}.{:02}", c / 100, c % 100),
            None => "\u{2014}".to_string(),
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            escape_markdown(&row.group),
            format_duration_human(row.total_secs),
            row.entry_count,
            earnings,
        ));
    }

    out.push_str(&format!(
        "| **Total** | **{}** | **{}** | |\n",
        format_duration_human(result.unique_total_secs),
        result.unique_entry_count,
    ));

    out
}

/// Renders rows as CSV with proper field escaping.
fn format_csv(rows: &[ReportRow]) -> String {
    let mut out = String::from("group,time_secs,time_human,entries,earnings_cents\n");
    for row in rows {
        let earnings = row
            .earnings_cents
            .map(|c| c.to_string())
            .unwrap_or_default();
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            escape_csv(&row.group),
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
    use crate::models::project::{Project, ProjectSource, ProjectStatus};
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
            source: ProjectSource::Manual,
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

        let result = generate_report(&entries, &GroupBy::Project);
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0].group, "app-1");
        assert_eq!(result.rows[0].total_secs, 5400);
        assert_eq!(result.rows[0].entry_count, 2);
        assert_eq!(result.rows[0].earnings_cents, Some(22500)); // 1.5h * $150
        assert_eq!(result.rows[1].group, "app-2");
        assert_eq!(result.rows[1].earnings_cents, None);
    }

    #[test]
    fn group_by_tag() {
        let p = make_project("app", None);
        let entries = vec![
            make_entry(&p, 3600, vec!["frontend", "client"]),
            make_entry(&p, 1800, vec!["frontend"]),
            make_entry(&p, 900, vec![]),
        ];

        let result = generate_report(&entries, &GroupBy::Tag);
        assert_eq!(result.rows.len(), 3); // (untagged), client, frontend

        let untagged = result
            .rows
            .iter()
            .find(|r| r.group == "(untagged)")
            .unwrap();
        assert_eq!(untagged.total_secs, 900);

        let frontend = result.rows.iter().find(|r| r.group == "frontend").unwrap();
        assert_eq!(frontend.total_secs, 5400); // 3600 + 1800

        let client = result.rows.iter().find(|r| r.group == "client").unwrap();
        assert_eq!(client.total_secs, 3600);
    }

    #[test]
    fn tag_earnings_from_project_rate() {
        let p = make_project("app", Some(10000)); // $100/hr
        let entries = vec![
            make_entry(&p, 3600, vec!["frontend"]), // 1h = $100
            make_entry(&p, 1800, vec!["frontend"]), // 30m = $50
        ];

        let result = generate_report(&entries, &GroupBy::Tag);
        let frontend = result.rows.iter().find(|r| r.group == "frontend").unwrap();
        assert_eq!(frontend.earnings_cents, Some(15000)); // $150
    }

    #[test]
    fn deduplicated_totals_for_tags() {
        let p = make_project("app", None);
        // One entry with two tags — should only count once in totals
        let entries = vec![make_entry(&p, 3600, vec!["frontend", "client"])];

        let result = generate_report(&entries, &GroupBy::Tag);
        assert_eq!(result.rows.len(), 2); // two tag groups
        assert_eq!(result.unique_total_secs, 3600); // not 7200
        assert_eq!(result.unique_entry_count, 1); // not 2
    }

    #[test]
    fn format_csv_output() {
        let result = ReportResult {
            rows: vec![ReportRow {
                group: "app".to_string(),
                total_secs: 5400,
                entry_count: 2,
                earnings_cents: Some(22500),
            }],
            unique_total_secs: 5400,
            unique_entry_count: 2,
        };
        let csv = format_report(&result, &ReportFormat::Csv);
        assert!(csv.contains("group,time_secs,time_human,entries,earnings_cents"));
        assert!(csv.contains("app,5400,1h 30m,2,22500"));
    }

    #[test]
    fn format_csv_escapes_commas() {
        let result = ReportResult {
            rows: vec![ReportRow {
                group: "my,app".to_string(),
                total_secs: 3600,
                entry_count: 1,
                earnings_cents: None,
            }],
            unique_total_secs: 3600,
            unique_entry_count: 1,
        };
        let csv = format_report(&result, &ReportFormat::Csv);
        assert!(csv.contains("\"my,app\""));
    }

    #[test]
    fn format_json_output() {
        let result = ReportResult {
            rows: vec![ReportRow {
                group: "app".to_string(),
                total_secs: 3600,
                entry_count: 1,
                earnings_cents: None,
            }],
            unique_total_secs: 3600,
            unique_entry_count: 1,
        };
        let json = format_report(&result, &ReportFormat::Json);
        assert!(json.contains("\"group\": \"app\""));
        assert!(json.contains("\"total_secs\": 3600"));
        assert!(json.contains("\"earnings_cents\": null"));
    }

    #[test]
    fn format_markdown_output() {
        let result = ReportResult {
            rows: vec![ReportRow {
                group: "app".to_string(),
                total_secs: 3600,
                entry_count: 1,
                earnings_cents: Some(15000),
            }],
            unique_total_secs: 3600,
            unique_entry_count: 1,
        };
        let md = format_report(&result, &ReportFormat::Markdown);
        assert!(md.contains("| app | 1h | 1 | $150.00 |"));
        assert!(md.contains("| **Total**"));
    }

    #[test]
    fn format_markdown_escapes_pipes() {
        let result = ReportResult {
            rows: vec![ReportRow {
                group: "a|b".to_string(),
                total_secs: 60,
                entry_count: 1,
                earnings_cents: None,
            }],
            unique_total_secs: 60,
            unique_entry_count: 1,
        };
        let md = format_report(&result, &ReportFormat::Markdown);
        assert!(md.contains("a\\|b"));
    }

    #[test]
    fn format_table_output() {
        let result = ReportResult {
            rows: vec![ReportRow {
                group: "app".to_string(),
                total_secs: 3600,
                entry_count: 1,
                earnings_cents: Some(15000),
            }],
            unique_total_secs: 3600,
            unique_entry_count: 1,
        };
        let table = format_report(&result, &ReportFormat::Table);
        assert!(table.contains("GROUP"));
        assert!(table.contains("app"));
        assert!(table.contains("1h"));
        assert!(table.contains("$150.00"));
        assert!(table.contains("Total"));
    }

    #[test]
    fn format_table_empty() {
        let result = ReportResult {
            rows: vec![],
            unique_total_secs: 0,
            unique_entry_count: 0,
        };
        let table = format_report(&result, &ReportFormat::Table);
        assert_eq!(table, "No entries found.\n");
    }
}
