//! Project domain model.

use std::path::PathBuf;
use time::OffsetDateTime;

use super::types::ProjectId;

/// Whether a project is actively tracked or archived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectStatus {
    /// Project is actively tracked.
    Active,
    /// Project is archived and hidden from default listings.
    Archived,
}

impl ProjectStatus {
    /// Returns the status as a lowercase string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }

    /// Parses a status from a stored string value.
    pub fn from_str_value(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}

/// How a project was created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectSource {
    /// Manually registered via `stint project add`.
    Manual,
    /// Auto-discovered from a `.git` directory.
    Discovered,
}

impl ProjectSource {
    /// Returns the source as a lowercase string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Discovered => "discovered",
        }
    }

    /// Parses a source from a stored string value.
    pub fn from_str_value(s: &str) -> Self {
        match s {
            "discovered" => Self::Discovered,
            _ => Self::Manual,
        }
    }
}

/// A tracked project.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    /// Unique identifier.
    pub id: ProjectId,
    /// User-facing name (unique).
    pub name: String,
    /// Directories that map to this project.
    pub paths: Vec<PathBuf>,
    /// User-defined tags.
    pub tags: Vec<String>,
    /// Hourly rate in cents (e.g., 15000 = $150.00).
    pub hourly_rate_cents: Option<i64>,
    /// Whether this project is active or archived.
    pub status: ProjectStatus,
    /// How this project was created.
    pub source: ProjectSource,
    /// When this project was created.
    pub created_at: OffsetDateTime,
    /// When this project was last updated.
    pub updated_at: OffsetDateTime,
}
