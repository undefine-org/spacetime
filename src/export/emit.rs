//! Emission: compose discovery + bundling + HTML rewriting → write to disk.
//!
//! Stubbed in Wave 0; populated TDD-style in Wave 2 by the EMIT task.

use std::path::Path;

use super::{ExportConfig, ExportError, ExportReport};

/// Run the full export pipeline. Replaces the legacy `super::export_site_legacy`.
#[allow(dead_code)]
pub(crate) fn run(config: &ExportConfig) -> Result<ExportReport, ExportError> {
    std::fs::create_dir_all(&config.output_dir).map_err(|e| ExportError::Io {
        path: config.output_dir.clone(),
        source: e,
    })?;

    let output_dir_canonical = config.output_dir.canonicalize().ok();

    let layout = super::discover::discover(&config.site_dir, output_dir_canonical.as_deref())?;
    let bundles = super::bundle::compile_bundles(&config.site_dir, &layout.st_entries);

    let mut report = ExportReport {
        pages_exported: 0,
        files_copied: 0,
        total_size: 0,
        warnings: Vec::new(),
    };

    // Which bundles actually ship assets? A discovered `.st` entry is only a
    // PAGE if something serves it:
    //   * an authored `.html` route names it as its nearest bundle dir, or
    //   * it compiles to file-scope markup, so the `.st`-only synthesis below
    //     turns it into a page.
    // Everything else is a fragment -- `modules/theme.st` is styles, not a
    // route -- and writing its 700KB+ runtime bundle would ship dead weight
    // under a URL nothing links to. Same judgement the dev server makes when
    // `/modules/theme` renders an empty body.
    let authored: std::collections::BTreeSet<&Path> =
        layout.routes.iter().map(|r| r.html_rel.as_path()).collect();
    let referenced_by_html: std::collections::BTreeSet<&Path> = layout
        .routes
        .iter()
        .map(|r| r.bundle_dir.as_path())
        .collect();

    // Every entry is compiled, page or not: a fragment that fails to compile
    // breaks the pages importing it, and silence there is how a broken build
    // looks green. `bundle.diagnostics` carries ERROR-severity pipeline errors
    // only (the compiler folds migration/hint channels separately), so a
    // non-empty list means the compiled output is not faithful — e.g. an
    // `%emit` block failed to parse (FEAT-170) and the primitive emitted
    // nothing. Such an entry must FAIL the export, never ship as a warning.
    // Collected up front so the failure names every broken route at once.
    let mut compile_errors: Vec<String> = Vec::new();
    for bundle in &bundles {
        for d in &bundle.diagnostics {
            compile_errors.push(format!(
                "Route '{}' has compile diagnostic: {}",
                bundle.dir.display(),
                d
            ));
        }
    }
    if !compile_errors.is_empty() {
        return Err(super::ExportError::Compile {
            message: compile_errors.join("\n"),
        });
    }

    // Paths the compiler owns: a static asset copied over one of these would
    // silently replace fresh compilation with a stale artifact.
    let mut compiler_outputs: std::collections::HashSet<std::path::PathBuf> =
        std::collections::HashSet::new();

    for bundle in &bundles {
        let is_page =
            referenced_by_html.contains(bundle.dir.as_path()) || !bundle.html.trim().is_empty();
        if !is_page {
            continue;
        }

        let out = config.output_dir.join(&bundle.dir);
        std::fs::create_dir_all(&out).map_err(|e| ExportError::Io {
            path: out.clone(),
            source: e,
        })?;

        let js_path = out.join("spacetime.js");
        let css_path = out.join("spacetime.css");
        compiler_outputs.insert(js_path.clone());
        compiler_outputs.insert(css_path.clone());
        std::fs::write(&js_path, &bundle.js).map_err(|e| ExportError::Io {
            path: js_path,
            source: e,
        })?;
        std::fs::write(&css_path, &bundle.css).map_err(|e| ExportError::Io {
            path: css_path,
            source: e,
        })?;

        report.total_size += bundle.js.len() as u64 + bundle.css.len() as u64;
    }

    for route in &layout.routes {
        let html_path = config.site_dir.join(&route.html_rel);
        let html = std::fs::read_to_string(&html_path).map_err(|e| ExportError::Io {
            path: html_path,
            source: e,
        })?;

        let css_href = super::path::absolute_href(&route.bundle_dir, "spacetime.css");
        let js_href = super::path::absolute_href(&route.bundle_dir, "spacetime.js");
        let processed = super::html::process_html(&html, &css_href, &js_href);

        let out_path = config.output_dir.join(&route.html_rel);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ExportError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        std::fs::write(&out_path, &processed).map_err(|e| ExportError::Io {
            path: out_path,
            source: e,
        })?;

        report.pages_exported += 1;
        report.total_size += processed.len() as u64;
    }

    // Full-Spacetime (`.st`-only) routes: a discovered entry with NO authored
    // `<dir>/index.html` produces no page above. Synthesize one from the
    // compiler-produced body markup (file-scope `<tag>` literals + FEAT-078
    // hydration markers) wrapped in the SHARED page shell -- the same shell the
    // dev server uses (FUP-039), so dev and export agree. This is what makes
    // file-scope reactive markup actually ship in a static `cargo build`, for
    // the root `index.st` AND for every clean-path sibling page the dev server
    // serves (`brand.st` -> `/brand/`, BUG-218).
    for bundle in &bundles {
        let index_rel = bundle.dir.join("index.html");
        if authored.contains(index_rel.as_path()) {
            // GH-21: an authored `index.html` wins over this `.st` entry's own file-scope
            // body markup VERBATIM and EXCLUSIVELY. That override must be EXPLICIT — a silent
            // escape hatch that beats the primary path turns a missing feature into data loss.
            // Surface a warning whenever the `.st` actually produced body markup so the author
            // knows their markup is NOT being shipped.
            if !bundle.html.trim().is_empty() {
                report.warnings.push(format!(
                    "{}: an authored index.html overrides this entry's file-scope body markup; the .st body is NOT exported (remove the authored index.html to synthesize the .st page)",
                    index_rel.display()
                ));
            }
            continue; // an authored .html already produced this page
        }
        if bundle.html.trim().is_empty() {
            continue; // no file-scope markup -> nothing to synthesize (pure CSS/behaviour .st)
        }
        let css_href = super::path::absolute_href(&bundle.dir, "spacetime.css");
        let js_href = super::path::absolute_href(&bundle.dir, "spacetime.js");
        let page =
            crate::html::page_shell::render_page_shell(&crate::html::page_shell::PageShell {
                title: "Spacetime",
                css_href: &css_href,
                js_href: &js_href,
                body_html: &bundle.html,
                lang: "en",
                tail: "",
            });
        let out_path = config.output_dir.join(&index_rel);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ExportError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        std::fs::write(&out_path, &page).map_err(|e| ExportError::Io {
            path: out_path,
            source: e,
        })?;
        report.pages_exported += 1;
        report.total_size += page.len() as u64;
    }

    for asset in &layout.assets {
        let src = config.site_dir.join(asset);
        let dst = config.output_dir.join(asset);
        // A stale build artifact left in the project (an old spacetime.js/css)
        // must NEVER overwrite the bundle this export just compiled — that is
        // how a months-old `// Primitive not found` comment shipped inside a
        // "fresh" bundle while every gate reported green. Refuse the
        // overwrite LOUDLY; keeping the compiled output is never ambiguous.
        if compiler_outputs.contains(&dst) {
            eprintln!(
                "warning: refusing to copy stale asset `{}` over the compiled bundle at `{}` — delete the stale file",
                src.display(),
                dst.display()
            );
            continue;
        }
        copy_tree(&src, &dst, &mut report)?;
    }

    Ok(report)
}

