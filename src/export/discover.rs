//! Site discovery: walk site_dir → SiteLayout.
//!
//! Stubbed in Wave 0; populated TDD-style in Wave 1 by the DISCOVER task.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// A single emitted page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct Route {
    /// Relative to site_dir.
    pub(crate) html_rel: PathBuf,
    /// Nearest-ancestor directory containing index.st, relative to site_dir.
    /// Empty PathBuf for site root.
    pub(crate) bundle_dir: PathBuf,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct SiteLayout {
    /// Discovered HTML routes, sorted deterministically.
    pub(crate) routes: Vec<Route>,
    /// Every compilable page entry, keyed by its ROUTE DIRECTORY relative to
    /// site_dir (empty PathBuf for the site root), valued by the entry `.st`
    /// file relative to site_dir.
    ///
    /// Two shapes produce an entry, mirroring the dev server's clean-path
    /// resolution (`server.rs::try_serve_st_page`, FEAT-093) so that what
    /// `spacetime serve` serves, the export emits:
    ///   * `<dir>/index.st`   -> route dir `<dir>`     (`/about/` -> about/index.st)
    ///   * `<dir>/<p>.st`     -> route dir `<dir>/<p>` (`/brand`  -> brand.st)
    /// A sibling `<p>.st` WINS over `<p>/index.st`, exactly as the dev
    /// server's candidate order does -- one resolution rule, two callers.
    ///
    /// Not every entry becomes a page: a `.st` that compiles to no file-scope
    /// markup (a module of styles/behaviour, e.g. `modules/theme.st`) is a
    /// fragment, and `emit` writes neither a page nor a bundle for it. That is
    /// the SAME criterion the dev server effectively applies -- it answers
    /// `/modules/theme` with an empty-bodied shell, i.e. nothing.
    pub(crate) st_entries: BTreeMap<PathBuf, PathBuf>,
    /// Static files to ship, relative to site_dir, sorted
    /// deterministically. Exactly the files the dev server
    /// (`server.rs`'s catch-all) would serve: any file with a served
    /// static extension, anywhere in the tree, minus denylisted dirs
    /// and `.html` routes (emitted compiled, not copied). Deploy parity
    /// rule: what `spacetime serve` serves, the edge serves. (W0-1 /
    /// PLAN-005 -- replaced the old top-level [assets, data, locales]
    /// allowlist, which missed fonts/, root-level css/js, and any
    /// site-specific directory and therefore 404'd real sites' assets.)
    pub(crate) assets: Vec<PathBuf>,
}

/// Walk `site_dir` and return the layout. Honors the denylist (`path::is_denylisted`).
/// Errors with `NoIndexSt` if root `index.st` is absent.
#[allow(dead_code)]
pub(crate) fn discover(
    site_dir: &Path,
    output_dir_canonical: Option<&Path>,
) -> Result<SiteLayout, super::ExportError> {
    // The site root's own entry, seeded with the same priority the dev server
    // applies: `index.st` first, then the literate `index.st.md` (SIP-002).
    // A site with neither has no root page at all -- that is the one hard
    // error discovery raises.
    let root_entry = DIR_INDEX_CANDIDATES
        .iter()
        .map(|(file, _)| PathBuf::from(file))
        .find(|file| site_dir.join(file).exists())
        .ok_or_else(|| super::ExportError::NoIndexSt(site_dir.to_path_buf()))?;

    let mut st_entries = BTreeMap::new();
    st_entries.insert(PathBuf::new(), root_entry);
    collect_st_entries(site_dir, site_dir, output_dir_canonical, &mut st_entries)?;

    // A file that another entry IMPORTS is a module, not a page. Nothing else
    // is a sound test: page furniture like `modules/reader-chrome.st` carries
    // file-scope markup (a sidebar, a header), so "has markup" would promote
    // it to a route and publish the site's chrome at /modules/reader-chrome/.
    // The import graph is the author's own statement of intent -- if a file is
    // pulled into another page, it is part of that page.
    let imported = collect_imported(site_dir, &st_entries);
    st_entries.retain(|route, entry| route.as_os_str().is_empty() || !imported.contains(entry));

    let st_dirs: BTreeSet<PathBuf> = st_entries.keys().cloned().collect();

    let mut routes = Vec::new();
    collect_routes(
        site_dir,
        site_dir,
        output_dir_canonical,
        &st_dirs,
        &mut routes,
    )?;

    routes.sort_by(|a, b| a.html_rel.cmp(&b.html_rel));

    let mut assets = Vec::new();
    collect_static_files(site_dir, site_dir, output_dir_canonical, &mut assets)?;
    assets.sort();

    Ok(SiteLayout {
        routes,
        st_entries,
        assets,
    })
}

