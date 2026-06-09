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

/// A timeline item: either a work entry or an idle gap.
enum TimelineItem<'a> {
    Entry {
        project: &'a Project,
        entry: &'a TimeEntry,
        start: OffsetDateTime,
        end: OffsetDateTime,
        duration_secs: i64,
    },
    Idle {
        start: OffsetDateTime,
        end: OffsetDateTime,
        duration_secs: i64,
    },
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
                entry,
                start,
                end,
                duration_secs,
            } => {
                let color = color_map.get(&project.name).copied().unwrap_or(Color::Cyan);
                let start_str = format_time(*start);
                let end_str = format_time(*end);
                let notes = entry.notes.as_deref().unwrap_or("");

                // Timeline bar
                let bar = "\u{2593}".repeat(4);
                let entry_line = format!(
                    " {} {} -- {} | {}  {:>8} {}",
                    bar,
                    start_str,
                    end_str,
                    project.name,
                    format_duration_human(*duration_secs),
                    if entry.is_running() { " (running)" } else { "" }
                );

                lines.push(Line::from(Span::styled(
                    entry_line,
                    Style::default().fg(color),
                )));

                if !notes.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("         notes: {}", notes),
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

/// Builds a grouped timeline from entries, merging consecutive entries for the same project
/// and inserting idle gaps between them.
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
        let actual_end = if entry.is_running() {
            now
        } else {
            entry.end.unwrap_or(entry.start)
        };

        let duration = if entry.is_running() {
            (now - entry.start).whole_seconds()
        } else {
            entry.computed_duration_secs().unwrap_or(0)
        };

        // Check if we should merge with next entry (same project, close in time)
        if let Some((next_entry, next_project)) = iter.peek() {
            let gap = (next_entry.start - actual_end).whole_seconds();
            let is_same_project = project.name == next_project.name;

            if gap > 120 && gap < 300 && is_same_project {
                // Merge: same project with small gap (< 5 min), show idle gap
                items.push(TimelineItem::Idle {
                    start: actual_end,
                    end: next_entry.start,
                    duration_secs: gap,
                });
            } else if gap > 120 && !is_same_project {
                // Different project, add idle gap if significant
                items.push(TimelineItem::Idle {
                    start: actual_end,
                    end: next_entry.start,
                    duration_secs: gap,
                });
            }
        }

        items.push(TimelineItem::Entry {
            project,
            entry,
            start: entry.start,
            end: actual_end,
            duration_secs: duration,
        });
    }

    items
}

/// Formats a time as HH:MM
fn format_time(dt: OffsetDateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339)
        .map(|s| s[11..16].to_string())
        .unwrap_or_else(|_| "??:??".to_string())
}