fn copy_tree(src: &Path, dst: &Path, report: &mut ExportReport) -> Result<(), ExportError> {
    // W0-1 (PLAN-005): layout.assets now names individual FILES (the
    // dev-serve-parity static set), not just wholesale directories.
    if src.is_file() {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ExportError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        let size = std::fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        std::fs::copy(src, dst).map_err(|e| ExportError::Io {
            path: src.to_path_buf(),
            source: e,
        })?;
        report.files_copied += 1;
        report.total_size += size;
        return Ok(());
    }

    std::fs::create_dir_all(dst).map_err(|e| ExportError::Io {
        path: dst.to_path_buf(),
        source: e,
    })?;

    for entry in std::fs::read_dir(src).map_err(|e| ExportError::Io {
        path: src.to_path_buf(),
        source: e,
    })? {
        let entry = entry.map_err(|e| ExportError::Io {
            path: src.to_path_buf(),
            source: e,
        })?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_tree(&src_path, &dst_path, report)?;
        } else {
            let size = std::fs::metadata(&src_path).map(|m| m.len()).unwrap_or(0);
            std::fs::copy(&src_path, &dst_path).map_err(|e| ExportError::Io {
                path: src_path.clone(),
                source: e,
            })?;
            report.files_copied += 1;
            report.total_size += size;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_st(tag: &str) -> String {
        format!(
            ".test-page-{} {{\n    display: block;\n}}\n\nbody {{\n    @on &.click {{ $opacity <- 1; }}\n}}\n",
            tag
        )
    }

    fn minimal_html(tag: &str) -> String {
        format!(
            "<!DOCTYPE html><html><head></head><body><!-- {} --></body></html>",
            tag
        )
    }

    fn dir_size(dir: &Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                } else if path.is_dir() {
                    total += dir_size(&path);
                }
            }
        }
        total
    }

    #[test]
    fn writes_each_bundle_to_correct_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::create_dir(site.join("about")).unwrap();

        std::fs::write(site.join("index.html"), minimal_html("root")).unwrap();
        std::fs::write(site.join("index.st"), minimal_st("root")).unwrap();
        std::fs::write(site.join("about/index.html"), minimal_html("about")).unwrap();
        std::fs::write(site.join("about/index.st"), minimal_st("about")).unwrap();

        let output = tmp.path().join("output");
        let config = ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        };
        let _ = run(&config);

        assert!(output.join("spacetime.js").exists());
        assert!(output.join("spacetime.css").exists());
        assert!(
            !std::fs::read_to_string(output.join("spacetime.js"))
                .unwrap()
                .is_empty()
        );
        assert!(
            !std::fs::read_to_string(output.join("spacetime.css"))
                .unwrap()
                .is_empty()
        );

        assert!(output.join("about/spacetime.js").exists());
        assert!(output.join("about/spacetime.css").exists());
        assert!(
            !std::fs::read_to_string(output.join("about/spacetime.js"))
                .unwrap()
                .is_empty()
        );
        assert!(
            !std::fs::read_to_string(output.join("about/spacetime.css"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn writes_each_html_with_absolute_hrefs() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::create_dir(site.join("about")).unwrap();

        std::fs::write(site.join("index.html"), minimal_html("root")).unwrap();
        std::fs::write(site.join("index.st"), minimal_st("root")).unwrap();
        std::fs::write(site.join("about/index.html"), minimal_html("about")).unwrap();
        std::fs::write(site.join("about/index.st"), minimal_st("about")).unwrap();

        let output = tmp.path().join("output");
        let config = ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        };
        let _ = run(&config);

        let root_html = std::fs::read_to_string(output.join("index.html")).unwrap();
        assert!(root_html.contains(r#"href="/spacetime.css""#));
        assert!(root_html.contains(r#"src="/spacetime.js""#));
        assert!(!root_html.contains("href=\"../"));

        let about_html = std::fs::read_to_string(output.join("about/index.html")).unwrap();
        assert!(about_html.contains(r#"href="/about/spacetime.css""#));
        assert!(about_html.contains(r#"src="/about/spacetime.js""#));
        assert!(!about_html.contains("href=\"../"));
    }

    #[test]
    fn copies_asset_trees_recursively() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::create_dir_all(site.join("assets")).unwrap();
        std::fs::create_dir_all(site.join("data")).unwrap();

        std::fs::write(site.join("index.html"), minimal_html("root")).unwrap();
        std::fs::write(site.join("index.st"), minimal_st("root")).unwrap();
        std::fs::write(site.join("assets/img.png"), "asset").unwrap();
        std::fs::write(site.join("data/foo.json"), "{}").unwrap();

        let output = tmp.path().join("output");
        let config = ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        };
        let _ = run(&config);

        assert_eq!(
            std::fs::read_to_string(output.join("assets/img.png")).unwrap(),
            "asset"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("data/foo.json")).unwrap(),
            "{}"
        );
    }

    #[test]
    fn report_counts_pages_and_files() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::create_dir(site.join("about")).unwrap();
        std::fs::create_dir_all(site.join("assets")).unwrap();
        std::fs::create_dir_all(site.join("data")).unwrap();

        std::fs::write(site.join("index.html"), minimal_html("root")).unwrap();
        std::fs::write(site.join("index.st"), minimal_st("root")).unwrap();
        std::fs::write(site.join("about/index.html"), minimal_html("about")).unwrap();
        std::fs::write(site.join("about/index.st"), minimal_st("about")).unwrap();
        // W0-1: the static set is dev-serve parity -- servable
        // extensions only (.txt was never served by the dev server).
        std::fs::write(site.join("assets/a.png"), "a").unwrap();
        std::fs::write(site.join("data/b.json"), "b").unwrap();

        let output = tmp.path().join("output");
        let config = ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        };
        let report = run(&config).unwrap();

        assert_eq!(report.pages_exported, 2);
        assert!(report.files_copied >= 2);
        assert!(report.total_size > 0);
    }

    #[test]
    fn report_total_size_matches_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();

        std::fs::write(site.join("index.html"), minimal_html("root")).unwrap();
        std::fs::write(site.join("index.st"), minimal_st("root")).unwrap();

        let output = tmp.path().join("output");
        let config = ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        };
        let report = run(&config).unwrap();

        let disk_size = dir_size(&output);
        assert_eq!(report.total_size, disk_size);
    }

    #[test]
    fn is_deterministic_across_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::create_dir(site.join("about")).unwrap();

        std::fs::write(site.join("index.html"), minimal_html("root")).unwrap();
        std::fs::write(site.join("index.st"), minimal_st("root")).unwrap();
        std::fs::write(site.join("about/index.html"), minimal_html("about")).unwrap();
        std::fs::write(site.join("about/index.st"), minimal_st("about")).unwrap();

        let output1 = tmp.path().join("output1");
        let config1 = ExportConfig {
            site_dir: site.clone(),
            output_dir: output1,
            minify: false,
        };
        let r1 = run(&config1).unwrap();

        let output2 = tmp.path().join("output2");
        let config2 = ExportConfig {
            site_dir: site,
            output_dir: output2,
            minify: false,
        };
        let r2 = run(&config2).unwrap();

        assert_eq!(r1.pages_exported, r2.pages_exported);
        assert_eq!(r1.files_copied, r2.files_copied);
        assert_eq!(r1.total_size, r2.total_size);
    }
}

