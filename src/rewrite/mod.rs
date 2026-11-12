//! Rewrite — the one way tools change source (PLAN-156 P1).
//!
//! A [`Transaction`] is a list of [`Edit`]s, each « these bytes of this file,
//! which read `expect`, now read `text` ». Every tool that writes `.st`
//! source goes through here: a studio gesture, an admin save, an MCP agent's
//! edit, a migration wave. So every writer gets the same guarantees:
//!
//! - **guarded**: an edit whose `expect` no longer matches the file is
//!   refused ([`Error::Stale`]); so is a file whose text changed since the
//!   hash it was read at;
//! - **whole**: overlapping edits are refused; the edits of one file apply
//!   back to front, so every span is in the coordinates the tool read;
//! - **parses**: a file the transaction leaves unparseable is never written
//!   ([`Error::Unparseable`]): a gesture can't break a page;
//! - **atomic**: each file is written to a temp file and renamed;
//! - **undoable**: the journal keeps each commit's before and after texts;
//!   [`undo`] restores the last one if nobody has changed the files since.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// One edit: the bytes `start..end` of `file` read `expect`; make them `text`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edit {
    pub file: String,
    pub start: usize,
    pub end: usize,
    #[serde(default)]
    pub expect: String,
    pub text: String,
}

/// Edits applied together, or not at all. `base` optionally pins whole files
/// to the hash ([`crate::reflect::text_hash`]) they were read at.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    #[serde(default)]
    pub edits: Vec<Edit>,
    #[serde(default)]
    pub base: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The file isn't one the transaction may touch.
    UnknownFile(String),
    /// A span is outside the file, or two edits overlap.
    BadSpan { file: String, start: usize, end: usize },
    /// The text at a span (or the whole file) isn't what the tool read.
    Stale { file: String, start: usize, found: String, expected: String },
    /// The result doesn't parse.
    Unparseable { file: String, message: String },
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownFile(p) => write!(f, "not a file of this page: {p}"),
            Error::BadSpan { file, start, end } => write!(f, "{file}: span {start}..{end} is outside the file or overlaps another edit"),
            Error::Stale { file, start, found, expected } => write!(
                f,
                "STALE: {file} changed since it was read (at {start}: expected `{expected}`, found `{found}`); reload and try again"
            ),
            Error::Unparseable { file, message } => write!(f, "{file} would no longer parse, so nothing was written: {message}"),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

/// Apply `tx` to `files` (path → text) in memory. Returns the files it
/// changed, with their new text. Nothing is written.
pub fn apply(files: &BTreeMap<String, String>, tx: &Transaction) -> Result<BTreeMap<String, String>, Error> {
    for (path, hash) in &tx.base {
        let text = files.get(path).ok_or_else(|| Error::UnknownFile(path.clone()))?;
        let now = crate::reflect::text_hash(text);
        if &now != hash {
            return Err(Error::Stale { file: path.clone(), start: 0, found: now, expected: hash.clone() });
        }
    }
    let mut by_file: BTreeMap<&str, Vec<&Edit>> = BTreeMap::new();
    for e in &tx.edits {
        by_file.entry(e.file.as_str()).or_default().push(e);
    }
    let mut out = BTreeMap::new();
    for (path, mut edits) in by_file {
        let text = files.get(path).ok_or_else(|| Error::UnknownFile(path.to_string()))?;
        edits.sort_by_key(|e| (e.start, e.end));
        for pair in edits.windows(2) {
            if pair[1].start < pair[0].end {
                return Err(Error::BadSpan { file: path.to_string(), start: pair[1].start, end: pair[1].end });
            }
        }
        let mut new = text.clone();
        for e in edits.iter().rev() {
            let found = text.get(e.start..e.end).ok_or_else(|| Error::BadSpan {
                file: path.to_string(),
                start: e.start,
                end: e.end,
            })?;
            if found != e.expect {
                return Err(Error::Stale {
                    file: path.to_string(),
                    start: e.start,
                    found: found.to_string(),
                    expected: e.expect.clone(),
                });
            }
            new.replace_range(e.start..e.end, &e.text);
        }
        if &new != text {
            out.insert(path.to_string(), new);
        }
    }
    check_parses(files, &out)?;
    Ok(out)
}

