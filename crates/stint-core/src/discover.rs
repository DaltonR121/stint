//! Project auto-discovery from filesystem markers.
//!
//! Walks up the directory tree looking for `.git` directories to automatically
//! detect projects without manual registration.

use std::path::{Path, PathBuf};

/// Result of a successful project discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredProject {
    /// The root directory of the discovered project (where `.git` lives).
    pub root: PathBuf,
    /// The project name (directory name of the root).
    pub name: String,
}

/// Maximum number of parent directories to walk up when searching for `.git`.
const MAX_DEPTH: usize = 10;

/// Attempts to discover a project from the given directory by walking up
/// the directory tree looking for a `.git` directory.
///
/// Returns `None` if no `.git` is found within `MAX_DEPTH` levels, or if
/// the path is under a common non-project directory (e.g., `/tmp`, `/`).
pub fn discover_project(cwd: &Path) -> Option<DiscoveredProject> {
    let mut current = cwd.to_path_buf();

    for _ in 0..=MAX_DEPTH {
        if current.join(".git").exists() {
            let name = current
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed".to_string());

            return Some(DiscoveredProject {
                root: current,
                name,
            });
        }

        if !current.pop() {
            break;
        }

        // Stop at filesystem root or common non-project directories
        if current == Path::new("/") || current == Path::new("") {
            break;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn discovers_git_repo_at_cwd() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();

        let result = discover_project(tmp.path()).unwrap();
        assert_eq!(result.root, tmp.path());
        assert!(!result.name.is_empty());
    }

    #[test]
    fn discovers_git_repo_from_subdirectory() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        let sub = tmp.path().join("src").join("components");
        fs::create_dir_all(&sub).unwrap();

        let result = discover_project(&sub).unwrap();
        assert_eq!(result.root, tmp.path());
    }

    #[test]
    fn returns_none_without_git() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("some").join("deep").join("path");
        fs::create_dir_all(&sub).unwrap();

        assert!(discover_project(&sub).is_none());
    }

    #[test]
    fn discovers_at_max_depth_boundary() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("my-project");
        fs::create_dir_all(project_dir.join(".git")).unwrap();

        // Build a path exactly MAX_DEPTH levels deep
        let mut deep = project_dir.clone();
        for i in 0..MAX_DEPTH {
            deep = deep.join(format!("level{i}"));
        }
        fs::create_dir_all(&deep).unwrap();

        let result = discover_project(&deep);
        assert!(result.is_some(), "should discover at exactly MAX_DEPTH");
        assert_eq!(result.unwrap().root, project_dir);
    }

    #[test]
    fn uses_directory_name_as_project_name() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("my-awesome-app");
        fs::create_dir_all(project_dir.join(".git")).unwrap();

        let result = discover_project(&project_dir).unwrap();
        assert_eq!(result.name, "my-awesome-app");
    }
}
