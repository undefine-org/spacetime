// Std-only vendor bundling core (PLAN-024 W1).
//
// This file is the SINGLE source of truth for the bundle mechanism. It has NO
// crate-internal dependencies (std + `Command` only) so it can be `include!`d
// from `build.rs` (the `cargo build` embed trigger) AND wrapped by the lib's
// `vendor.rs` (the `vendor build` CLI / hot-reload trigger). One routine, two
// triggers — see `docs/stdlib/VENDORING.md`.
//
// Higher-level concerns (parsing `%vendor`, blake3 content hashing) live in the
// lib wrapper; this core only resolves paths, applies the lazy escape, and
// shells out to `git` / `bun`.

#[allow(dead_code)]
pub mod vendor_core {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// How to flatten a vendored dependency into a single file.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Strategy {
        /// Already one self-contained file — copy verbatim.
        Direct,
        /// Multi-file ESM/TS graph — `bun build --format=iife`.
        Bun,
    }

    /// A fully-resolved bundle request (paths still relative to `module_dir`).
    #[derive(Debug, Clone)]
    pub struct BundleSpec {
        /// Global name the IIFE exposes (the `%vendor` name).
        pub name: String,
        /// Directory the `source`/`out` paths are relative to (e.g. `stdlib/text`).
        pub module_dir: PathBuf,
        /// Submodule path, relative to `module_dir`.
        pub source: String,
        /// Bundler entry, relative to the submodule.
        pub entry: String,
        /// Output artifact, relative to `module_dir`.
        pub out: String,
        pub strategy: Strategy,
    }

    /// Outcome of a bundle attempt.
    #[derive(Debug, Clone)]
    pub struct BundleOutcome {
        pub out_path: PathBuf,
        pub bytes: usize,
        pub submodule_commit: Option<String>,
        pub skipped: bool,
    }

    /// Why a bundle failed.
    #[derive(Debug)]
    pub enum BundleError {
        BundlerMissing { name: String, out: PathBuf },
        SubmoduleMissing(PathBuf),
        EntryMissing(PathBuf),
        BundlerFailed { name: String, stderr: String },
        Io(String),
    }

    impl std::fmt::Display for BundleError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                BundleError::BundlerMissing { name, out } => write!(
                    f,
                    "bundler `{}` not found and no existing artifact at {} — install {} or commit the bundle",
                    name,
                    out.display(),
                    name
                ),
                BundleError::SubmoduleMissing(p) => write!(
                    f,
                    "vendor submodule not found at {} (run `git submodule update --init`)",
                    p.display()
                ),
                BundleError::EntryMissing(p) => {
                    write!(f, "vendor entry file not found: {}", p.display())
                }
                BundleError::BundlerFailed { name, stderr } => {
                    write!(f, "bundler `{}` failed:\n{}", name, stderr)
                }
                BundleError::Io(e) => write!(f, "io error: {}", e),
            }
        }
    }

    fn submodule_commit(submodule: &Path) -> Option<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(submodule)
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() { None } else { Some(s) }
    }

    fn bun_available() -> bool {
        Command::new("bun")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn stamp_path(out: &Path) -> PathBuf {
        let mut p = out.as_os_str().to_os_string();
        p.push(".commit");
        PathBuf::from(p)
    }

    fn recorded_commit(out: &Path) -> Option<String> {
        std::fs::read_to_string(stamp_path(out))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn read_outcome(
        out: PathBuf,
        commit: Option<String>,
        skipped: bool,
    ) -> Result<BundleOutcome, BundleError> {
        let data = std::fs::read(&out).map_err(|e| BundleError::Io(e.to_string()))?;
        Ok(BundleOutcome {
            bytes: data.len(),
            out_path: out,
            submodule_commit: commit,
            skipped,
        })
    }

    /// Build (or lazily skip) one bundle. `force` bypasses the lazy escape.
    ///
    /// Lazy escape matrix (force = false):
    /// | artifact | commit cmp | bun | action |
    /// |----------|-----------|-----|--------|
    /// | present  | match     | —   | skip   |
    /// | present  | no git    | —   | skip   |
    /// | present  | differs   | yes | build  |
    /// | present  | differs   | no  | skip+warn (stale but buildable) |
    /// | absent   | —         | yes | build  |
    /// | absent   | —         | no  | error  |
    pub fn bundle(spec: &BundleSpec, force: bool) -> Result<BundleOutcome, BundleError> {
        let submodule = spec.module_dir.join(&spec.source);
        // `entry` is relative to the MODULE dir, not the submodule: it is usually
        // a small author-controlled wrapper (e.g. `vendor/<dep>-entry.ts`) that
        // re-exports the submodule's surface onto a global. `source` stays the
        // submodule purely for commit-pinning (the lazy escape).
        let entry = spec.module_dir.join(&spec.entry);
        let out = spec.module_dir.join(&spec.out);
        let current = submodule_commit(&submodule);

        if !force && out.exists() {
            let fresh = match (&current, recorded_commit(&out)) {
                (Some(cur), Some(rec)) => cur == &rec,
                (None, _) => true,
                (Some(_), None) => !bun_available(),
            };
            if fresh {
                return read_outcome(out, current, true);
            }
        }

        if !submodule.exists() {
            if out.exists() {
                return read_outcome(out, None, true);
            }
            return Err(BundleError::SubmoduleMissing(submodule));
        }

        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| BundleError::Io(e.to_string()))?;
        }

        match spec.strategy {
            Strategy::Direct => {
                if !entry.exists() {
                    return Err(BundleError::EntryMissing(entry));
                }
                std::fs::copy(&entry, &out).map_err(|e| BundleError::Io(e.to_string()))?;
            }
            Strategy::Bun => {
                if !bun_available() {
                    if out.exists() {
                        return read_outcome(out, current, true);
                    }
                    return Err(BundleError::BundlerMissing {
                        name: "bun".to_string(),
                        out,
                    });
                }
                if !entry.exists() {
                    return Err(BundleError::EntryMissing(entry));
                }
                let output = Command::new("bun")
                    .arg("build")
                    .arg(&entry)
                    .arg("--format=iife")
                    .arg(format!("--global-name={}", spec.name))
                    .arg("--minify")
                    .arg("--outfile")
                    .arg(&out)
                    .output()
                    .map_err(|e| BundleError::Io(e.to_string()))?;
                if !output.status.success() {
                    return Err(BundleError::BundlerFailed {
                        name: "bun".to_string(),
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    });
                }
            }
        }

        if let Some(commit) = &current {
            let _ = std::fs::write(stamp_path(&out), commit);
        }
        read_outcome(out, current, false)
    }
}