#[cfg(test)]
mod feat078_export_tests {
    use super::*;

    /// FUP-039: a full-Spacetime route (index.st with file-scope markup, NO authored index.html)
    /// must export a real index.html carrying the file-scope markup + hydration markers wrapped
    /// in the shared page shell, plus the bundle hrefs.
    #[test]
    fn st_only_route_synthesizes_page_from_compiled_html() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        // index.st with file-scope markup + a reactive hole; NO index.html authored.
        std::fs::write(
            site.join("index.st"),
            "$count number: 7;\n<main><p>Count <span>`$count`</span></p></main>\n",
        )
        .unwrap();

        let output = tmp.path().join("output");
        let config = ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        };
        let report = run(&config).unwrap();

        // A page was synthesized for the st-only route.
        let index = output.join("index.html");
        assert!(index.exists(), "st-only route must export an index.html");
        let html = std::fs::read_to_string(&index).unwrap();
        // SSG: the file-scope markup + the hole's static initial value (marker) are present.
        assert!(html.contains("<main>"), "file-scope markup: {html}");
        assert!(
            html.contains("data-st-hole=") && html.contains(">7</span>"),
            "hydration marker + initial: {html}"
        );
        // The shared shell wired the production bundle.
        assert!(
            html.contains(r#"href="/spacetime.css""#),
            "css href: {html}"
        );
        assert!(html.contains(r#"src="/spacetime.js""#), "js href: {html}");
        // The bundle carries the hole-hydration JS.
        let js = std::fs::read_to_string(output.join("spacetime.js")).unwrap();
        assert!(
            js.contains("local:count:updated"),
            "hole hydration in bundle: {js}"
        );
        assert_eq!(report.pages_exported, 1, "exactly one page");
    }

    /// An authored index.html for the same dir takes precedence: no double-emit / overwrite.
    #[test]
    fn authored_html_wins_over_synthesis() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.st"), "<main>generated</main>\n").unwrap();
        std::fs::write(
            site.join("index.html"),
            "<!DOCTYPE html><html><head></head><body><h1>authored</h1></body></html>",
        )
        .unwrap();

        let output = tmp.path().join("output");
        let config = ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        };
        let report = run(&config).unwrap();

        let html = std::fs::read_to_string(output.join("index.html")).unwrap();
        assert!(html.contains("authored"), "authored html must win: {html}");
        assert!(
            !html.contains("generated"),
            "must NOT synthesize over an authored route: {html}"
        );
        assert_eq!(
            report.pages_exported, 1,
            "exactly one page (no double-emit)"
        );
    }

    /// A pure-CSS/behaviour .st (no file-scope markup) synthesizes NO page (nothing to show).
    #[test]
    fn no_markup_st_synthesizes_no_page() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.st"), ".hero { color: red; }\n").unwrap();

        let output = tmp.path().join("output");
        let config = ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        };
        let report = run(&config).unwrap();
        assert!(
            !output.join("index.html").exists(),
            "no markup -> no synthesized page"
        );
        assert_eq!(report.pages_exported, 0);
    }
}

