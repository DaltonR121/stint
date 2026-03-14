//! Newtype wrappers for entity identifiers.

use std::fmt;
use std::str::FromStr;

/// A ULID-based unique identifier for projects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectId(String);

/// A ULID-based unique identifier for time entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryId(String);

/// A ULID-based unique identifier for shell sessions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(String);

macro_rules! impl_id {
    ($ty:ident) => {
        impl $ty {
            /// Generates a new unique identifier.
            pub fn new() -> Self {
                Self(ulid::Ulid::new().to_string())
            }

            /// Returns the inner string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Constructs an ID from a stored string value without validation.
            ///
            /// This is intended for use by the storage layer when loading values
            /// that were previously validated on insert.
            pub(crate) fn from_storage(s: String) -> Self {
                Self(s)
            }
        }

        impl Default for $ty {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $ty {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                // Validate it's a valid ULID (26 Crockford base32 chars)
                ulid::Ulid::from_string(s)
                    .map(|u| Self(u.to_string()))
                    .map_err(|e| format!("invalid ULID: {e}"))
            }
        }
    };
}

impl_id!(ProjectId);
impl_id!(EntryId);
impl_id!(SessionId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_generates_valid_ulid() {
        let id = ProjectId::new();
        assert_eq!(id.as_str().len(), 26);
        // Roundtrip through FromStr validates format
        let parsed: ProjectId = id.as_str().parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn display_and_from_str_roundtrip() {
        let id = EntryId::new();
        let displayed = id.to_string();
        let parsed: EntryId = displayed.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn different_ids_are_not_equal() {
        let a = SessionId::new();
        let b = SessionId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn invalid_ulid_returns_error() {
        let result: Result<ProjectId, _> = "not-a-ulid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn ids_of_different_types_are_distinct() {
        // This is a compile-time guarantee, but we verify the values are independent
        let pid = ProjectId::new();
        let eid = EntryId::new();
        assert_ne!(pid.as_str(), eid.as_str());
    }

    #[test]
    fn from_storage_preserves_value() {
        let id = ProjectId::new();
        let raw = id.as_str().to_owned();
        let restored = ProjectId::from_storage(raw.clone());
        assert_eq!(restored.as_str(), raw);
    }
}
