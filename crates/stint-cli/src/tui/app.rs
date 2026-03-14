//! TUI application state and data fetching.

use stint_core::models::entry::{EntryFilter, TimeEntry};
use stint_core::models::project::Project;
use stint_core::service::StintService;
use stint_core::storage::sqlite::SqliteStorage;
use time::OffsetDateTime;

/// Which panel is currently focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// Today's entries list.
    Today,
    /// Weekly project totals.
    Week,
}

impl Panel {
    /// Cycles to the next panel.
    pub fn next(self) -> Self {
        match self {
            Self::Today => Self::Week,
            Self::Week => Self::Today,
        }
    }
}

/// Dashboard application state.
pub struct App {
    service: StintService<SqliteStorage>,
    /// Currently running timer and its project, if any.
    pub running_timer: Option<(TimeEntry, Project)>,
    /// Today's time entries with their projects.
    pub today_entries: Vec<(TimeEntry, Project)>,
    /// This week's per-project totals: (project_name, total_secs).
    pub week_totals: Vec<(String, i64)>,
    /// Currently focused panel.
    pub selected_panel: Panel,
    /// Scroll offset for the today panel.
    pub today_scroll: usize,
    /// Scroll offset for the week panel.
    pub week_scroll: usize,
    /// Whether the app should quit.
    pub should_quit: bool,
}

impl App {
    /// Creates a new App with the given storage backend.
    pub fn new(storage: SqliteStorage) -> Self {
        let service = StintService::new(storage);
        let mut app = Self {
            service,
            running_timer: None,
            today_entries: vec![],
            week_totals: vec![],
            selected_panel: Panel::Today,
            today_scroll: 0,
            week_scroll: 0,
            should_quit: false,
        };
        app.refresh();
        app
    }

    /// Refreshes all dashboard data from the database.
    pub fn refresh(&mut self) {
        let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());

        // Running timer
        self.running_timer = self.service.get_status().unwrap_or(None);

        // Today's entries
        let today_start = now.date().midnight().assume_utc();
        let today_filter = EntryFilter {
            from: Some(today_start),
            ..Default::default()
        };
        self.today_entries = self.service.get_entries(&today_filter).unwrap_or_default();

        // This week's totals (Monday to now)
        let weekday = now.weekday().number_days_from_monday();
        let week_start = today_start - time::Duration::days(weekday as i64);
        let week_filter = EntryFilter {
            from: Some(week_start),
            ..Default::default()
        };
        let week_entries = self.service.get_entries(&week_filter).unwrap_or_default();

        // Aggregate by project
        let mut totals: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        for (entry, project) in &week_entries {
            let duration = entry.computed_duration_secs().unwrap_or(0);
            *totals.entry(project.name.clone()).or_insert(0) += duration;
        }
        self.week_totals = totals.into_iter().collect();
        // Sort by total descending
        self.week_totals.sort_by(|a, b| b.1.cmp(&a.1));
    }

    /// Scrolls the focused panel up.
    pub fn scroll_up(&mut self) {
        match self.selected_panel {
            Panel::Today => {
                self.today_scroll = self.today_scroll.saturating_sub(1);
            }
            Panel::Week => {
                self.week_scroll = self.week_scroll.saturating_sub(1);
            }
        }
    }

    /// Scrolls the focused panel down.
    pub fn scroll_down(&mut self) {
        match self.selected_panel {
            Panel::Today => {
                let max = self.today_entries.len().saturating_sub(1);
                if self.today_scroll < max {
                    self.today_scroll += 1;
                }
            }
            Panel::Week => {
                let max = self.week_totals.len().saturating_sub(1);
                if self.week_scroll < max {
                    self.week_scroll += 1;
                }
            }
        }
    }
}