/// BUG-218: the static export must emit the SAME set of pages the dev server
/// serves. `server.rs::try_serve_st_page` (FEAT-093) resolves a clean path
/// `/p` to `p.st` then `p/index.st`; before this suite the export only knew
/// about `<dir>/index.st`, so every sibling page compiled to nothing and 404'd
/// once deployed while working perfectly under `spacetime serve`.
#[cfg(test)]
mod bug218_clean_path_export_tests {
    use super::*;

    /// The headline case: `brand.st` beside `index.st` ships as `/brand/`.
    #[test]
    fn sibling_st_page_exports_as_its_own_route() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.st"), "<main><h1>home</h1></main>\n").unwrap();
        std::fs::write(site.join("brand.st"), "<main><h1>brand book</h1></main>\n").unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        assert_eq!(report.pages_exported, 2, "root + the sibling page");
        let brand = output.join("brand").join("index.html");
        assert!(brand.exists(), "brand.st must export to brand/index.html");
        let html = std::fs::read_to_string(&brand).unwrap();
        assert!(html.contains("brand book"), "page markup: {html}");
        // Its bundle is co-located and referenced absolutely, so the page works
        // at `/brand/` on a static host with no rewrite rules.
        assert!(
            html.contains(r#"href="/brand/spacetime.css""#),
            "css href: {html}"
        );
        assert!(
            html.contains(r#"src="/brand/spacetime.js""#),
            "js href: {html}"
        );
        assert!(output.join("brand").join("spacetime.js").exists());
        assert!(output.join("brand").join("spacetime.css").exists());
    }

    /// A nested sibling (`docs/guide.st`) routes to `docs/guide/`, matching the
    /// dev server's resolution at any depth.
    #[test]
    fn nested_sibling_st_page_exports_at_its_depth() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(site.join("docs")).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        std::fs::write(site.join("docs/guide.st"), "<main>the guide</main>\n").unwrap();

        let output = tmp.path().join("output");
        run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        let guide = output.join("docs").join("guide").join("index.html");
        assert!(guide.exists(), "docs/guide.st -> docs/guide/index.html");
        let html = std::fs::read_to_string(&guide).unwrap();
        assert!(html.contains("the guide"));
        assert!(html.contains(r#"src="/docs/guide/spacetime.js""#), "{html}");
    }

    /// Dev-server candidate ORDER is the contract: `p.st` wins over
    /// `p/index.st`, so export and serve can never disagree about which file
    /// answers `/p`.
    #[test]
    fn sibling_st_wins_over_dir_index_st() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(site.join("brand")).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        std::fs::write(site.join("brand.st"), "<main>from sibling</main>\n").unwrap();
        std::fs::write(site.join("brand/index.st"), "<main>from dir</main>\n").unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        let html = std::fs::read_to_string(output.join("brand").join("index.html")).unwrap();
        assert!(html.contains("from sibling"), "sibling must win: {html}");
        assert!(!html.contains("from dir"), "one route, one entry: {html}");
        assert_eq!(report.pages_exported, 2, "one page per ROUTE, not per file");
    }

    /// A module fragment (styles/behaviour only, e.g. `modules/theme.st`) is not
    /// a page: no HTML, and critically no 700KB+ runtime bundle written under a
    /// URL nothing links to.
    #[test]
    fn module_fragment_exports_neither_page_nor_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(site.join("modules")).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        std::fs::write(site.join("modules/theme.st"), ".hero { color: red; }\n").unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        assert_eq!(report.pages_exported, 1, "only the root page");
        assert!(!output.join("modules").join("index.html").exists());
        assert!(
            !output.join("modules").join("spacetime.js").exists(),
            "a fragment must not ship a runtime bundle"
        );
    }

