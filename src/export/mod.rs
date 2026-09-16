//! Static site export.
//!
//! Layered pipeline:
//!   discover → bundle → html (passes) → emit
//!
//! Each layer lives in its own submodule and is independently tested.
//! This file owns only the public surface; orchestration lives in `emit::run`.

mod artifact;
mod bundle;
pub mod compress;
mod discover;
mod emit;
mod html;
mod path;

use std::path::PathBuf;

use serde::Serialize;

/// Configuration for a site export.
pub struct ExportConfig {
    pub site_dir: PathBuf,
    pub output_dir: PathBuf,
    pub minify: bool,
}

/// Summary of an export run.
#[derive(Debug, Serialize)]
pub struct ExportReport {
    pub pages_exported: usize,
    pub files_copied: usize,
    pub total_size: u64,
    pub warnings: Vec<String>,
}

/// Typed export errors. Converted to `String` at the public boundary so the
/// CLI surface stays unchanged.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum ExportError {
    NoIndexSt(PathBuf),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    HtmlRewrite {
        path: PathBuf,
        msg: String,
    },
    /// One or more entries failed to COMPILE (ERROR-severity pipeline errors).
    /// FEAT-170: reporting these as warnings and exporting anyway let a broken
    /// page ship green — the banned silent-acceptance class.
    Compile { message: String },
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::NoIndexSt(p) => write!(f, "No index.st found in {}", p.display()),
            ExportError::Io { path, source } => {
                write!(f, "IO error on {}: {}", path.display(), source)
            }
            ExportError::HtmlRewrite { path, msg } => {
                write!(f, "HTML rewrite failed for {}: {}", path.display(), msg)
            }
            ExportError::Compile { message } => write!(f, "{message}"),
        }
    }
}

impl From<ExportError> for String {
    fn from(e: ExportError) -> Self {
        e.to_string()
    }
}

/// Export a Spacetime site to a static directory.
///
/// Delegates to the layered pipeline in `emit::run` and converts the typed
/// error to a `String` for the existing CLI contract.
pub use artifact::{
    ArtifactError, ArtifactFile, ChunkEntry, FileEntry, Fingerprint, Manifest, SiteArtifact,
    SiteMeta, build_artifact, canonical_json, changed_files, compute_artifact_hash, fingerprint,
};

pub fn export_site(config: &ExportConfig) -> Result<ExportReport, String> {
    emit::run(config).map_err(String::from)
}
