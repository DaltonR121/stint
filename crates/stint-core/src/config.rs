//! Configuration file support for Stint.
//!
//! Reads a TOML-like config from `~/.config/stint/config.toml` (XDG-compliant).
//! Falls back to sensible defaults when the file doesn't exist.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Stint configuration with sensible defaults.
#[derive(Debug, Clone)]
pub struct StintConfig {
    /// Idle threshold in seconds before auto-pause (default: 300 = 5 minutes).
    pub idle_threshold_secs: i64,
    /// Default hourly rate in cents for new projects (default: None).
    pub default_rate_cents: Option<i64>,
    /// Whether .git auto-discovery is enabled (default: true).
    pub auto_discover: bool,
    /// Default tags applied to auto-discovered projects.
    pub default_tags: Vec<String>,
}

impl Default for StintConfig {
    fn default() -> Self {
        Self {
            idle_threshold_secs: 300,
            default_rate_cents: None,
            auto_discover: true,
            default_tags: vec![],
        }
    }
}

impl StintConfig {
    /// Returns the XDG-compliant config file path.
    pub fn default_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"));
        config_dir.join("stint").join("config.toml")
    }

    /// Loads config from the default path, falling back to defaults.
    pub fn load() -> Self {
        let path = Self::default_path();
        Self::load_from(&path).unwrap_or_default()
    }

    /// Loads config from a specific path.
    pub fn load_from(path: &Path) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;
        Some(Self::parse(&contents))
    }

    /// Parses a simple TOML-like config string.
    ///
    /// Supports `key = value` lines. Comments start with `#`.
    /// Unrecognized keys are ignored.
    fn parse(contents: &str) -> Self {
        let mut config = Self::default();
        let mut values: HashMap<String, String> = HashMap::new();

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().trim_matches('"').to_string();
                values.insert(key, value);
            }
        }

        if let Some(v) = values.get("idle_threshold") {
            if let Ok(secs) = v.parse::<i64>() {
                config.idle_threshold_secs = secs;
            }
        }

        if let Some(v) = values.get("default_rate") {
            if let Ok(cents) = v.parse::<i64>() {
                config.default_rate_cents = Some(cents);
            }
        }

        if let Some(v) = values.get("auto_discover") {
            match v.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => config.auto_discover = true,
                "false" | "0" | "no" | "off" => config.auto_discover = false,
                _ => {} // Unknown value — keep default
            }
        }

        if let Some(v) = values.get("default_tags") {
            config.default_tags = v
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = StintConfig::default();
        assert_eq!(config.idle_threshold_secs, 300);
        assert!(config.auto_discover);
        assert!(config.default_rate_cents.is_none());
    }

    #[test]
    fn parse_all_fields() {
        let input = r#"
# Stint configuration
idle_threshold = 600
default_rate = 15000
auto_discover = true
default_tags = rust, cli
"#;
        let config = StintConfig::parse(input);
        assert_eq!(config.idle_threshold_secs, 600);
        assert_eq!(config.default_rate_cents, Some(15000));
        assert!(config.auto_discover);
        assert_eq!(config.default_tags, vec!["rust", "cli"]);
    }

    #[test]
    fn parse_disables_auto_discover() {
        let input = "auto_discover = false";
        let config = StintConfig::parse(input);
        assert!(!config.auto_discover);
    }

    #[test]
    fn parse_ignores_unknown_keys() {
        let input = "unknown_key = whatever\nidle_threshold = 120";
        let config = StintConfig::parse(input);
        assert_eq!(config.idle_threshold_secs, 120);
    }

    #[test]
    fn parse_empty_string() {
        let config = StintConfig::parse("");
        assert_eq!(config.idle_threshold_secs, 300); // default
    }

    #[test]
    fn missing_file_returns_none() {
        assert!(StintConfig::load_from(Path::new("/nonexistent/path")).is_none());
    }
}
