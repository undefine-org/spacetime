//! Shared "package a site directory into a Spacetime Host push archive"
//! logic (PLAN-075) -- the ONE implementation of tar+gzip + widget-tail
//! splicing, used by BOTH:
//!   - `commercial/host`'s `spacetime-host-cli::main::package_directory`
//!     (the real `spacetime host push` terminal command), via a path
//!     dependency on this crate (`commercial/host` -> `verse`, never the
//!     reverse -- `verse` stays buildable/testable with zero knowledge
//!     that `commercial/host` exists on disk)
//!   - `verse`'s own local dev-server push route (`POST
//!     /__spacetime/host/push`, `src/server.rs`), letting the `__host__`
//!     widget's "Push preview" button work directly from the pill
//!
//! Before this module existed, this exact tar+gzip+splice loop lived
//! ONLY in `spacetime-host-cli::main::package_directory` -- lifted here
//! VERBATIM (same walk order, same per-file HTML-splice condition, same
//! tar header shape) so a second, drifting reimplementation never has to
//! be written to give the local dev server its own push capability.
//!
//! Deliberately does NOT touch sentinel substitution (the
//! `__SPACETIME_HOST_PROJECT_ID__`/`__SPACETIME_HOST_API_BASE__`/
//! `__SPACETIME_HOST_BRIDGE_BASE_URL__` replacement): each caller
//! performs that itself, with its own already-correct trust boundary
//! (`compile_host_widget_raw` in `verse`; `widget_bundle::render` in the
//! CLI) -- this module only accepts the ALREADY-SUBSTITUTED widget
//! assets and packages them.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Packages `dir` into a gzipped tar archive matching exactly what
/// `spacetime-host-server`'s own `archive.rs` (`extract_and_upload`)
/// validates on the receiving end: relative paths, no symlinks, regular
/// files only.
///
/// Every `.html`/`.htm` file under `dir` gets `widget_html_tail` spliced
/// before its own `</body>` (a file without one is copied through
/// byte-for-byte, unmodified -- e.g. a non-HTML asset, or a genuinely
/// bodyless HTML fragment). The widget's own JS/CSS are then added as
/// two additional archive entries at `__spacetime/host/runtime.js` /
/// `__spacetime/host/styles.css`, so a deployed page's injected
/// `<script src="/__spacetime/host/runtime.js">` tag resolves against
/// the STATIC archive itself (a deployed site has no per-request compile
/// step to serve that path dynamically).
pub fn package_site_archive(
    dir: &Path,
    widget_js: &str,
    widget_css: &str,
    widget_html_tail: &str,
) -> Result<Vec<u8>, String> {
    let mut tar_bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);

        for entry in walkdir_files(dir).map_err(|e| e.to_string())? {
            let rel_path = entry
                .strip_prefix(dir)
                .unwrap_or(&entry)
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&entry).map_err(|e| e.to_string())?;
            let is_html = rel_path.ends_with(".html") || rel_path.ends_with(".htm");
            let final_bytes = if is_html {
                match std::str::from_utf8(&bytes) {
                    Ok(text) if text.contains("</body>") => text
                        .replacen("</body>", &format!("{widget_html_tail}</body>"), 1)
                        .into_bytes(),
                    _ => bytes,
                }
            } else {
                bytes
            };
            append_tar_entry(&mut builder, &rel_path, &final_bytes)?;
        }

        append_tar_entry(
            &mut builder,
            "__spacetime/host/runtime.js",
            widget_js.as_bytes(),
        )?;
        append_tar_entry(
            &mut builder,
            "__spacetime/host/styles.css",
            widget_css.as_bytes(),
        )?;

        builder.finish().map_err(|e| e.to_string())?;
    }

    let mut gz_bytes = Vec::new();
    {
        let mut encoder =
            flate2::write::GzEncoder::new(&mut gz_bytes, flate2::Compression::default());
        encoder.write_all(&tar_bytes).map_err(|e| e.to_string())?;
        encoder.finish().map_err(|e| e.to_string())?;
    }

    Ok(gz_bytes)
}

fn append_tar_entry(
    builder: &mut tar::Builder<&mut Vec<u8>>,
    path: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, path, bytes)
        .map_err(|e| e.to_string())
}

