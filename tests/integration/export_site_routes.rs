//! Integration tests for BUG-025: export_site multi-route discovery.
//!
//! Verifies that export_site emits every HTML, compiles per-route bundles,
//! computes relative hrefs correctly, respects the denylist, copies assets,
//! and produces deterministic reports.

use spacetime::export::{ExportConfig, export_site};
use std::fs;
use std::path::{Path, PathBuf};

/// Guard that keeps a tempfile::TempDir alive (and thus auto-deleted on drop).
struct TempDirGuard {
    _tmp: tempfile::TempDir,
}

/// Deterministically copy a directory tree (files + subdirs) from src to dst.
fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    let mut entries: Vec<_> = fs::read_dir(src).unwrap().flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path);
        } else {
            fs::copy(&src_path, &dst_path).unwrap();
        }
    }
}

/// Return the path to the fixture site directory.
fn fixture_site_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi_route_site")
}

/// Copy the fixture into a fresh temp directory and return (guard, site_path).
fn setup_temp_site() -> (TempDirGuard, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let site_src = fixture_site_dir();
    let site_dst = tmp.path().join("site");
    copy_dir_all(&site_src, &site_dst);
    (TempDirGuard { _tmp: tmp }, site_dst)
}

// =============================================================================
// 1. All expected HTMLs emitted, denylisted files absent
// =============================================================================

#[test]
fn test_export_emits_all_routes() {
    let (_guard, site) = setup_temp_site();
    let out = site.join("dist");
    let config = ExportConfig {
        site_dir: site.clone(),
        output_dir: out.clone(),
        minify: false,
    };
    let report = export_site(&config).expect("export should succeed");

    let expected_htmls = [
        "index.html",
        "about/index.html",
        "about/team/index.html",
        "work/index.html",
        "pages/legacy/index.html",
    ];
    for rel in &expected_htmls {
        let path = out.join(rel);
        assert!(path.exists(), "Expected {} to exist", path.display());
    }

    // Each emitted HTML should still contain its unique route marker
    let markers = [
        ("index.html", "<!-- ROUTE:root -->"),
        ("about/index.html", "<!-- ROUTE:about -->"),
        ("about/team/index.html", "<!-- ROUTE:about-team -->"),
        ("work/index.html", "<!-- ROUTE:work -->"),
        ("pages/legacy/index.html", "<!-- ROUTE:pages-legacy -->"),
    ];
    for (rel, marker) in &markers {
        let content = fs::read_to_string(out.join(rel)).unwrap();
        assert!(
            content.contains(marker),
            "{} should contain marker {}",
            rel,
            marker
        );
    }

    assert_eq!(report.pages_exported, expected_htmls.len());
}

// =============================================================================
// 2. Per-route bundles exist (or not) as expected
// =============================================================================

#[test]
fn test_export_per_route_bundles() {
    let (_guard, site) = setup_temp_site();
    let out = site.join("dist");
    let config = ExportConfig {
        site_dir: site,
        output_dir: out.clone(),
        minify: false,
    };
    export_site(&config).unwrap();

    // Root bundle exists and is non-empty
    let root_js = out.join("spacetime.js");
    let root_css = out.join("spacetime.css");
    assert!(root_js.exists() && root_js.metadata().unwrap().len() > 0);
    assert!(root_css.exists() && root_css.metadata().unwrap().len() > 0);

    // about bundle exists and is non-empty
    let about_js = out.join("about/spacetime.js");
    let about_css = out.join("about/spacetime.css");
    assert!(about_js.exists() && about_js.metadata().unwrap().len() > 0);
    assert!(about_css.exists() && about_css.metadata().unwrap().len() > 0);

    // about/team has no .st → no bundle
    assert!(!out.join("about/team/spacetime.js").exists());
    assert!(!out.join("about/team/spacetime.css").exists());

    // work has no .st → no bundle (uses root)
    assert!(!out.join("work/spacetime.js").exists());
    assert!(!out.join("work/spacetime.css").exists());

    // pages/legacy has no .st → no bundle (uses root)
    assert!(!out.join("pages/legacy/spacetime.js").exists());
    assert!(!out.join("pages/legacy/spacetime.css").exists());
}

// =============================================================================
// 3. Injected hrefs are correct per HTML
// =============================================================================

