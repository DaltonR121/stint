//! Internal build tasks for stint.
//!
//! Currently provides `build-site`, which renders `README.md` into the
//! GitHub Pages landing page so the site never drifts from the README.
//! The generated output is deployed to the `gh-pages` branch by
//! `.github/workflows/pages.yml`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pulldown_cmark::{html, Event, Options, Parser, Tag};

/// Base URL for rewriting relative README links to their GitHub pages.
const REPO_BLOB_BASE: &str = "https://github.com/DaltonR121/stint/blob/main/";
/// Base URL for rewriting relative README image paths to raw content.
const REPO_RAW_BASE: &str = "https://raw.githubusercontent.com/DaltonR121/stint/main/";
/// Placeholder in `site/template.html` replaced with the rendered README.
const CONTENT_PLACEHOLDER: &str = "{{CONTENT}}";

/// Entry point: dispatches the requested task.
///
/// Usage: `cargo run -p xtask -- build-site [--out <dir>]`
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("build-site") => {
            let out_dir = parse_out_dir(&args[1..]).unwrap_or_else(|| PathBuf::from("_site"));
            match build_site(&repo_root(), &out_dir) {
                Ok(()) => {
                    println!("Site written to {}", out_dir.display());
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("error: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("usage: cargo run -p xtask -- build-site [--out <dir>]");
            ExitCode::FAILURE
        }
    }
}

/// Parses an optional `--out <dir>` flag from the remaining arguments.
///
/// Returns `None` when the flag is absent or has no value.
fn parse_out_dir(args: &[String]) -> Option<PathBuf> {
    let pos = args.iter().position(|a| a == "--out")?;
    args.get(pos + 1).map(PathBuf::from)
}

/// Returns the repository root, derived from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate lives one level below the repo root")
        .to_path_buf()
}

/// Builds the static Pages site into `out_dir`.
///
/// Renders `README.md` through the HTML template as `index.html` and copies
/// `site/404.html` verbatim. Returns an error message on any I/O failure.
fn build_site(repo_root: &Path, out_dir: &Path) -> Result<(), String> {
    let readme = fs::read_to_string(repo_root.join("README.md"))
        .map_err(|e| format!("reading README.md: {e}"))?;
    let template = fs::read_to_string(repo_root.join("site/template.html"))
        .map_err(|e| format!("reading site/template.html: {e}"))?;
    if !template.contains(CONTENT_PLACEHOLDER) {
        return Err(format!(
            "site/template.html is missing the {CONTENT_PLACEHOLDER} placeholder"
        ));
    }

    let page = template.replace(CONTENT_PLACEHOLDER, &render_markdown(&readme));

    fs::create_dir_all(out_dir).map_err(|e| format!("creating {}: {e}", out_dir.display()))?;
    fs::write(out_dir.join("index.html"), page).map_err(|e| format!("writing index.html: {e}"))?;
    fs::copy(repo_root.join("site/404.html"), out_dir.join("404.html"))
        .map_err(|e| format!("copying site/404.html: {e}"))?;
    Ok(())
}

/// Renders GitHub-flavored Markdown to an HTML fragment.
///
/// Enables tables and strikethrough, and rewrites relative link/image
/// destinations to absolute GitHub URLs so README-relative paths like
/// `LICENSE` or `CONTRIBUTING.md` resolve from the Pages site.
fn render_markdown(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(markdown, options).map(|event| match event {
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: rewrite_url(&dest_url, REPO_BLOB_BASE).into(),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: rewrite_url(&dest_url, REPO_RAW_BASE).into(),
            title,
            id,
        }),
        other => other,
    });
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

/// Rewrites a repo-relative URL against `base`; absolute URLs, fragments,
/// and mailto links are returned unchanged.
fn rewrite_url(url: &str, base: &str) -> String {
    let is_absolute = url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with('#')
        || url.starts_with('/')
        || url.starts_with("mailto:");
    if is_absolute {
        url.to_string()
    } else {
        format!("{base}{}", url.trim_start_matches("./"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_url_leaves_absolute_urls_unchanged() {
        for url in [
            "https://github.com/DaltonR121/stint",
            "http://example.com",
            "#install",
            "/stint/",
            "mailto:hi@example.com",
        ] {
            assert_eq!(rewrite_url(url, REPO_BLOB_BASE), url);
        }
    }

    #[test]
    fn rewrite_url_resolves_relative_paths_against_base() {
        assert_eq!(
            rewrite_url("LICENSE", REPO_BLOB_BASE),
            "https://github.com/DaltonR121/stint/blob/main/LICENSE"
        );
        assert_eq!(
            rewrite_url("./docs/guide.md", REPO_BLOB_BASE),
            "https://github.com/DaltonR121/stint/blob/main/docs/guide.md"
        );
        assert_eq!(
            rewrite_url("assets/logo.png", REPO_RAW_BASE),
            "https://raw.githubusercontent.com/DaltonR121/stint/main/assets/logo.png"
        );
    }

    #[test]
    fn render_markdown_rewrites_relative_readme_links() {
        let html = render_markdown("See [CHANGELOG.md](CHANGELOG.md) for history.");
        assert!(
            html.contains(r#"href="https://github.com/DaltonR121/stint/blob/main/CHANGELOG.md""#),
            "got: {html}"
        );
    }

    #[test]
    fn render_markdown_supports_gfm_tables() {
        let html = render_markdown("| Command | Description |\n|---|---|\n| `stint stop` | Stop |");
        assert!(html.contains("<table>"), "got: {html}");
        assert!(html.contains("<code>stint stop</code>"), "got: {html}");
    }

    #[test]
    fn render_markdown_passes_raw_html_through() {
        let html = render_markdown(
            "<details>\n<summary><h2>Reference</h2></summary>\n\nbody\n\n</details>",
        );
        assert!(html.contains("<details>"), "got: {html}");
        assert!(html.contains("</details>"), "got: {html}");
    }

    #[test]
    fn build_site_renders_readme_into_template() {
        let root = std::env::temp_dir().join(format!("xtask-test-{}", std::process::id()));
        let out = root.join("out");
        fs::create_dir_all(root.join("site")).unwrap();
        fs::write(root.join("README.md"), "# Hello\n\nWorld [a](LICENSE)").unwrap();
        fs::write(
            root.join("site/template.html"),
            "<html><body>{{CONTENT}}</body></html>",
        )
        .unwrap();
        fs::write(root.join("site/404.html"), "missing").unwrap();

        build_site(&root, &out).unwrap();

        let index = fs::read_to_string(out.join("index.html")).unwrap();
        assert!(index.contains("<h1>Hello</h1>"), "got: {index}");
        assert!(index.contains("blob/main/LICENSE"), "got: {index}");
        assert!(!index.contains(CONTENT_PLACEHOLDER), "got: {index}");
        assert_eq!(fs::read_to_string(out.join("404.html")).unwrap(), "missing");

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn build_site_fails_when_template_lacks_placeholder() {
        let root = std::env::temp_dir().join(format!("xtask-test-bad-{}", std::process::id()));
        fs::create_dir_all(root.join("site")).unwrap();
        fs::write(root.join("README.md"), "# Hello").unwrap();
        fs::write(
            root.join("site/template.html"),
            "<html>no placeholder</html>",
        )
        .unwrap();
        fs::write(root.join("site/404.html"), "missing").unwrap();

        let err = build_site(&root, &root.join("out")).unwrap_err();
        assert!(err.contains("placeholder"), "got: {err}");

        fs::remove_dir_all(&root).unwrap();
    }
}