/// Recursively lists every REGULAR file under `dir` (no symlinks -- the
/// receiving server's own `archive.rs` validation is the real trust
/// boundary here). Hand-rolled (not `tar::Builder::append_dir_all`)
/// because per-file HTML rewriting requires reading each file's bytes
/// before archiving, which `append_dir_all` doesn't expose a hook for.
fn walkdir_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                out.push(path);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ports the exact assertions
    /// `spacetime-host-cli::main::package_directory_injects_widget_into_html_and_bundles_its_assets`
    /// made against the pre-extraction implementation, run here against
    /// the shared function directly -- this is the regression guard that
    /// the extraction changed nothing observable.
    #[test]
    fn package_site_archive_injects_widget_and_bundles_assets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.html"),
            "<html><body><h1>Hi</h1></body></html>",
        )
        .unwrap();
        std::fs::write(dir.path().join("style.css"), "body{color:red}").unwrap();

        let widget_tail =
            "\n<!-- widget --><script src=\"/__spacetime/host/runtime.js\"></script>\n";
        let archive_bytes = package_site_archive(
            &dir.path().to_path_buf(),
            "console.log('js: proj_test api: https://api.example.com')",
            "body{}",
            widget_tail,
        )
        .expect("package_site_archive should succeed");

        let tar_bytes = {
            let mut decoder = flate2::read::GzDecoder::new(&archive_bytes[..]);
            let mut out = Vec::new();
            std::io::Read::read_to_end(&mut decoder, &mut out).unwrap();
            out
        };
        let mut archive = tar::Archive::new(&tar_bytes[..]);
        let mut found_html = false;
        let mut found_widget_js = false;
        let mut found_widget_css = false;
        let mut found_style_css = false;
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_string_lossy().to_string();
            let mut content = String::new();
            let _ = std::io::Read::read_to_string(&mut entry, &mut content);
            match path.as_str() {
                "index.html" => {
                    found_html = true;
                    assert!(
                        content.contains("st-host-widget") || content.contains("widget"),
                        "index.html must have the widget tail injected: {content}"
                    );
                    assert!(
                        content.contains("<h1>Hi</h1>"),
                        "original page content must be preserved"
                    );
                }
                "style.css" => {
                    found_style_css = true;
                    assert_eq!(
                        content, "body{color:red}",
                        "unrelated files must pass through unmodified"
                    );
                }
                "__spacetime/host/runtime.js" => {
                    found_widget_js = true;
                    assert!(content.contains("proj_test"));
                }
                "__spacetime/host/styles.css" => found_widget_css = true,
                _ => {}
            }
        }
        assert!(found_html, "index.html must be present in the archive");
        assert!(found_style_css, "style.css must be present in the archive");
        assert!(found_widget_js, "the widget's runtime.js must be bundled");
        assert!(found_widget_css, "the widget's styles.css must be bundled");
    }

    #[test]
    fn package_site_archive_leaves_non_html_files_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data.json"), "{\"a\":1}").unwrap();

        let archive_bytes =
            package_site_archive(&dir.path().to_path_buf(), "", "", "<tail>").unwrap();
        let tar_bytes = {
            let mut decoder = flate2::read::GzDecoder::new(&archive_bytes[..]);
            let mut out = Vec::new();
            std::io::Read::read_to_end(&mut decoder, &mut out).unwrap();
            out
        };
        let mut archive = tar::Archive::new(&tar_bytes[..]);
        let mut found = false;
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_string_lossy().to_string();
            if path == "data.json" {
                found = true;
                let mut content = String::new();
                std::io::Read::read_to_string(&mut entry, &mut content).unwrap();
                assert_eq!(content, "{\"a\":1}");
            }
        }
        assert!(found, "data.json must be present in the archive");
    }

    /// W0-1 (PLAN-005) regression guard: the push pipeline packages the
    /// EXPORTED site (via `crate::export::export_site`), never the source
    /// tree. Before the fix, the dev-server push route tarred the source
    /// directory itself -- .st sources shipped to the edge while every
    /// /spacetime.js reference 404'd. This test drives the exact
    /// composition `host_push_handler` now performs.
    #[test]
    fn package_built_site_has_compiled_assets_and_no_sources() {
        let site = tempfile::tempdir().unwrap();
        std::fs::write(
            site.path().join("index.html"),
            "<html><head></head><body><h1>Hi</h1></body></html>",
        )
        .unwrap();
        std::fs::write(site.path().join("index.st"), "").unwrap();
        // A source file that must NEVER cross the wire.
        std::fs::write(site.path().join("secret-notes.st"), "$secret: 42").unwrap();

        let out = tempfile::tempdir().unwrap();
        let built_dir = out.path().join("site");
        crate::export::export_site(&crate::export::ExportConfig {
            site_dir: site.path().to_path_buf(),
            output_dir: built_dir.clone(),
            minify: true,
        })
        .expect("export_site should succeed on a minimal site");

        let archive_bytes = package_site_archive(&built_dir, "WIDGET_JS", "WIDGET_CSS", "<tail>")
            .expect("packaging the built site should succeed");

        let tar_bytes = {
            let mut decoder = flate2::read::GzDecoder::new(&archive_bytes[..]);
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut decoder, &mut buf).unwrap();
            buf
        };
        let mut archive = tar::Archive::new(&tar_bytes[..]);
        let mut paths = Vec::new();
        let mut index_html = String::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_string_lossy().to_string();
            if path == "index.html" {
                std::io::Read::read_to_string(&mut entry, &mut index_html).unwrap();
            }
            paths.push(path);
        }

        assert!(
            paths.iter().any(|p| p == "spacetime.js"),
            "built archive must contain the compiled runtime, got: {paths:?}"
        );
        assert!(
            paths.iter().any(|p| p == "spacetime.css"),
            "built archive must contain the compiled styles, got: {paths:?}"
        );
        assert!(
            !paths.iter().any(|p| p.ends_with(".st")),
            "no .st source may cross the wire, got: {paths:?}"
        );
        assert!(
            index_html.contains("<tail>"),
            "widget tail must be spliced into the exported index.html: {index_html}"
        );
    }

    #[test]
    fn package_site_archive_skips_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("real.txt"), "real").unwrap();
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(
                dir.path().join("real.txt"),
                dir.path().join("link.txt"),
            );
        }

        let archive_bytes =
            package_site_archive(&dir.path().to_path_buf(), "", "", "<tail>").unwrap();
        let tar_bytes = {
            let mut decoder = flate2::read::GzDecoder::new(&archive_bytes[..]);
            let mut out = Vec::new();
            std::io::Read::read_to_end(&mut decoder, &mut out).unwrap();
            out
        };
        let mut archive = tar::Archive::new(&tar_bytes[..]);
        let mut paths = Vec::new();
        for entry in archive.entries().unwrap() {
            let entry = entry.unwrap();
            paths.push(entry.path().unwrap().to_string_lossy().to_string());
        }
        assert!(paths.contains(&"real.txt".to_string()));
        assert!(
            !paths.contains(&"link.txt".to_string()),
            "symlinks must be skipped, found: {paths:?}"
        );
    }
}