#[test]
fn test_export_html_hrefs_relative() {
    let (_guard, site) = setup_temp_site();
    let out = site.join("dist");
    let config = ExportConfig {
        site_dir: site,
        output_dir: out.clone(),
        minify: false,
    };
    export_site(&config).unwrap();

    // Hrefs are ABSOLUTE (rooted at site root). This is deliberate: hosts
    // like Vercel serve /route/index.html at both `/route` and `/route/`, but
    // a browser resolving a relative URL against `/route` (no slash) fetches
    // the wrong bundle. Absolute paths fix this once and for all.
    let cases = [
        ("index.html", "/spacetime.css", "/spacetime.js"),
        (
            "about/index.html",
            "/about/spacetime.css",
            "/about/spacetime.js",
        ),
        (
            "about/team/index.html",
            "/about/spacetime.css",
            "/about/spacetime.js",
        ),
        ("work/index.html", "/spacetime.css", "/spacetime.js"),
        ("pages/legacy/index.html", "/spacetime.css", "/spacetime.js"),
    ];

    for (rel, expected_css, expected_js) in &cases {
        let content = fs::read_to_string(out.join(rel)).unwrap();
        assert!(
            content.contains(&format!(r#"href="{}""#, expected_css)),
            "{} should link to css {}",
            rel,
            expected_css
        );
        assert!(
            content.contains(&format!(r#"src="{}""#, expected_js)),
            "{} should link to js {}",
            rel,
            expected_js
        );
    }
}

// =============================================================================
// 4. Denylist prevents emission
// =============================================================================

#[test]
fn test_export_denylist() {
    let (_guard, site) = setup_temp_site();
    let out = site.join("dist");
    let config = ExportConfig {
        site_dir: site,
        output_dir: out.clone(),
        minify: false,
    };
    export_site(&config).unwrap();

    assert!(!out.join("_partials/skip.html").exists());
    assert!(!out.join("dist/stale.html").exists());
    assert!(!out.join("node_modules/m/index.html").exists());
    // No node_modules subtree at all
    assert!(!out.join("node_modules").exists());
}

// =============================================================================
// 5. Assets copied with original contents
// =============================================================================

#[test]
fn test_export_copies_assets() {
    let (_guard, site) = setup_temp_site();
    let out = site.join("dist");
    let config = ExportConfig {
        site_dir: site.clone(),
        output_dir: out.clone(),
        minify: false,
    };
    export_site(&config).unwrap();

    let asset = out.join("assets/img.txt");
    assert!(asset.exists());
    assert_eq!(fs::read_to_string(&asset).unwrap(), "asset");

    let data = out.join("data/foo.json");
    assert!(data.exists());
    assert_eq!(fs::read_to_string(&data).unwrap(), "{}");
}

// =============================================================================
// 6. Report counters are accurate
// =============================================================================

#[test]
fn test_export_report_counts() {
    let (_guard, site) = setup_temp_site();
    let out = site.join("dist");
    let config = ExportConfig {
        site_dir: site,
        output_dir: out,
        minify: false,
    };
    let report = export_site(&config).unwrap();

    assert_eq!(report.pages_exported, 5, "Expected 5 pages exported");
    // 71a19cc8 (PLAN-005 W0-1): the static set is dev-serve parity (an
    // extension allowlist), not wholesale directories — `assets/img.txt`
    // (.txt) is NOT served in dev and no longer copied, leaving
    // `data/foo.json` as the fixture's single countable asset.
    assert!(
        report.files_copied >= 1,
        "Expected at least 1 copied asset file (data/foo.json), got {}",
        report.files_copied
    );
    assert!(report.total_size > 0, "total_size should be > 0");
}

// =============================================================================
// 7. Determinism: two runs on fresh copies produce identical counts
// =============================================================================

#[test]
fn test_export_determinism() {
    let run = || {
        let (_guard, site) = setup_temp_site();
        let out = site.join("dist");
        let config = ExportConfig {
            site_dir: site,
            output_dir: out,
            minify: false,
        };
        export_site(&config).unwrap()
    };

    let r1 = run();
    let r2 = run();

    assert_eq!(r1.pages_exported, r2.pages_exported);
    assert_eq!(r1.total_size, r2.total_size);
}