    /// A `_`-prefixed sibling is a private partial by the same naming
    /// convention the directory denylist uses -- never a route, never compiled.
    #[test]
    fn underscore_prefixed_sibling_is_not_a_route() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        std::fs::write(site.join("_nav.st"), "<nav>private</nav>\n").unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        assert_eq!(report.pages_exported, 1);
        assert!(!output.join("_nav").exists(), "_nav.st is a partial");
    }

    /// An authored `<p>/index.html` still wins over synthesis for the sibling's
    /// route -- the precedence rule is unchanged by clean-path discovery.
    #[test]
    fn authored_html_still_wins_for_a_sibling_route() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(site.join("brand")).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        std::fs::write(site.join("brand.st"), "<main>generated</main>\n").unwrap();
        std::fs::write(
            site.join("brand/index.html"),
            "<!DOCTYPE html><html><head></head><body><h1>authored</h1></body></html>",
        )
        .unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        let html = std::fs::read_to_string(output.join("brand").join("index.html")).unwrap();
        assert!(html.contains("authored"), "authored wins: {html}");
        assert!(!html.contains("generated"), "no double-emit: {html}");
        assert_eq!(report.pages_exported, 2);
    }

    /// BUG-218 second defect: the export never passed `site_dir` to the
    /// compiler, so every build-time file read resolved against the process CWD
    /// instead of the served site. `@doc(src:)` therefore inlined a "Document
    /// not found" stub into the DEPLOYED page while `spacetime serve` rendered
    /// the real markdown -- a silent, green-looking divergence.
    #[test]
    fn doc_src_resolves_against_the_site_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(site.join("brand")).unwrap();
        std::fs::write(
            site.join("brand/GUIDELINES.md"),
            "# Evidence Brutalism\n\nEvery surface is a receipt.\n",
        )
        .unwrap();
        std::fs::write(
            site.join("index.st"),
            "@import \"stdlib/md\";\n<main><article class=\"doc\"></article></main>\n\
             .doc { @doc(src: \"brand/GUIDELINES.md\") }\n",
        )
        .unwrap();

        let output = tmp.path().join("output");
        run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        let js = std::fs::read_to_string(output.join("spacetime.js")).unwrap();
        assert!(
            !js.contains("Document not found"),
            "@doc must resolve against site_dir, not the CWD"
        );
        assert!(
            js.contains("Every surface is a receipt"),
            "the markdown bytes must be inlined into the shipped bundle"
        );
    }
}