/// Every site-local file reachable by `@import` from any candidate entry,
/// relative to `site_dir`.
///
/// Reads only the import lines (a cheap textual scan, no parse): an import is
/// `@import "<path>"` at file scope, and only RELATIVE paths name site files --
/// a bare `stdlib/...` specifier resolves inside the toolchain, never here.
/// Walks transitively, so a module imported only by another module is still
/// recognised as a module.
fn collect_imported(site_dir: &Path, st_entries: &BTreeMap<PathBuf, PathBuf>) -> BTreeSet<PathBuf> {
    let mut imported = BTreeSet::new();
    let mut queue: Vec<PathBuf> = st_entries.values().cloned().collect();
    let mut seen: BTreeSet<PathBuf> = queue.iter().cloned().collect();

    while let Some(rel) = queue.pop() {
        let Ok(text) = std::fs::read_to_string(site_dir.join(&rel)) else {
            continue;
        };
        let parent = rel.parent().unwrap_or(Path::new("")).to_path_buf();
        for spec in import_specifiers(&text) {
            if !(spec.starts_with("./") || spec.starts_with("../")) {
                continue; // stdlib or package specifier -- not a site file
            }
            let Some(target) = normalize_relative(&parent, &spec) else {
                continue;
            };
            if !site_dir.join(&target).exists() {
                continue;
            }
            imported.insert(target.clone());
            if seen.insert(target.clone()) {
                queue.push(target);
            }
        }
    }
    imported
}

/// Extract the quoted specifier of every `@import "..."` line in `text`.
fn import_specifiers(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix("@import")?;
            let rest = rest.trim_start();
            let quoted = rest.strip_prefix('"')?;
            let end = quoted.find('"')?;
            Some(quoted[..end].to_string())
        })
        .collect()
}

/// Resolve `spec` (a `./` or `../` path) against `base`, collapsing `.` and
/// `..` lexically. Returns `None` when the result would escape the site root.
fn normalize_relative(base: &Path, spec: &str) -> Option<PathBuf> {
    let mut parts: Vec<std::ffi::OsString> = base
        .components()
        .map(|c| c.as_os_str().to_owned())
        .collect();
    for seg in spec.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(std::ffi::OsString::from(other)),
        }
    }
    let mut out = PathBuf::new();
    for p in parts {
        out.push(p);
    }
    Some(out)
}

/// Candidate priority for the entry that answers a route, mirroring
/// `server.rs::try_serve_st_page`'s candidate ARRAY ORDER exactly:
///
///   0  `<p>.st`            1  `<p>.st.md`
///   2  `<p>/index.st`      3  `<p>/index.st.md`
///
/// Lower wins. Encoding the order as a rank (rather than relying on
/// insertion order) is what lets discovery walk the tree in whatever order
/// the filesystem yields and still land on the same entry the dev server
/// would serve. Two callers, ONE order -- they cannot drift apart silently,
/// which is the exact failure BUG-218 and BUG-220 both were.
const DIR_INDEX_CANDIDATES: &[(&str, u8)] = &[("index.st", 2), ("index.st.md", 3)];

/// Classify a file name as a page entry: `(stem, rank)`, or `None` when the
/// file is not a Spacetime entry at all.
///
/// SIP-002 literate entries (`.st.md`) are first-class pages, so the stem of
/// `brand.st.md` is `brand` -- NOT `brand.st`, which a naive `file_stem` gives
/// and which would route the page to `/brand.st/`.
fn classify_entry(file_name: &str) -> Option<(&str, u8)> {
    if let Some(stem) = file_name.strip_suffix(".st.md") {
        return Some((stem, 1));
    }
    if let Some(stem) = file_name.strip_suffix(".st") {
        return Some((stem, 0));
    }
    None
}

