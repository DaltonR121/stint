//! Timeline view for the TUI dashboard.
//!
//! Renders a chronological timeline of entries with:
//! - Visual timeline blocks proportional to duration
//! - Activity gaps shown as idle periods
//! - Session merging (consecutive same-project entries grouped)
//! - "Yesterday" toggle via 'y' key

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use stint_core::duration::format_duration_human;
use stint_core::models::entry::TimeEntry;
use stint_core::models::project::Project;
use time::OffsetDateTime;

use super::app::TimelineView;

/// Color palette for project timeline bars.
static PROJECT_COLORS: &[Color] = &[
    Color::Cyan,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::Blue,
    Color::Red,
];

/// Consecutive same-project entries separated by less than this (in seconds) are
/// merged into a single session block.
const MERGE_GAP_SECS: i64 = 300; // 5 minutes

/// Gaps shorter than this (in seconds) are not rendered as idle markers.
const IDLE_MIN_SECS: i64 = 120; // 2 minutes

/// A timeline item: either a work session or an idle gap.
///
/// An [`TimelineItem::Entry`] may represent several merged entries — see
/// [`build_grouped_timeline`].
enum TimelineItem<'a> {
    Entry {
        project: &'a Project,
        start: OffsetDateTime,
        end: OffsetDateTime,
        /// Sum of the tracked durations of the merged entries (gaps excluded).
        duration_secs: i64,
        /// Distinct, non-empty notes from the merged entries, in order.
        notes: Vec<&'a str>,
        /// Whether the session is currently running.
        is_running: bool,
        /// How many raw entries were merged into this session (≥ 1).
        merged_count: usize,
    },
    Idle {
        start: OffsetDateTime,
        end: OffsetDateTime,
        duration_secs: i64,
    },
}

/// Returns the effective end of an entry — `now` if it is still running.
fn entry_end(entry: &TimeEntry, now: OffsetDateTime) -> OffsetDateTime {
    if entry.is_running() {
        now
    } else {
        entry.end.unwrap_or(entry.start)
    }
}

/// Returns the tracked duration of an entry in seconds — measured against `now`
/// if it is still running.
fn entry_duration(entry: &TimeEntry, now: OffsetDateTime) -> i64 {
    if entry.is_running() {
        (now - entry.start).whole_seconds()
    } else {
        entry.computed_duration_secs().unwrap_or(0)
    }
}

/// Appends an entry's note to `notes`, skipping empties and consecutive
/// duplicates (the same task logged repeatedly across merged entries).
fn push_note<'a>(notes: &mut Vec<&'a str>, entry: &'a TimeEntry) {
    if let Some(n) = entry.notes.as_deref() {
        if !n.is_empty() && notes.last() != Some(&n) {
            notes.push(n);
        }
    }
}

/// Renders the timeline panel.
pub fn render_timeline(
    frame: &mut Frame,
    area: Rect,
    entries: &[(TimeEntry, Project)],
    scroll: usize,
    view: TimelineView,
    is_focused: bool,
) {
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let lines = build_timeline_lines(entries, view);

    // Clamp scroll defensively — App clamps it too, but never trust the caller to
    // keep an offset that would skip every line and blank the panel.
    let skip = scroll.min(lines.len());
    let scrolled_lines: Vec<Line> = lines.into_iter().skip(skip).collect();

    let view_label = match view {
        TimelineView::Today => "Timeline (Today)",
        TimelineView::Yesterday => "Timeline (Yesterday)",
    };
    let view_title = format!(" {view_label} ");
    let timeline = Paragraph::new(scrolled_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(view_title)
            .border_style(border_style),
    );
    frame.render_widget(timeline, area);
}

/// Returns the total number of rendered lines for the given view, so the caller
/// can clamp scrolling exactly to the content (no stranded bottom, no blank
/// overscroll). Mirrors the line count produced by [`render_timeline`].
pub fn line_count(entries: &[(TimeEntry, Project)], view: TimelineView) -> usize {
    build_timeline_lines(entries, view).len()
}