/// BUG-220: SIP-002 made literate `.st.md` documents first-class pages and
/// taught the DEV SERVER to resolve them (`try_serve_st_page`'s four-candidate
/// array). Export discovery still matched `.st` only, so converting a page to
/// literate form silently removed it from the build — the same dev/deploy
/// divergence as BUG-218, reintroduced by a new file kind.
#[cfg(test)]
mod bug220_literate_export_tests {
    use super::*;

    /// A literate sibling ships exactly like an `.st` one.
    #[test]
    fn literate_sibling_exports_as_its_own_route() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        std::fs::write(
            site.join("brand.st.md"),
            "# Brand\n\nProse the page owns.\n\n```st\n<main><h1>brand book</h1></main>\n```\n",
        )
        .unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        assert_eq!(report.pages_exported, 2, "root + the literate page");
        let brand = output.join("brand").join("index.html");
        assert!(
            brand.exists(),
            "brand.st.md must export to brand/index.html, NOT brand.st/"
        );
        assert!(
            !output.join("brand.st").exists(),
            "the `.st.md` stem is `brand` -- a naive file_stem would route to brand.st/"
        );
        let html = std::fs::read_to_string(&brand).unwrap();
        assert!(html.contains("brand book"), "tangled markup: {html}");
        assert!(
            html.contains(r#"src="/brand/spacetime.js""#),
            "bundle href: {html}"
        );
        // The PROSE is served too (SSG markdown): the literate section must
        // carry its rendered heading in the shipped HTML, not an empty mount
        // that only the browser fills. A crawler / no-JS client / first paint
        // sees the document, which is the whole point of a document format.
        // Requires the `headless` build-time engine (default feature).
        #[cfg(feature = "headless")]
        assert!(
            html.contains("<h1>Brand</h1>") && html.contains("Prose the page owns."),
            "literate prose must be rendered into the served HTML at build time: {html}"
        );
    }