/// Record `entry` as the page for `route_dir` if no higher-priority candidate
/// already claimed it. Ties cannot occur: every candidate has a distinct rank.
fn claim(
    st_entries: &mut BTreeMap<PathBuf, PathBuf>,
    route_dir: PathBuf,
    entry: PathBuf,
    rank: u8,
) {
    match st_entries.get(&route_dir) {
        Some(existing) if rank_of(existing) <= rank => {}
        _ => {
            st_entries.insert(route_dir, entry);
        }
    }
}

/// Recover a recorded entry's rank from its path, so `claim` can compare a
/// new candidate against one recorded by an earlier pass.
fn rank_of(entry: &Path) -> u8 {
    let name = entry
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let is_index = name.starts_with("index.");
    let literate = name.ends_with(".st.md");
    match (is_index, literate) {
        (false, false) => 0,
        (false, true) => 1,
        (true, false) => 2,
        (true, true) => 3,
    }
}

/// Walk `dir` recursively, recording every page entry into `st_entries`
/// (route dir -> entry file, both relative to `site_dir`).
///
/// Mirrors `server.rs::try_serve_st_page`'s candidate order via
/// `DIR_INDEX_CANDIDATES` / `classify_entry` ranks, so dev and export agree on
/// what a clean path means: `<p>.st` beats `<p>.st.md` beats `<p>/index.st`
/// beats `<p>/index.st.md`. Rank comparison (not walk order) decides, so the
/// result does not depend on the order the filesystem yields entries in.
fn collect_st_entries(
    dir: &Path,
    site_dir: &Path,
    output_dir_canonical: Option<&Path>,
    st_entries: &mut BTreeMap<PathBuf, PathBuf>,
) -> Result<(), super::ExportError> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| super::ExportError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?
        .flatten()
        .collect();
    entries.sort_by_key(|a| a.file_name());

    // Pass 1: sibling entries at this level -> route dir `<dir>/<stem>`.
    // `index.*` is excluded: it names its OWN directory's route, which is
    // seeded by the caller (root) or by pass 2 (subdirectories).
    for entry in &entries {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let Some((stem, rank)) = classify_entry(&name) else {
            continue;
        };
        if stem == "index" {
            continue;
        }
        // A leading `_` marks a private partial by the same convention the
        // directory denylist uses (`path::is_denylisted`), so `_nav.st` is a
        // fragment by NAME, never a route -- and never a wasted compile.
        if stem.starts_with('_') {
            continue;
        }
        let relative = path
            .strip_prefix(site_dir)
            .map_err(|e| super::ExportError::Io {
                path: path.clone(),
                source: std::io::Error::other(e),
            })?;
        let route_dir = relative.with_file_name(stem);
        claim(st_entries, route_dir, relative.to_path_buf(), rank);
    }

    // Pass 2: descend, recording `<d>/index.*` as route dir `<d>`.
    for entry in &entries {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if super::path::is_denylisted(&name_str) {
            continue;
        }
        if let Some(output) = output_dir_canonical
            && let Ok(canonical) = path.canonicalize()
            && canonical == output
        {
            continue;
        }
        let relative = path
            .strip_prefix(site_dir)
            .map_err(|e| super::ExportError::Io {
                path: path.clone(),
                source: std::io::Error::other(e),
            })?;
        for (file, rank) in DIR_INDEX_CANDIDATES {
            if path.join(file).exists() {
                claim(
                    st_entries,
                    relative.to_path_buf(),
                    relative.join(file),
                    *rank,
                );
            }
        }
        collect_st_entries(&path, site_dir, output_dir_canonical, st_entries)?;
    }
    Ok(())
}

