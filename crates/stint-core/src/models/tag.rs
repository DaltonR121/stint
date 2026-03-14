//! Tag normalization and parsing utilities.

/// Normalizes a tag string: lowercased, trimmed, whitespace removed.
pub fn normalize_tag(tag: &str) -> String {
    tag.trim().to_lowercase().replace(' ', "-")
}

/// Parses a comma-separated tag string into a sorted, deduplicated vec of normalized tags.
///
/// Empty segments are discarded.
pub fn parse_tags(input: &str) -> Vec<String> {
    let mut tags: Vec<String> = input
        .split(',')
        .map(normalize_tag)
        .filter(|t| !t.is_empty())
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lowercases() {
        assert_eq!(normalize_tag("Frontend"), "frontend");
    }

    #[test]
    fn normalize_trims_whitespace() {
        assert_eq!(normalize_tag("  api  "), "api");
    }

    #[test]
    fn normalize_replaces_spaces_with_hyphens() {
        assert_eq!(normalize_tag("long tag name"), "long-tag-name");
    }

    #[test]
    fn parse_splits_commas() {
        assert_eq!(parse_tags("a,b, c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_deduplicates() {
        assert_eq!(parse_tags("a,a,b"), vec!["a", "b"]);
    }

    #[test]
    fn parse_empty_string() {
        let result = parse_tags("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_handles_trailing_comma() {
        assert_eq!(parse_tags("x,y,"), vec!["x", "y"]);
    }
}