/// Builds the full set of rendered lines for the timeline (header, entries, idle
/// gaps), independent of scroll. Shared by [`render_timeline`] and [`line_count`]
/// so the two can never drift.
fn build_timeline_lines(
    entries: &[(TimeEntry, Project)],
    view: TimelineView,
) -> Vec<Line<'static>> {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());

    // Determine date range based on view mode. Use replace_time to preserve the
    // local offset, matching how App::refresh fetches entries — otherwise the
    // filter boundaries here disagree with the fetched window and entries near
    // midnight get dropped for any non-UTC user.
    let today_start = now.replace_time(time::Time::MIDNIGHT);
    let (day_start, day_end) = match view {
        TimelineView::Today => (today_start, today_start + time::Duration::days(1)),
        TimelineView::Yesterday => (today_start - time::Duration::days(1), today_start),
    };

    // Build grouped timeline items
    let grouped = build_grouped_timeline(entries, day_start, day_end, now);

    let mut lines: Vec<Line> = Vec::new();

    // Header line with view toggle hint
    let toggle_hint = match view {
        TimelineView::Today => " [y]esterday",
        TimelineView::Yesterday => " [t]oday",
    };
    let total_time: i64 = grouped
        .iter()
        .filter_map(|item| match item {
            TimelineItem::Entry { duration_secs, .. } => Some(*duration_secs),
            _ => None,
        })
        .sum();
    lines.push(Line::from(vec![
        Span::styled(
            format_duration_human(total_time),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(toggle_hint),
    ]));
    lines.push(Line::from(""));

    if grouped.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No entries for this day",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Build project color map for consistent coloring
    let mut color_map: std::collections::HashMap<String, Color> = std::collections::HashMap::new();
    let mut color_idx = 0;
    for item in &grouped {
        if let TimelineItem::Entry { project, .. } = item {
            if !color_map.contains_key(&project.name) {
                let color = PROJECT_COLORS[color_idx % PROJECT_COLORS.len()];
                color_map.insert(project.name.clone(), color);
                color_idx += 1;
            }
        }
    }

    // Render each timeline item
    for item in &grouped {
        match item {
            TimelineItem::Entry {
                project,
                start,
                end,
                duration_secs,
                notes,
                is_running,
                merged_count,
            } => {
                let color = color_map.get(&project.name).copied().unwrap_or(Color::Cyan);
                let start_str = format_time(*start);
                let end_str = format_time(*end);

                // Timeline bar
                let bar = "\u{2593}".repeat(4);
                let running_tag = if *is_running { " (running)" } else { "" };
                let session_tag = if *merged_count > 1 {
                    format!(" ({merged_count}×)")
                } else {
                    String::new()
                };
                let entry_line = format!(
                    " {} {} -- {} | {}  {:>8}{}{}",
                    bar,
                    start_str,
                    end_str,
                    project.name,
                    format_duration_human(*duration_secs),
                    running_tag,
                    session_tag,
                );

                lines.push(Line::from(Span::styled(
                    entry_line,
                    Style::default().fg(color),
                )));

                if !notes.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("         notes: {}", notes.join("; ")),
                        Style::default().fg(Color::Gray),
                    )));
                }
                lines.push(Line::from(""));
            }
            TimelineItem::Idle {
                start,
                end,
                duration_secs,
            } => {
                let start_str = format_time(*start);
                let end_str = format_time(*end);

                // Only show idle gaps > 2 minutes
                if *duration_secs > 120 {
                    let idle_line = format!(
                        "   {} -- {} | idle  {:>8}",
                        start_str,
                        end_str,
                        format_duration_human(*duration_secs)
                    );
                    lines.push(Line::from(Span::styled(
                        idle_line,
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(""));
                }
            }
        }
    }

    lines
}

/// Builds a grouped timeline from entries.
///
/// Consecutive entries for the *same* project separated by less than
/// [`MERGE_GAP_SECS`] are merged into a single session block (spanning the first
/// entry's start to the last entry's end, with their tracked durations summed and
/// notes combined). Gaps larger than [`IDLE_MIN_SECS`] between distinct sessions
/// are emitted as idle markers.
fn build_grouped_timeline<'a>(
    entries: &'a [(TimeEntry, Project)],
    day_start: OffsetDateTime,
    day_end: OffsetDateTime,
    now: OffsetDateTime,
) -> Vec<TimelineItem<'a>> {
    // Filter entries for this day and sort by start time
    let mut day_entries: Vec<_> = entries
        .iter()
        .filter(|(entry, _)| entry.start >= day_start && entry.start < day_end)
        .collect();
    day_entries.sort_by_key(|(entry, _)| entry.start);

    if day_entries.is_empty() {
        return vec![];
    }

    let mut items = Vec::new();
    let mut iter = day_entries.into_iter().peekable();

    while let Some((entry, project)) = iter.next() {
        // Anchor a new session on this entry.
        let start = entry.start;
        let mut end = entry_end(entry, now);
        let mut duration_secs = entry_duration(entry, now);
        let mut is_running = entry.is_running();
        let mut merged_count = 1;

        let mut notes: Vec<&str> = Vec::new();
        push_note(&mut notes, entry);

        // Greedily absorb following same-project entries within the merge window.
        while let Some((next_entry, next_project)) = iter.peek() {
            let gap = (next_entry.start - end).whole_seconds();
            if next_project.name != project.name || gap >= MERGE_GAP_SECS {
                break;
            }
            let (next_entry, _) = iter.next().expect("peek guaranteed a next item");
            // max() guards against overlapping entries shortening the session.
            end = end.max(entry_end(next_entry, now));
            duration_secs += entry_duration(next_entry, now);
            is_running = next_entry.is_running();
            push_note(&mut notes, next_entry);
            merged_count += 1;
        }

        items.push(TimelineItem::Entry {
            project,
            start,
            end,
            duration_secs,
            notes,
            is_running,
            merged_count,
        });

        // Emit an idle marker before the next session if the gap is significant.
        if let Some((next_entry, _)) = iter.peek() {
            let gap = (next_entry.start - end).whole_seconds();
            if gap > IDLE_MIN_SECS {
                items.push(TimelineItem::Idle {
                    start: end,
                    end: next_entry.start,
                    duration_secs: gap,
                });
            }
        }
    }

    items
}