fn collect_routes(
    dir: &Path,
    site_dir: &Path,
    output_dir_canonical: Option<&Path>,
    st_dirs: &BTreeSet<PathBuf>,
    routes: &mut Vec<Route>,
) -> Result<(), super::ExportError> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| super::ExportError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?
        .flatten()
        .collect();
    entries.sort_by_key(|a| a.file_name());

    for entry in entries {
        let path = entry.path();
        let relative = path
            .strip_prefix(site_dir)
            .map_err(|e| super::ExportError::Io {
                path: path.clone(),
                source: std::io::Error::other(e),
            })?;
        if path.is_dir() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if super::path::is_denylisted(&name_str) {
                continue;
            }
            if let Some(output) = output_dir_canonical
                && let Ok(canonical) = path.canonicalize()
                && canonical == output
            {
                continue;
            }
            collect_routes(&path, site_dir, output_dir_canonical, st_dirs, routes)?;
        } else if path.extension().map(|e| e == "html").unwrap_or(false) {
            let parent = relative.parent().unwrap_or(Path::new(""));
            let bundle_dir = super::path::find_nearest_st_dir(parent, st_dirs);
            routes.push(Route {
                html_rel: relative.to_path_buf(),
                bundle_dir,
            });
        }
    }
    Ok(())
}

/// Extensions the dev server (`server.rs`'s catch-all chain) serves as
/// static files. The artifact's static set is exactly these -- no more
/// (nothing undeployable ships), no less (nothing servable 404s).
/// `.html` is deliberately absent: routes are emitted compiled.
/// W1-1 (SiteArtifact) will own this list in one shared place.
const SERVED_STATIC_EXTENSIONS: &[&str] = &[
    "css", "js", "jpg", "jpeg", "png", "webp", "svg", "mp4", "webm", "json", "glb", "gltf", "bin",
    "ktx2", "hdr", "exr", "draco", "otf", "ttf", "woff", "woff2",
];