    /// A `.st.md`-only root is a valid site: discovery must not demand an
    /// `index.st` that a literate project never has.
    #[test]
    fn literate_root_index_is_a_valid_site() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(
            site.join("index.st.md"),
            "# Home\n\n```st\n<main>literate home</main>\n```\n",
        )
        .unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        assert_eq!(report.pages_exported, 1);
        let html = std::fs::read_to_string(output.join("index.html")).unwrap();
        assert!(html.contains("literate home"), "{html}");
    }

    /// Candidate PRIORITY matches the dev server's array order exactly:
    /// `p.st` > `p.st.md` > `p/index.st` > `p/index.st.md`. One route, one
    /// entry, regardless of which files happen to coexist.
    #[test]
    fn candidate_priority_matches_the_dev_server_order() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(site.join("page")).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        // All four candidates for route `page`, worst to best.
        std::fs::write(
            site.join("page/index.st.md"),
            "```st\n<main>dir literate</main>\n```\n",
        )
        .unwrap();
        std::fs::write(site.join("page/index.st"), "<main>dir plain</main>\n").unwrap();
        std::fs::write(
            site.join("page.st.md"),
            "```st\n<main>sibling literate</main>\n```\n",
        )
        .unwrap();
        std::fs::write(site.join("page.st"), "<main>sibling plain</main>\n").unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        let html = std::fs::read_to_string(output.join("page").join("index.html")).unwrap();
        assert!(
            html.contains("sibling plain"),
            "`page.st` is the highest-priority candidate: {html}"
        );
        for loser in ["sibling literate", "dir plain", "dir literate"] {
            assert!(!html.contains(loser), "{loser} must not win: {html}");
        }
        assert_eq!(report.pages_exported, 2, "one page per ROUTE, not per file");
    }

    /// With no plain `.st` sibling, the literate one wins over the directory
    /// forms -- the second rung of the same ladder.
    #[test]
    fn literate_sibling_beats_directory_index() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(site.join("page")).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        std::fs::write(site.join("page/index.st"), "<main>dir plain</main>\n").unwrap();
        std::fs::write(
            site.join("page.st.md"),
            "```st\n<main>sibling literate</main>\n```\n",
        )
        .unwrap();

        let output = tmp.path().join("output");
        run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        let html = std::fs::read_to_string(output.join("page").join("index.html")).unwrap();
        assert!(html.contains("sibling literate"), "{html}");
        assert!(!html.contains("dir plain"), "{html}");
    }

    /// A `_`-prefixed literate partial is a fragment by NAME, exactly as its
    /// `.st` counterpart is.
    #[test]
    fn underscore_prefixed_literate_sibling_is_not_a_route() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        std::fs::write(
            site.join("_notes.st.md"),
            "# Notes\n\n```st\n<aside>private</aside>\n```\n",
        )
        .unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        assert_eq!(report.pages_exported, 1);
        assert!(!output.join("_notes").exists());
    }

    /// A plain `.md` file is documentation, NOT a page: only the `.st.md`
    /// double extension marks a literate Spacetime document.
    #[test]
    fn plain_markdown_is_not_a_page() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.st"), "<main>home</main>\n").unwrap();
        std::fs::write(site.join("README.md"), "# Just docs\n").unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        assert_eq!(report.pages_exported, 1, "README.md is not a route");
        assert!(!output.join("README").exists());
    }
}

