//! SiteArtifact — the ONE build-artifact generation path (W1-1 /
//! PLAN-005, contract: docs/deploy/ARTIFACT.md).
//!
//! CLI `build`, `serve --built`, and the push path all build through
//! `build_artifact`; no second builder survives. The manifest shape,
//! canonical JSON form, quota constants, and the chunked-file hash
//! construction are pinned BYTE-FOR-BYTE against the host's
//! `spacetime-host-core::manifest` / `::deploy` — the cross-repo vector
//! tests below exist so a drift on either side fails loudly.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// Quotas — MUST match spacetime-host-core::manifest exactly: a manifest
// verse ACCEPTS must never be rejected server-side.
pub const MAX_FILE_COUNT: usize = 10_000;
pub const MAX_DEPLOY_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_BLOB_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_CHUNK_BYTES: u64 = 8 * 1024 * 1024;

/// Domain-separation context for the chunked-file identity hash. Pinned
/// in docs/deploy/ARTIFACT.md; the host verifies assembled files with
/// the same construction. Never edit in place — bump the `/v2/`
/// segment instead.
const CHUNK_CONTEXT: &str = "spacetime-host/v2/file-chunks";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteArtifact {
    pub manifest: Manifest,
    /// The artifact's file bytes, keyed by manifest order. Skipped by
    /// serde: the manifest is the wire form; bytes upload separately as
    /// blobs/chunks.
    #[serde(skip)]
    pub files: Vec<ArtifactFile>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub artifact_hash: String,
    pub compiler: String,
    pub files: Vec<FileEntry>,
    pub site: SiteMeta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub b3: String,
    pub chunks: Option<Vec<ChunkEntry>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkEntry {
    pub b3: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteMeta {
    pub deploy: serde_json::Value,
    pub locales: Vec<String>,
}

/// Content fingerprint of the SOURCE tree (W3-1 skips no-op rebuilds
/// with it; W3-2 shows the changed-files list). Content, never mtimes.
#[derive(Debug, Clone, PartialEq)]
pub struct Fingerprint {
    pub hash: String,
    /// path → content blake3 (hex), sorted by path.
    pub files: BTreeMap<String, String>,
}

#[derive(Debug)]
pub enum ArtifactError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Export(String),
    NoIndexSt(PathBuf),
    InvalidPath(String),
    TooManyFiles,
    DeployTooLarge,
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactError::Io { path, source } => {
                write!(f, "I/O error at {}: {source}", path.display())
            }
            ArtifactError::Export(msg) => write!(f, "site export failed: {msg}"),
            ArtifactError::NoIndexSt(p) => write!(f, "no index.st found in {}", p.display()),
            ArtifactError::InvalidPath(p) => write!(
                f,
                "artifact path {p:?} is not deploy-safe (must be relative, ASCII, \
                 no backslashes/control chars/empty or '..' segments)"
            ),
            ArtifactError::TooManyFiles => {
                write!(f, "deploy exceeds the {MAX_FILE_COUNT}-file limit")
            }
            ArtifactError::DeployTooLarge => write!(
                f,
                "deploy exceeds the {} MiB total-size limit",
                MAX_DEPLOY_BYTES / (1024 * 1024)
            ),
        }
    }
}
impl std::error::Error for ArtifactError {}

/// Canonical manifest JSON: keys sorted recursively, no whitespace,
/// `artifact_hash` omitted from the hashed form. Byte-identical to the
/// host's `canonical_json` — the vector tests pin this.
///
/// `artifact_hash` is removed BEFORE sorting: serde_json's
/// `preserve_order` feature (transitively enabled in some builds) makes
/// `Map::remove` a reordering swap_remove, which would silently destroy
/// canonical order if applied after sorting.
pub fn canonical_json(m: &Manifest) -> String {
    let mut v = serde_json::to_value(m).expect("Manifest serializes to JSON");
    if let serde_json::Value::Object(map) = &mut v {
        map.remove("artifact_hash");
    }
    serde_json::to_string(&sort_value(v)).expect("sorted Value serializes to JSON")
}