/// Formats a time as HH:MM
fn format_time(dt: OffsetDateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339)
        .map(|s| s[11..16].to_string())
        .unwrap_or_else(|_| "??:??".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stint_core::models::{EntryId, EntrySource, ProjectId, ProjectSource, ProjectStatus};
    use time::macros::datetime;

    /// A wide day window so the per-day filter never trims test entries.
    const DAY_START: OffsetDateTime = datetime!(2026-01-01 0:00 UTC);
    const DAY_END: OffsetDateTime = datetime!(2026-01-02 0:00 UTC);
    /// A fixed "now" past all fixture entries, for running-entry math.
    const NOW: OffsetDateTime = datetime!(2026-01-01 23:00 UTC);

    fn project(name: &str) -> Project {
        Project {
            id: ProjectId::new(),
            name: name.to_string(),
            paths: vec![],
            tags: vec![],
            hourly_rate_cents: None,
            status: ProjectStatus::Active,
            source: ProjectSource::Manual,
            created_at: DAY_START,
            updated_at: DAY_START,
        }
    }

    fn entry(start: OffsetDateTime, end: Option<OffsetDateTime>, notes: Option<&str>) -> TimeEntry {
        TimeEntry {
            id: EntryId::new(),
            project_id: ProjectId::new(),
            session_id: None,
            start,
            end,
            duration_secs: None,
            source: EntrySource::Manual,
            notes: notes.map(str::to_string),
            tags: vec![],
            created_at: start,
            updated_at: start,
        }
    }

    /// Extracts the `(merged_count, duration_secs, notes)` of every Entry item.
    fn entry_summaries(items: &[TimelineItem]) -> Vec<(usize, i64, Vec<String>)> {
        items
            .iter()
            .filter_map(|i| match i {
                TimelineItem::Entry {
                    merged_count,
                    duration_secs,
                    notes,
                    ..
                } => Some((
                    *merged_count,
                    *duration_secs,
                    notes.iter().map(|s| s.to_string()).collect(),
                )),
                TimelineItem::Idle { .. } => None,
            })
            .collect()
    }

    fn idle_count(items: &[TimelineItem]) -> usize {
        items
            .iter()
            .filter(|i| matches!(i, TimelineItem::Idle { .. }))
            .count()
    }

    #[test]
    fn same_project_small_gap_merges_into_one_session() {
        // Two API entries 3 minutes apart (< 5 min) → one merged block, no idle.
        let entries = vec![
            (
                entry(
                    datetime!(2026-01-01 9:00 UTC),
                    Some(datetime!(2026-01-01 10:00 UTC)),
                    Some("part one"),
                ),
                project("api"),
            ),
            (
                entry(
                    datetime!(2026-01-01 10:03 UTC),
                    Some(datetime!(2026-01-01 10:30 UTC)),
                    Some("part two"),
                ),
                project("api"),
            ),
        ];

        let items = build_grouped_timeline(&entries, DAY_START, DAY_END, NOW);
        let summaries = entry_summaries(&items);

        assert_eq!(summaries.len(), 1, "expected a single merged session");
        let (count, duration, notes) = &summaries[0];
        assert_eq!(*count, 2);
        // Tracked time is summed (60m + 27m), the 3m gap is excluded.
        assert_eq!(*duration, 87 * 60);
        assert_eq!(notes, &["part one", "part two"]);
        assert_eq!(idle_count(&items), 0, "merged sessions hide internal gaps");

        // The block spans first start to last end.
        let TimelineItem::Entry { start, end, .. } = &items[0] else {
            panic!("expected an Entry");
        };
        assert_eq!(*start, datetime!(2026-01-01 9:00 UTC));
        assert_eq!(*end, datetime!(2026-01-01 10:30 UTC));
    }

    #[test]
    fn same_project_large_gap_stays_separate_with_idle_marker() {
        // 30-minute gap (≥ 5 min) → two sessions with an idle marker between.
        let entries = vec![
            (
                entry(
                    datetime!(2026-01-01 9:00 UTC),
                    Some(datetime!(2026-01-01 10:00 UTC)),
                    None,
                ),
                project("api"),
            ),
            (
                entry(
                    datetime!(2026-01-01 10:30 UTC),
                    Some(datetime!(2026-01-01 11:00 UTC)),
                    None,
                ),
                project("api"),
            ),
        ];

        let items = build_grouped_timeline(&entries, DAY_START, DAY_END, NOW);

        assert_eq!(entry_summaries(&items).len(), 2);
        assert_eq!(idle_count(&items), 1);
    }

    #[test]
    fn different_projects_never_merge() {
        // Adjacent entries (1-minute gap) but different projects → two sessions,
        // and the sub-2-minute gap is too small for an idle marker.
        let entries = vec![
            (
                entry(
                    datetime!(2026-01-01 9:00 UTC),
                    Some(datetime!(2026-01-01 10:00 UTC)),
                    None,
                ),
                project("api"),
            ),
            (
                entry(
                    datetime!(2026-01-01 10:01 UTC),
                    Some(datetime!(2026-01-01 10:30 UTC)),
                    None,
                ),
                project("frontend"),
            ),
        ];

        let items = build_grouped_timeline(&entries, DAY_START, DAY_END, NOW);

        let summaries = entry_summaries(&items);
        assert_eq!(summaries.len(), 2);
        assert!(summaries.iter().all(|(count, ..)| *count == 1));
        assert_eq!(idle_count(&items), 0);
    }

    #[test]
    fn running_entry_merges_and_stays_running() {
        // A closed entry followed 1 minute later by a still-running one (same
        // project) → one merged, running session measured against NOW.
        let entries = vec![
            (
                entry(
                    datetime!(2026-01-01 9:00 UTC),
                    Some(datetime!(2026-01-01 10:00 UTC)),
                    None,
                ),
                project("api"),
            ),
            (
                entry(datetime!(2026-01-01 10:01 UTC), None, None),
                project("api"),
            ),
        ];

        let items = build_grouped_timeline(&entries, DAY_START, DAY_END, NOW);

        assert_eq!(items.len(), 1);
        let TimelineItem::Entry {
            is_running,
            merged_count,
            end,
            ..
        } = &items[0]
        else {
            panic!("expected an Entry");
        };
        assert!(*is_running);
        assert_eq!(*merged_count, 2);
        assert_eq!(*end, NOW, "a running session extends to now");
    }

    #[test]
    fn entries_outside_the_day_window_are_filtered_out() {
        let entries = vec![
            (
                entry(
                    datetime!(2025-12-31 23:00 UTC),
                    Some(datetime!(2025-12-31 23:30 UTC)),
                    None,
                ),
                project("api"),
            ),
            (
                entry(
                    datetime!(2026-01-01 9:00 UTC),
                    Some(datetime!(2026-01-01 10:00 UTC)),
                    None,
                ),
                project("api"),
            ),
        ];

        let items = build_grouped_timeline(&entries, DAY_START, DAY_END, NOW);
        assert_eq!(entry_summaries(&items).len(), 1);
    }

    #[test]
    fn duplicate_notes_are_collapsed_when_merging() {
        let entries = vec![
            (
                entry(
                    datetime!(2026-01-01 9:00 UTC),
                    Some(datetime!(2026-01-01 9:30 UTC)),
                    Some("same task"),
                ),
                project("api"),
            ),
            (
                entry(
                    datetime!(2026-01-01 9:31 UTC),
                    Some(datetime!(2026-01-01 10:00 UTC)),
                    Some("same task"),
                ),
                project("api"),
            ),
        ];

        let items = build_grouped_timeline(&entries, DAY_START, DAY_END, NOW);
        let summaries = entry_summaries(&items);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].2, &["same task"], "duplicate notes dedup");
    }
}