/// BUG-227: the export must be CWD-INDEPENDENT. `compile_bundles` passed the
/// SITE dir where `from_file` wants the WORKSPACE ROOT, so `resolve_imports`
/// looked for the toolchain's stdlib inside the user's project. Every
/// `@import "stdlib/..."` failed, each route degraded to an EMPTY bundle behind
/// a warning, and `spacetime-host push` (which necessarily runs from the user's
/// project directory) uploaded a deployment whose `/spacetime.js` was 0 bytes
/// while printing success.
#[cfg(test)]
mod bug227_toolchain_root_tests {
    use super::*;

    /// Building a site NESTED in a toolchain checkout must resolve stdlib from
    /// the checkout, no matter what the process CWD is. The bundle being
    /// non-empty is the whole assertion: an unresolved stdlib import yields
    /// zero bytes, which is precisely what shipped.
    #[test]
    fn stdlib_resolves_from_the_enclosing_checkout_not_the_site() {
        let tmp = tempfile::tempdir().unwrap();
        // A fake toolchain checkout: `stdlib/` with one importable module.
        // `stdlib/primitives/` is what marks a directory as a toolchain root
        // (BUG-350 — a bare `stdlib/` is satisfied by any folder that merely has
        // that name, e.g. `docs/stdlib/`, which silently produced empty bundles).
        let checkout = tmp.path().join("toolchain");
        std::fs::create_dir_all(checkout.join("stdlib").join("primitives")).unwrap();
        std::fs::write(
            checkout.join("stdlib").join("tokens.st"),
            ".from-stdlib { color: rebeccapurple; }\n",
        )
        .unwrap();

        // The site lives INSIDE it, exactly as `projects/<name>/` does, and
        // imports that stdlib module.
        let site = checkout.join("projects").join("mysite");
        std::fs::create_dir_all(&site).unwrap();
        std::fs::write(
            site.join("index.st"),
            "@import \"stdlib/tokens\"\n<main><h1>home</h1></main>\n",
        )
        .unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        assert!(
            report.warnings.is_empty(),
            "a resolvable stdlib import must not warn: {:?}",
            report.warnings
        );
        let css = std::fs::read_to_string(output.join("spacetime.css")).unwrap();
        assert!(
            css.contains("rebeccapurple"),
            "the stdlib module must be resolved and compiled in: {css}"
        );
        // The CSS above is the proof the import resolved. Assert the PAGE
        // shipped too: an unresolved import fails the whole route compile, so
        // there would be no synthesized page at all -- the 0-byte-bundle
        // failure and the missing-page failure are the same event.
        assert_eq!(report.pages_exported, 1, "the route must compile and ship");
        let html = std::fs::read_to_string(output.join("index.html")).unwrap();
        assert!(html.contains("home"), "page markup: {html}");
    }

    /// A site with NO enclosing checkout still builds: the embedded stdlib
    /// serves those imports, so a distributed binary is not broken by the fix.
    #[test]
    fn standalone_site_without_a_checkout_still_builds() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("standalone");
        std::fs::create_dir_all(&site).unwrap();
        std::fs::write(site.join("index.st"), "<main>standalone</main>\n").unwrap();

        let output = tmp.path().join("output");
        let report = run(&ExportConfig {
            site_dir: site,
            output_dir: output.clone(),
            minify: false,
        })
        .unwrap();

        assert_eq!(report.pages_exported, 1);
        let html = std::fs::read_to_string(output.join("index.html")).unwrap();
        assert!(html.contains("standalone"), "{html}");
    }
}