/// Every changed file must still parse. (A file that didn't parse before the
/// edit is the author's to fix; refusing to touch it would trap them, so only
/// a file that parsed and stops parsing is refused.)
pub fn check_parses(before: &BTreeMap<String, String>, changed: &BTreeMap<String, String>) -> Result<(), Error> {
    for (path, text) in changed {
        if !path.ends_with(".st") {
            continue;
        }
        let parsed_before = before.get(path).is_none_or(|old| crate::parser::parse(old).is_ok());
        if !parsed_before {
            continue;
        }
        if let Err(e) = crate::parser::parse(text) {
            return Err(Error::Unparseable { file: path.clone(), message: format!("{e:?}") });
        }
    }
    Ok(())
}

/// Apply `tx` to the page `entry` of the site at `site_dir` and write it: what
/// `POST /__spacetime/rewrite` and MCP's `st_rewrite` do. Returns the files
/// changed.
pub fn page(site_dir: &Path, entry: &str, tx: &Transaction) -> Result<Vec<String>, Error> {
    let site = crate::studio::outline::Site::load(site_dir, entry).map_err(Error::Io)?;
    let before: BTreeMap<String, String> = site.files.iter().map(|f| (f.path.clone(), f.text.clone())).collect();
    let changed = apply(&before, tx)?;
    commit(site_dir, &before, &changed)?;
    Ok(changed.into_keys().collect())
}

/// One commit, as the journal keeps it.
#[derive(Debug, Clone)]
struct Commit {
    root: PathBuf,
    files: Vec<(String, String, String)>, // path, before, after
}

static JOURNAL: Mutex<Vec<Commit>> = Mutex::new(Vec::new());
const JOURNAL_DEPTH: usize = 100;

/// Write `changed` (path relative to `root` → new text) to disk, atomically
/// per file, after checking each still parses; record the commit for undo.
/// `before` holds the texts the changes were computed from.
pub fn commit(root: &Path, before: &BTreeMap<String, String>, changed: &BTreeMap<String, String>) -> Result<(), Error> {
    check_parses(before, changed)?;
    let mut record = Commit { root: root.to_path_buf(), files: Vec::new() };
    for (path, text) in changed {
        let old = before.get(path).cloned().unwrap_or_default();
        write_atomic(&root.join(path), text)?;
        record.files.push((path.clone(), old, text.clone()));
    }
    if !record.files.is_empty() {
        let mut j = JOURNAL.lock().unwrap_or_else(|e| e.into_inner());
        j.push(record);
        let excess = j.len().saturating_sub(JOURNAL_DEPTH);
        j.drain(..excess);
    }
    Ok(())
}

/// Write a file through a temp file and a rename, so a reader never sees half of it.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), Error> {
    let tmp = path.with_extension(format!(
        "{}.rewrite-tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    std::fs::write(&tmp, text).map_err(|e| Error::Io(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path).map_err(|e| Error::Io(format!("rename {}: {e}", path.display())))
}

/// Undo the last commit under `root`: each file goes back to its text before
/// the commit, if it still reads as the commit left it. Returns the files
/// restored; `Ok(vec![])` when there is nothing to undo.
pub fn undo(root: &Path) -> Result<Vec<String>, Error> {
    let mut j = JOURNAL.lock().unwrap_or_else(|e| e.into_inner());
    let Some(at) = j.iter().rposition(|c| c.root == root) else { return Ok(Vec::new()) };
    let c = j[at].clone();
    for (path, _, after) in &c.files {
        let now = std::fs::read_to_string(root.join(path)).map_err(|e| Error::Io(e.to_string()))?;
        if &now != after {
            return Err(Error::Stale { file: path.clone(), start: 0, found: "a later edit".into(), expected: "the undone commit's text".into() });
        }
    }
    for (path, before, _) in &c.files {
        write_atomic(&root.join(path), before)?;
    }
    j.remove(at);
    Ok(c.files.iter().map(|f| f.0.clone()).collect())
}

#[cfg(test)]
mod tests;
