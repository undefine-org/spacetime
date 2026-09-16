//! Pure path helpers for the export pipeline.
//!
//! Stubbed in Wave 0; populated TDD-style in Wave 1 by the PATH task.

use std::path::{Path, PathBuf};

/// Compute an absolute href (leading slash) for a bundle co-located with the
/// given directory. Trailing-slash-agnostic; deploy-root-anchored.
#[allow(dead_code)]
pub(crate) fn absolute_href(bundle_dir: &Path, name: &str) -> String {
    let dir = bundle_dir.to_string_lossy().replace('\\', "/");
    if dir.is_empty() {
        format!("/{}", name)
    } else {
        format!("/{}/{}", dir, name)
    }
}

/// Climb from `html_dir` toward the site root, returning the nearest ancestor
/// (or self) directory contained in `st_dirs`.
#[allow(dead_code)]
pub(crate) fn find_nearest_st_dir(
    html_dir: &Path,
    st_dirs: &std::collections::BTreeSet<std::path::PathBuf>,
) -> std::path::PathBuf {
    let mut current = html_dir;
    loop {
        if st_dirs.contains(current) {
            return current.to_path_buf();
        }
        if current.as_os_str().is_empty() || current == Path::new(".") {
            return PathBuf::new();
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return PathBuf::new(),
        }
    }
}

/// Return true when the named directory should NOT be descended into.
/// Centralised denylist: build/source-control/asset-copy dirs.
#[allow(dead_code)]
pub(crate) fn is_denylisted(dir_name: &str) -> bool {
    matches!(
        dir_name,
        "dist" | "node_modules" | "target" | "tests" | "data" | "assets" | "locales"
    ) || dir_name.starts_with('.')
        || dir_name.starts_with('_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn absolute_href_root_bundle() {
        assert_eq!(
            absolute_href(Path::new(""), "spacetime.css"),
            "/spacetime.css"
        );
    }

    #[test]
    fn absolute_href_route_bundle() {
        assert_eq!(
            absolute_href(Path::new("about"), "spacetime.css"),
            "/about/spacetime.css"
        );
    }

    #[test]
    fn absolute_href_nested_route_bundle() {
        assert_eq!(
            absolute_href(Path::new("talent/jessica-lee"), "spacetime.js"),
            "/talent/jessica-lee/spacetime.js"
        );
    }

    #[test]
    fn absolute_href_always_starts_with_slash() {
        let cases = ["about", "talent/jessica-lee", "a/b/c"];
        for case in &cases {
            let href = absolute_href(Path::new(case), "spacetime.css");
            assert!(href.starts_with('/'), "href should start with /: {}", href);
            assert!(!href.contains(".."), "href should not contain ..: {}", href);
        }
    }

    #[test]
    fn find_nearest_st_returns_self_when_route_has_st() {
        let mut st_dirs = BTreeSet::new();
        st_dirs.insert(PathBuf::from(""));
        st_dirs.insert(PathBuf::from("about"));
        assert_eq!(
            find_nearest_st_dir(Path::new("about"), &st_dirs),
            PathBuf::from("about")
        );
    }

    #[test]
    fn find_nearest_st_climbs_to_ancestor() {
        let mut st_dirs = BTreeSet::new();
        st_dirs.insert(PathBuf::from(""));
        st_dirs.insert(PathBuf::from("about"));
        assert_eq!(
            find_nearest_st_dir(Path::new("about/team"), &st_dirs),
            PathBuf::from("about")
        );
    }

    #[test]
    fn find_nearest_st_falls_through_to_root() {
        let mut st_dirs = BTreeSet::new();
        st_dirs.insert(PathBuf::from(""));
        assert_eq!(
            find_nearest_st_dir(Path::new("unknown/deep"), &st_dirs),
            PathBuf::from("")
        );
    }

    #[test]
    fn denylist_excludes_build_dirs() {
        for name in &["dist", "node_modules", "target", "tests"] {
            assert!(is_denylisted(name), "{} should be denylisted", name);
        }
    }

    #[test]
    fn denylist_excludes_hidden_and_underscore() {
        for name in &[".git", "_partials", ".hidden", "_x"] {
            assert!(is_denylisted(name), "{} should be denylisted", name);
        }
    }

    #[test]
    fn denylist_passes_normal_dirs() {
        for name in &["about", "talent", "modules"] {
            assert!(!is_denylisted(name), "{} should not be denylisted", name);
        }
    }
}