fn sort_value(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(m) => serde_json::Value::Object(
            m.into_iter()
                .map(|(k, v)| (k, sort_value(v)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        serde_json::Value::Array(a) => {
            serde_json::Value::Array(a.into_iter().map(sort_value).collect())
        }
        x => x,
    }
}

pub fn compute_artifact_hash(m: &Manifest) -> blake3::Hash {
    blake3::hash(canonical_json(m).as_bytes())
}

/// Builds the deploy artifact for `site_dir`: export the site to a
/// tempdir (the same `export_site` the CLI's `build` uses), then hash
/// every emitted file into a manifest. Source files (`.st`, `.org`)
/// can never enter the artifact — only export output is walked.
pub fn build_artifact(site_dir: &Path) -> Result<SiteArtifact, ArtifactError> {
    if !site_dir.join("index.st").exists() {
        return Err(ArtifactError::NoIndexSt(site_dir.to_path_buf()));
    }
    let tmp = tempfile::tempdir().map_err(|e| ArtifactError::Io {
        path: site_dir.to_path_buf(),
        source: e,
    })?;
    super::export_site(&super::ExportConfig {
        site_dir: site_dir.to_path_buf(),
        output_dir: tmp.path().to_path_buf(),
        minify: true,
    })
    .map_err(ArtifactError::Export)?;

    let files = collect_files(tmp.path())?;
    let mut entries = Vec::with_capacity(files.len());
    let mut total = 0u64;
    for file in &files {
        let size = file.bytes.len() as u64;
        total = total
            .checked_add(size)
            .ok_or(ArtifactError::DeployTooLarge)?;
        if total > MAX_DEPLOY_BYTES {
            return Err(ArtifactError::DeployTooLarge);
        }
        let chunks = (size > MAX_BLOB_BYTES).then(|| make_chunks(&file.bytes));
        let b3 = chunks
            .as_ref()
            .map(|c| chunked_hash(c))
            .unwrap_or_else(|| blake3::hash(&file.bytes).to_hex().to_string());
        entries.push(FileEntry {
            path: file.path.clone(),
            size,
            b3,
            chunks,
        });
    }

    let mut manifest = Manifest {
        artifact_hash: String::new(),
        compiler: format!("spacetime {}", env!("CARGO_PKG_VERSION")),
        files: entries,
        // TODO(W3-1): populate the @deploy fact and locales from the
        // export report once the push client consumes them — the host
        // treats the whole site block as opaque canonical content.
        site: SiteMeta {
            deploy: serde_json::Value::Null,
            locales: Vec::new(),
        },
    };
    manifest.artifact_hash = compute_artifact_hash(&manifest).to_hex().to_string();
    Ok(SiteArtifact { manifest, files })
}

fn collect_files(root: &Path) -> Result<Vec<ArtifactFile>, ArtifactError> {
    let mut out = Vec::new();
    collect_rec(root, root, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    if out.len() > MAX_FILE_COUNT {
        return Err(ArtifactError::TooManyFiles);
    }
    Ok(out)
}

fn collect_rec(root: &Path, dir: &Path, out: &mut Vec<ArtifactFile>) -> Result<(), ArtifactError> {
    for entry in std::fs::read_dir(dir)
        .map_err(|e| ArtifactError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?
        .flatten()
    {
        let p = entry.path();
        if p.is_dir() {
            collect_rec(root, &p, out)?;
        } else if p.is_file() {
            let path = relative_slash_path(root, &p)?;
            validate_path(&path)?;
            out.push(ArtifactFile {
                path,
                bytes: std::fs::read(&p).map_err(|e| ArtifactError::Io {
                    path: p.clone(),
                    source: e,
                })?,
            });
        }
    }
    Ok(())
}

/// Builds the manifest path from path COMPONENTS (never a lossy string
/// replace): a literal backslash inside a filename survives to
/// `validate_path`, which rejects it — the pre-hardening version
/// silently mangled `a\b.png` into the fake nested path `a/b.png`.
fn relative_slash_path(root: &Path, p: &Path) -> Result<String, ArtifactError> {
    let rel = p
        .strip_prefix(root)
        .map_err(|_| ArtifactError::InvalidPath(p.display().to_string()))?;
    let mut parts = Vec::new();
    for component in rel.components() {
        match component {
            std::path::Component::Normal(os) => parts.push(os.to_string_lossy().into_owned()),
            _ => return Err(ArtifactError::InvalidPath(rel.display().to_string())),
        }
    }
    Ok(parts.join("/"))
}

/// Mirrors the host's `validate_manifest` path rules exactly.
fn validate_path(path: &str) -> Result<(), ArtifactError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || !path.is_ascii()
        || path.bytes().any(|b| b < 0x20 || b == 0x7f)
        || path.split('/').any(|s| s.is_empty() || s == "..")
    {
        return Err(ArtifactError::InvalidPath(path.into()));
    }
    Ok(())
}

fn make_chunks(bytes: &[u8]) -> Vec<ChunkEntry> {
    fastcdc::v2020::FastCDC::new(bytes, 512 * 1024, 1024 * 1024, MAX_CHUNK_BYTES as u32)
        .map(|c| {
            let b = &bytes[c.offset..c.offset + c.length];
            ChunkEntry {
                b3: blake3::hash(b).to_hex().to_string(),
                size: b.len() as u64,
            }
        })
        .collect()
}

/// Chunked-file identity: keyed blake3 over the concatenated RAW
/// 32-byte chunk hashes, domain-separated by CHUNK_CONTEXT. Pinned in
/// docs/deploy/ARTIFACT.md — the host re-verifies with the same
/// construction at commit time.
fn chunked_hash(chunks: &[ChunkEntry]) -> String {
    let mut raw = Vec::with_capacity(chunks.len() * 32);
    for c in chunks {
        raw.extend_from_slice(
            blake3::Hash::from_hex(&c.b3)
                .expect("chunk b3 is produced as hex above")
                .as_bytes(),
        );
    }
    blake3::keyed_hash(&blake3::derive_key(CHUNK_CONTEXT, &[]), &raw)
        .to_hex()
        .to_string()
}

/// Content fingerprint of the SOURCE tree: sorted {path, content-hash}
/// pairs hashed together. Excludes build output and VCS/vendor noise
/// (dist, .git, node_modules, target) — everything else counts,
/// including .st sources, so an edit always moves the fingerprint.
pub fn fingerprint(site_dir: &Path) -> Result<Fingerprint, ArtifactError> {
    let mut files = BTreeMap::new();
    fingerprint_rec(site_dir, site_dir, &mut files)?;
    let mut h = blake3::Hasher::new();
    for (path, content_hash) in &files {
        h.update(path.as_bytes());
        h.update(
            blake3::Hash::from_hex(content_hash)
                .expect("fingerprint hashes are produced as hex")
                .as_bytes(),
        );
    }
    Ok(Fingerprint {
        hash: h.finalize().to_hex().to_string(),
        files,
    })
}

/// The changed-files list between two fingerprints: added, removed, or
/// content-changed paths, sorted. W3-2 renders this in the widget.
pub fn changed_files(prev: &Fingerprint, next: &Fingerprint) -> Vec<String> {
    let mut changed: Vec<String> = prev
        .files
        .keys()
        .chain(next.files.keys())
        .filter(|p| prev.files.get(*p) != next.files.get(*p))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    changed.sort();
    changed
}

fn fingerprint_rec(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, String>,
) -> Result<(), ArtifactError> {
    for entry in std::fs::read_dir(dir)
        .map_err(|e| ArtifactError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?
        .flatten()
    {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if ["dist", ".git", "node_modules", "target"].contains(&name.as_str()) {
                continue;
            }
            fingerprint_rec(root, &p, out)?;
        } else if p.is_file() {
            let rel = relative_slash_path(root, &p)?;
            let bytes = std::fs::read(&p).map_err(|e| ArtifactError::Io {
                path: p.clone(),
                source: e,
            })?;
            out.insert(rel, blake3::hash(&bytes).to_hex().to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The minimal exportable site (empty index.st compiles; proven by
    /// host_package's W0-1 regression test).
    fn write_minimal_site(dir: &Path) {
        std::fs::write(
            dir.join("index.html"),
            "<html><head></head><body><h1>Hi</h1></body></html>",
        )
        .unwrap();
        std::fs::write(dir.join("index.st"), "").unwrap();
    }

    fn dummy_manifest() -> Manifest {
        Manifest {
            artifact_hash: "0".repeat(64),
            compiler: "spacetime 0.9.4".to_string(),
            files: vec![FileEntry {
                path: "index.html".to_string(),
                size: 15296,
                b3: "a".repeat(64),
                chunks: None,
            }],
            site: SiteMeta {
                deploy: serde_json::json!({"project": "demo"}),
                locales: vec!["en".to_string()],
            },
        }
    }

    // ── Cross-repo canonical-form parity (host: spacetime-host-core ──
    // ── src/manifest.rs — SAME input, SAME expected string) ─────────
    #[test]
    fn canonical_json_matches_host_vector() {
        let mut m = dummy_manifest();
        m.site.deploy = serde_json::json!({"z": 1, "a": 2});
        m.files.push(FileEntry {
            path: "media/tour.mp4".to_string(),
            size: MAX_BLOB_BYTES + 1,
            b3: "1".repeat(64),
            chunks: Some(vec![ChunkEntry {
                b3: "2".repeat(64),
                size: MAX_BLOB_BYTES + 1,
            }]),
        });

        let json = canonical_json(&m);
        assert!(!json.contains("artifact_hash"));
        assert!(!json.contains('\n'));
        let expected = format!(
            concat!(
                r#"{{"compiler":"spacetime 0.9.4","files":"#,
                r#"[{{"b3":"{0}","chunks":null,"path":"index.html","size":15296}},"#,
                r#"{{"b3":"{1}","chunks":[{{"b3":"{2}","size":4194305}}],"path":"media/tour.mp4","size":4194305}}],"#,
                r#""site":{{"deploy":{{"a":2,"z":1}},"locales":["en"]}}}}"#
            ),
            "a".repeat(64),
            "1".repeat(64),
            "2".repeat(64)
        );
        assert_eq!(json, expected);
    }

    #[test]
    fn canonical_json_is_independent_of_parsed_key_order() {
        let a: Manifest = serde_json::from_str(
            r#"{"artifact_hash":"00","compiler":"c","files":[],"site":{"deploy":{},"locales":[]}}"#,
        )
        .unwrap();
        let b: Manifest = serde_json::from_str(
            r#"{"site":{"locales":[],"deploy":{}},"files":[],"compiler":"c","artifact_hash":"00"}"#,
        )
        .unwrap();
        assert_eq!(canonical_json(&a), canonical_json(&b));
        assert!(!canonical_json(&a).contains("artifact_hash"));
    }

    // ── Cross-repo chunk-hash parity (host: src/deploy.rs tests) ────
    #[test]
    fn chunked_hash_matches_host_construction() {
        let chunk1 = b"chunk one ".repeat(100);
        let chunk2 = b"chunk two ".repeat(100);
        let chunks = vec![
            ChunkEntry {
                b3: blake3::hash(&chunk1).to_hex().to_string(),
                size: chunk1.len() as u64,
            },
            ChunkEntry {
                b3: blake3::hash(&chunk2).to_hex().to_string(),
                size: chunk2.len() as u64,
            },
        ];
        // Independently recompute per docs/deploy/ARTIFACT.md's pinned
        // construction — this is exactly what the host does when
        // verifying assembled files at commit time.
        let mut raw = Vec::new();
        for c in &chunks {
            raw.extend_from_slice(blake3::Hash::from_hex(&c.b3).unwrap().as_bytes());
        }
        let expected = blake3::keyed_hash(&blake3::derive_key(CHUNK_CONTEXT, &[]), &raw)
            .to_hex()
            .to_string();
        assert_eq!(chunked_hash(&chunks), expected);
        // And it must NOT degenerate to hashing the bytes or hex strings.
        assert_ne!(
            chunked_hash(&chunks),
            blake3::hash(&[chunk1.clone(), chunk2.clone()].concat())
                .to_hex()
                .to_string()
        );
    }

    #[test]
    fn chunked_file_roundtrip() {
        // > 4 MiB forces the chunked path; reassembly is plain
        // concatenation in manifest order (the host's assemble_file).
        // Data must be non-periodic — CDC finds no cut points in a
        // repeating pattern. Deterministic xorshift stream, no rand dep.
        let mut state = 0x9E3779B97F4A7C15u64;
        let bytes: Vec<u8> = (0..(MAX_BLOB_BYTES + 1024 * 1024) as usize)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 32) as u8
            })
            .collect();
        let chunks = make_chunks(&bytes);
        assert!(chunks.len() > 1, "multi-MiB input must chunk");
        let total: u64 = chunks.iter().map(|c| c.size).sum();
        assert_eq!(total, bytes.len() as u64);
        // Sizes within the CDC bounds.
        for c in &chunks {
            assert!(c.size <= MAX_CHUNK_BYTES);
        }
    }

    #[test]
    fn artifact_is_deterministic() {
        let site = tempfile::tempdir().unwrap();
        write_minimal_site(site.path());
        std::fs::write(site.path().join("data.json"), "{\"a\": 1}").unwrap();
        let a = build_artifact(site.path()).expect("first build");
        let b = build_artifact(site.path()).expect("second build");
        assert_eq!(a.manifest, b.manifest, "same source → same manifest");
        assert_eq!(a.files, b.files, "same source → same bytes");
    }

    #[test]
    fn artifact_contains_no_source() {
        let site = tempfile::tempdir().unwrap();
        write_minimal_site(site.path());
        std::fs::write(site.path().join("secret-notes.st"), "$secret: 42").unwrap();
        let artifact = build_artifact(site.path()).expect("build");
        for file in &artifact.files {
            assert!(
                !file.path.ends_with(".st") && !file.path.ends_with(".org"),
                "source file leaked into artifact: {}",
                file.path
            );
        }
        assert!(
            artifact.files.iter().any(|f| f.path == "index.html"),
            "built html present"
        );
    }

    #[test]
    fn artifact_rejects_unsafe_built_paths() {
        // collect_files operates on export output, but the validation
        // must hold independently of what produced the directory.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/ok.png"), b"x").unwrap();
        let files = collect_files(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "sub/ok.png");

        for bad in [
            "a\\b.png",
            "/abs.png",
            "a//b.png",
            "a/../b.png",
            "ünïcode.png",
            "ctl\x07.png",
        ] {
            assert!(
                validate_path(bad).is_err(),
                "path must be rejected: {bad:?}"
            );
        }
        for good in ["a/b/c.png", "index.html", "fonts/AspektaVF.woff2"] {
            assert!(validate_path(good).is_ok(), "path must pass: {good:?}");
        }
    }

    #[test]
    fn backslash_filename_is_rejected_not_mangled() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a\\b.png"), b"x").unwrap();
        let err = collect_files(dir.path()).unwrap_err();
        assert!(matches!(err, ArtifactError::InvalidPath(_)), "{err}");
    }

    #[test]
    fn fingerprint_skips_noop_rebuild_and_detects_changes() {
        let site = tempfile::tempdir().unwrap();
        write_minimal_site(site.path());
        let f1 = fingerprint(site.path()).unwrap();
        let f2 = fingerprint(site.path()).unwrap();
        assert_eq!(f1.hash, f2.hash, "no edits → same fingerprint");
        assert!(changed_files(&f1, &f2).is_empty());

        std::fs::write(
            site.path().join("index.html"),
            "<html><body>changed</body></html>",
        )
        .unwrap();
        std::fs::write(site.path().join("new.css"), "body{}").unwrap();
        let f3 = fingerprint(site.path()).unwrap();
        assert_ne!(f1.hash, f3.hash, "an edit must move the fingerprint");
        assert_eq!(
            changed_files(&f1, &f3),
            vec!["index.html".to_string(), "new.css".to_string()]
        );

        // Build output never moves the source fingerprint.
        std::fs::create_dir(site.path().join("dist")).unwrap();
        std::fs::write(site.path().join("dist/index.html"), "built").unwrap();
        let f4 = fingerprint(site.path()).unwrap();
        assert_eq!(f3.hash, f4.hash, "dist/ is build output, not source");
    }
}