/// Recursively collects every servable static file under `dir`,
/// skipping denylisted directories and the output directory.
fn collect_static_files(
    dir: &Path,
    site_dir: &Path,
    output_dir_canonical: Option<&Path>,
    files: &mut Vec<PathBuf>,
) -> Result<(), super::ExportError> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| super::ExportError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?
        .flatten()
        .collect();
    entries.sort_by_key(|a| a.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Route discovery denylists `data`/`assets`/`locales` (asset
            // trees must not compile .html routes) -- but static
            // collection MUST walk them: they hold exactly the media/
            // json the deployed site serves.
            if super::path::is_denylisted(&name_str)
                && !matches!(name_str.as_ref(), "data" | "assets" | "locales")
            {
                continue;
            }
            if let Some(output) = output_dir_canonical
                && let Ok(canonical) = path.canonicalize()
                && canonical == output
            {
                continue;
            }
            collect_static_files(&path, site_dir, output_dir_canonical, files)?;
        } else {
            let served = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| SERVED_STATIC_EXTENSIONS.contains(&e))
                .unwrap_or(false);
            if served {
                let relative = path
                    .strip_prefix(site_dir)
                    .map_err(|e| super::ExportError::Io {
                        path: path.clone(),
                        source: std::io::Error::other(e),
                    })?;
                files.push(relative.to_path_buf());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn requires_root_index_st() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        let err = discover(&site, None).unwrap_err();
        match err {
            super::super::ExportError::NoIndexSt(p) => assert!(p.ends_with("site")),
            other => panic!("expected NoIndexSt, got {:?}", other),
        }
    }

    #[test]
    fn discovers_root_only() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.html"), "<html></html>").unwrap();
        std::fs::write(site.join("index.st"), "").unwrap();
        let layout = discover(&site, None).unwrap();
        assert_eq!(layout.routes.len(), 1);
        assert_eq!(layout.routes[0].html_rel, PathBuf::from("index.html"));
        assert_eq!(layout.routes[0].bundle_dir, PathBuf::from(""));
        assert_eq!(
            layout.st_entries.get(&PathBuf::from("")),
            Some(&PathBuf::from("index.st"))
        );
        assert!(layout.assets.is_empty());
    }

    #[test]
    fn discovers_sibling_routes() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.html"), "<html></html>").unwrap();
        std::fs::write(site.join("index.st"), "").unwrap();
        std::fs::create_dir(site.join("about")).unwrap();
        std::fs::write(site.join("about/index.html"), "<html></html>").unwrap();
        std::fs::write(site.join("about/index.st"), "").unwrap();
        let layout = discover(&site, None).unwrap();
        assert_eq!(layout.routes.len(), 2);
        let about = layout
            .routes
            .iter()
            .find(|r| r.html_rel == PathBuf::from("about/index.html"))
            .unwrap();
        assert_eq!(about.bundle_dir, PathBuf::from("about"));
    }

    #[test]
    fn discovers_nested_routes_use_ancestor_bundle() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.html"), "<html></html>").unwrap();
        std::fs::write(site.join("index.st"), "").unwrap();
        std::fs::create_dir_all(site.join("about/team")).unwrap();
        std::fs::write(site.join("about/index.st"), "").unwrap();
        std::fs::write(site.join("about/team/index.html"), "<html></html>").unwrap();
        let layout = discover(&site, None).unwrap();
        let team = layout
            .routes
            .iter()
            .find(|r| r.html_rel == PathBuf::from("about/team/index.html"))
            .unwrap();
        assert_eq!(team.bundle_dir, PathBuf::from("about"));
    }

    #[test]
    fn ignores_denylisted_dirs() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.html"), "<html></html>").unwrap();
        std::fs::write(site.join("index.st"), "").unwrap();
        for d in &["_partials", "dist", "node_modules", ".git"] {
            std::fs::create_dir_all(site.join(d)).unwrap();
            std::fs::write(site.join(d).join("index.html"), "<html></html>").unwrap();
            std::fs::write(site.join(d).join("index.st"), "").unwrap();
        }
        let layout = discover(&site, None).unwrap();
        assert_eq!(layout.routes.len(), 1);
        assert_eq!(layout.routes[0].html_rel, PathBuf::from("index.html"));
    }

    #[test]
    fn assets_collects_servable_files_at_any_depth() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.html"), "<html></html>").unwrap();
        std::fs::write(site.join("index.st"), "").unwrap();
        // Servable at root and at depth (W0-1: fonts/ and root-level css
        // were the production casualties of the old allowlist).
        std::fs::write(site.join("theme.css"), "body{}").unwrap();
        std::fs::create_dir_all(site.join("fonts")).unwrap();
        std::fs::write(site.join("fonts/brand.woff2"), "woff").unwrap();
        std::fs::create_dir_all(site.join("data")).unwrap();
        std::fs::write(site.join("data/x.json"), "{}").unwrap();
        // NOT servable: sources, routes, prose, scripts.
        std::fs::write(site.join("notes.md"), "secret").unwrap();
        std::fs::write(site.join("plan.org"), "secret").unwrap();
        std::fs::write(site.join("helper.py"), "secret").unwrap();
        // Denylisted dirs never ship, even their servable files.
        std::fs::create_dir_all(site.join("dist")).unwrap();
        std::fs::write(site.join("dist/stale.css"), "old{}").unwrap();
        let layout = discover(&site, None).unwrap();
        assert_eq!(
            layout.assets,
            vec![
                PathBuf::from("data/x.json"),
                PathBuf::from("fonts/brand.woff2"),
                PathBuf::from("theme.css"),
            ]
        );
    }

    #[test]
    fn sort_order_deterministic() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir(&site).unwrap();
        std::fs::write(site.join("index.html"), "<html></html>").unwrap();
        std::fs::write(site.join("index.st"), "").unwrap();
        std::fs::create_dir(site.join("contact")).unwrap();
        std::fs::write(site.join("contact/index.html"), "<html></html>").unwrap();
        std::fs::write(site.join("contact/index.st"), "").unwrap();
        std::fs::create_dir(site.join("about")).unwrap();
        std::fs::write(site.join("about/index.html"), "<html></html>").unwrap();
        std::fs::write(site.join("about/index.st"), "").unwrap();
        let layout1 = discover(&site, None).unwrap();
        let layout2 = discover(&site, None).unwrap();
        assert_eq!(layout1.routes, layout2.routes);
        for i in 1..layout1.routes.len() {
            assert!(layout1.routes[i - 1].html_rel <= layout1.routes[i].html_rel);
        }
    }
}
