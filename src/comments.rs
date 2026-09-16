//! Structured comments (PLAN-123) — THE scanner, the model, and the store.
//!
//! A comment is a conversation pinned to a place in a project. Humans write
//! them in the pill; agents write them over MCP; both read the same records.
//! This module is the ONLY thing that parses `//@` headers, the ONLY thing
//! that reads or writes `.comments/`, and therefore the only place the three
//! readers (pill routes, `spacetime check`, MCP tools) can disagree — which is
//! precisely why there is exactly one of it.
//!
//! # Storage is hybrid, split by ROLE
//!
//! - The comment's CONTENT lives inline in `.st` source, as a real comment.
//!   It travels with the code, shows up in review, and merges like code.
//! - The comment's WORKFLOW STATE (status, thread, who claimed it) lives in
//!   `<project>/.comments/<id>.json`, one file per comment. State changes far
//!   more often than content and must be writable for files that cannot host a
//!   comment at all (JSON, assets) or for anchors that are not a file position
//!   (a route). One file per comment keeps concurrent edits merge-friendly.
//!
//! Both are committed to git: comment state is team data, not scratch.
//!
//! # The `//@` surface
//!
//! ```text
//! //@: a bare header is a note
//! //@todo: replace the placeholder photography
//! //@bug(severity: high): nav overlaps the CTA below 380px
//! //@> a continuation line carries the explicit marker
//! //@>
//! //@> a bare marker is a paragraph break
//! ```
//!
//! ZERO ABSORPTION: an unmarked `//` line is never swallowed into a comment,
//! no matter how adjacent. Ambiguity here would be silent and unfixable — a
//! stray thought becoming part of a work item nobody wrote. The cost of the
//! rule is one marker per line; the benefit is that a `//@>` with no header
//! above it is DETECTABLE, so agent edits that delete a header get reported
//! instead of quietly orphaning prose.
//!
//! # The invariant
//!
//! Comments are trivia. Nothing in this module can change emitted output, so
//! every failure here degrades to a diagnostic: an unknown type, a malformed
//! header, a missing field — all warnings, never a broken build. That is what
//! makes the surface cheap to evolve (the scanner parses leniently; no
//! `%migration` wave is needed for trivia) and what must stay true.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::parser::comments::extract_comments;
use crate::parser::meta_ast::{CommentFieldKind, CommentTypeDefAst};

/// Sidecar record schema version. Records are read LENIENTLY (an older `v` is
/// upgraded in memory) and written at the current version, so a schema change
/// never strands a team's comment history behind a toolchain upgrade.
pub const SCHEMA_VERSION: u32 = 1;

/// The directory, relative to the project root, holding sidecar records.
pub const COMMENTS_DIR: &str = ".comments";

/// The type id a bare `//@:` header resolves to.
pub const DEFAULT_TYPE: &str = "note";

// ============================================================================
// Model
// ============================================================================

/// Where a comment is pinned.
///
/// A tagged union rather than a nullable file+line: the anchor kinds have
/// genuinely different resolution rules, and flattening them would force every
/// reader to re-derive which fields are meaningful. `Unsupported` preserves a
/// future anchor unchanged: a reader that cannot interpret a new kind must not
/// discard the entire team conversation on its next sidecar scan.
#[derive(Debug, Clone, PartialEq)]
pub enum Anchor {
    /// A position in a source file that CAN host comments — the inline case.
    /// `line` is 1-based and advisory: it is refreshed from the live scan, so
    /// edits above the comment move it without orphaning anything.
    Inline { file: String, line: usize },
    /// A file that cannot host a `//@` comment (JSON, an asset), optionally
    /// narrowed to a path within it (e.g. a JSON pointer).
    File {
        file: String,
        json_path: Option<String>,
    },
    /// A route, for remarks about a page as a whole rather than a line of it.
    Page { route: String },
    /// A specific element on a served route.
    ///
    /// `selector` is a POSITIONAL HINT, not the identity: Spacetime's structural
    /// ids shift when markup is inserted above the target. `fingerprint` is the
    /// identity — see [`element_fingerprint`] for why anything else lets a
    /// comment silently re-point at the wrong element.
    ///
    /// `None` means a legacy record written before fingerprints existed. Those
    /// are readable and re-anchorable but cannot be trusted to still point where
    /// they did; readers surface them as unverified rather than pretending.
    Element {
        route: String,
        selector: String,
        fingerprint: Option<String>,
        /// What the element LOOKED LIKE when the comment was made.
        ///
        /// The fingerprint proves a match but cannot be reversed, so a reader
        /// with only the hash can verify nothing and locate nothing. Keeping the
        /// descriptor lets the pill find the element on a live page and lets a
        /// human see what the comment was about even after it is gone — an
        /// orphan that cannot say what it lost is a dead end.
        #[allow(clippy::struct_field_names)]
        content: Option<Box<ElementCandidate>>,
    },
    /// A future tagged kind retained as opaque sidecar data rather than skipped.
    Unsupported {
        kind: String,
        fields: serde_json::Map<String, serde_json::Value>,
    },
}

impl Serialize for Anchor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut object = serde_json::Map::new();
        match self {
            Anchor::Inline { file, line } => {
                object.insert("kind".into(), serde_json::json!("inline"));
                object.insert("file".into(), serde_json::json!(file));
                object.insert("line".into(), serde_json::json!(line));
            }
            Anchor::File { file, json_path } => {
                object.insert("kind".into(), serde_json::json!("file"));
                object.insert("file".into(), serde_json::json!(file));
                if let Some(json_path) = json_path {
                    object.insert("json_path".into(), serde_json::json!(json_path));
                }
            }
            Anchor::Page { route } => {
                object.insert("kind".into(), serde_json::json!("page"));
                object.insert("route".into(), serde_json::json!(route));
            }
            Anchor::Element {
                route,
                selector,
                fingerprint,
                content,
            } => {
                object.insert("kind".into(), serde_json::json!("element"));
                object.insert("route".into(), serde_json::json!(route));
                object.insert("selector".into(), serde_json::json!(selector));
                // Omitted rather than written null, so a legacy record stays
                // byte-identical on a read/write round-trip and does not churn
                // every sidecar in a project's git history.
                if let Some(fingerprint) = fingerprint {
                    object.insert("fingerprint".into(), serde_json::json!(fingerprint));
                }
                if let Some(content) = content {
                    object.insert("content".into(), serde_json::json!(content));
                }
            }
            Anchor::Unsupported { kind, fields } => {
                object = fields.clone();
                object.insert("kind".into(), serde_json::json!(kind));
            }
        }
        serde_json::Value::Object(object).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Anchor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut object = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
        let kind = object
            .remove("kind")
            .and_then(|value| value.as_str().map(str::to_owned))
            .ok_or_else(|| serde::de::Error::custom("anchor kind must be a string"))?;
        let string = |name: &str, fields: &mut serde_json::Map<String, serde_json::Value>| {
            fields
                .remove(name)
                .and_then(|value| value.as_str().map(str::to_owned))
                .ok_or_else(|| serde::de::Error::custom(format!("anchor {name} must be a string")))
        };
        match kind.as_str() {
            "inline" => {
                let file = string("file", &mut object)?;
                let line = object
                    .remove("line")
                    .and_then(|value| value.as_u64())
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or_else(|| {
                        serde::de::Error::custom("anchor line must be a positive integer")
                    })?;
                Ok(Anchor::Inline { file, line })
            }
            "file" => {
                let file = string("file", &mut object)?;
                let json_path = match object.remove("json_path") {
                    None | Some(serde_json::Value::Null) => None,
                    Some(value) => Some(value.as_str().map(str::to_owned).ok_or_else(|| {
                        serde::de::Error::custom("anchor json_path must be a string")
                    })?),
                };
                Ok(Anchor::File { file, json_path })
            }
            "page" => Ok(Anchor::Page {
                route: string("route", &mut object)?,
            }),
            "element" => {
                let route = string("route", &mut object)?;
                let selector = string("selector", &mut object)?;
                // Absent on records written before fingerprints existed. A
                // MISSING fingerprint is legacy; a present-but-wrong-typed one is
                // corruption and must not be silently downgraded to legacy.
                let fingerprint = match object.remove("fingerprint") {
                    None | Some(serde_json::Value::Null) => None,
                    Some(value) => Some(value.as_str().map(str::to_owned).ok_or_else(|| {
                        serde::de::Error::custom("anchor fingerprint must be a string")
                    })?),
                };
                let content = match object.remove("content") {
                    None | Some(serde_json::Value::Null) => None,
                    Some(value) => Some(Box::new(
                        serde_json::from_value::<ElementCandidate>(value).map_err(|error| {
                            serde::de::Error::custom(format!(
                                "anchor content is malformed: {error}"
                            ))
                        })?,
                    )),
                };
                Ok(Anchor::Element {
                    route,
                    selector,
                    fingerprint,
                    content,
                })
            }
            _ => Ok(Anchor::Unsupported {
                kind,
                fields: object,
            }),
        }
    }
}

impl Anchor {
    /// The file this anchor names, when it names one.
    pub fn file(&self) -> Option<&str> {
        match self {
            Anchor::Inline { file, .. } | Anchor::File { file, .. } => Some(file),
            Anchor::Page { .. } | Anchor::Element { .. } | Anchor::Unsupported { .. } => None,
        }
    }

    /// Human-readable location for `check` rows and pill headings.
    pub fn display(&self) -> String {
        match self {
            Anchor::Inline { file, line } => format!("{file}:{line}"),
            Anchor::File {
                file,
                json_path: Some(p),
            } => format!("{file}#{p}"),
            Anchor::File { file, .. } => file.clone(),
            Anchor::Page { route } => route.clone(),
            // A legacy element anchor (no fingerprint) cannot be verified to
            // still point where it did, so it says so rather than rendering
            // identically to a verified one. Silence here is what let the
            // original defect hide.
            Anchor::Element {
                route,
                selector,
                fingerprint: Some(_),
                ..
            } => format!("{route} — {selector}"),
            Anchor::Element {
                route, selector, ..
            } => format!("{route} — {selector} (unverified)"),
            Anchor::Unsupported { kind, .. } => format!("unsupported anchor ({kind})"),
        }
    }
}

/// Workflow state. A CLOSED set: the pill, `check`, and MCP all render status,
/// and a free-form string would let one writer invent a state the others cannot
/// display. Extension happens in the type roster (which is data), not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    #[default]
    Open,
    InProgress,
    Resolved,
    Wontfix,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::InProgress => "in-progress",
            Status::Resolved => "resolved",
            Status::Wontfix => "wontfix",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Status::Open),
            "in-progress" => Some(Status::InProgress),
            "resolved" => Some(Status::Resolved),
            "wontfix" => Some(Status::Wontfix),
            _ => None,
        }
    }

    /// Whether this status still wants someone's attention — the filter behind
    /// the pill's badge count and `check`'s roster.
    pub fn is_open(&self) -> bool {
        matches!(self, Status::Open | Status::InProgress)
    }
}

/// Who wrote or claimed something. `kind` is what lets the pill show an agent's
/// reply differently from a teammate's, and what the agency tier maps onto a
/// real actor with permissions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Author {
    pub kind: AuthorKind,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthorKind {
    Human,
    Agent,
}

/// One reply in a comment's thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    pub author: Author,
    pub text: String,
    pub at: String,
}

/// One recorded status change: who moved it, from what, to what, and when.
///
/// A bare `status` field answers "where is this now" but not "who decided
/// that" — and on a shared record, the second question is the accountable one.
/// "Resolved" with no author is indistinguishable from a mistake, an
/// over-eager agent, or someone closing work they had not read; a reviewer who
/// disagrees has nobody to ask. Replies already carry an author and a time, so
/// a status change carrying less was the inconsistency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusChange {
    pub author: Author,
    /// The status this record moved AWAY from. Kept because "reopened from
    /// resolved" and "claimed from open" are different events, and a log of
    /// destinations alone cannot distinguish them.
    pub from: Status,
    pub to: Status,
    pub at: String,
}

/// A comment: content + workflow state, merged from inline source and sidecar.
///
/// This struct IS the JSON on disk, the JSON on the route, and the JSON an MCP
/// tool returns. Keeping them one shape is deliberate — the agency tier turns
/// this record into a hosted resource, and a fork between "what the pill sends"
/// and "what the store holds" is exactly the seam that would make that port a
/// rewrite instead of a transport swap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentRecord {
    /// Content-hash for inline comments, ULID for sidecar-originated ones.
    pub id: String,
    #[serde(rename = "type")]
    pub type_id: String,
    #[serde(default)]
    pub status: Status,
    pub author: Author,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<Author>,
    pub anchor: Anchor,
    pub text: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thread: Vec<Reply>,
    /// Status transitions, oldest first. Empty on records written before this
    /// existed — absence means "unknown", never "never changed", and readers
    /// must not present it as the latter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<StatusChange>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Schema version of this record as stored.
    #[serde(default = "default_version")]
    pub v: u32,
    /// True when the content came from a live `//@` comment in source. Not
    /// serialized: it is a property of THIS scan, not of the stored record —
    /// persisting it would let a stale flag outlive the comment it describes.
    #[serde(skip)]
    pub inline: bool,
}

fn default_version() -> u32 {
    1
}

// ============================================================================
// Inline payload parsing
// ============================================================================

/// A `//@` header parsed out of one comment line.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineHeader {
    pub type_id: String,
    pub meta: BTreeMap<String, String>,
    pub text: String,
    /// Meta keys whose value could not be parsed, kept so validation can name
    /// them rather than the shape silently differing from what was written.
    pub malformed_meta: Vec<String>,
}

/// What one comment line IS, lexically.
#[derive(Debug, Clone, PartialEq)]
pub enum InlineLine {
    /// `//@…: …` — starts a comment.
    Header(InlineHeader),
    /// `//@> …` — continues the header above it. Empty text = paragraph break.
    Continuation(String),
}

/// Parse a single comment's text as a `//@` line, if it is one.
///
/// Lenient BY DESIGN: this is the one versioning point for the `//@` surface,
/// so it accepts what it can and records what it cannot (`malformed_meta`)
/// rather than rejecting. A comment that fails to parse stays a comment.
pub fn parse_inline_line(comment_text: &str) -> Option<InlineLine> {
    let t = comment_text.trim_start();
    let rest = t.strip_prefix("//")?;

    // `//@>` before `//@`: the continuation marker is a longer prefix, and
    // testing it second would classify every continuation as a header.
    if let Some(body) = rest.strip_prefix("@>") {
        return Some(InlineLine::Continuation(body.trim().to_string()));
    }

    let after_at = rest.strip_prefix('@')?;
    // `///@…` must NOT parse: `///` is a doc comment, whose contiguous runs
    // are collected by the parser for real symbols. Overloading it would make
    // a doc comment silently become a work item.
    if rest.starts_with("/") {
        return None;
    }

    // Split the header from the text at the FIRST colon outside the meta
    // parens AND outside any quoted value. Everything after it is literal —
    // colons, URLs and `//` included, because prose routinely contains all
    // three.
    //
    // Quote-awareness is not decoration: an acceptance command like
    // `(acceptance: "cargo test -- --nocapture (")` carries an unmatched
    // literal paren, and a depth counter blind to quotes would never find the
    // real colon — the comment would silently vanish from the roster.
    let mut depth = 0usize;
    let mut in_quotes = false;
    let mut escaped = false;
    let mut split_at = None;
    for (i, c) in after_at.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' if in_quotes => escaped = true,
            '"' => in_quotes = !in_quotes,
            '(' if !in_quotes => depth += 1,
            ')' if !in_quotes => depth = depth.saturating_sub(1),
            ':' if depth == 0 && !in_quotes => {
                split_at = Some(i);
                break;
            }
            _ => {}
        }
    }
    let split_at = split_at?;
    let (head, text) = after_at.split_at(split_at);
    let text = text[1..].trim().to_string();

    let (type_part, meta_part) = match head.find('(') {
        Some(i) => {
            let close = head.rfind(')')?;
            if close < i {
                return None;
            }
            (&head[..i], Some(&head[i + 1..close]))
        }
        None => (head, None),
    };

    let type_id = match type_part.trim() {
        "" => DEFAULT_TYPE.to_string(),
        other => other.to_string(),
    };

    let mut meta = BTreeMap::new();
    let mut malformed_meta = Vec::new();
    if let Some(raw) = meta_part {
        for pair in split_meta_pairs(raw) {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            match pair.split_once(':') {
                Some((k, v)) => {
                    let key = k.trim().to_string();
                    let raw = v.trim();
                    // An unterminated quoted value means the author's intent
                    // is genuinely unclear (where does the value end?), and
                    // guessing would silently store something they did not
                    // write — AND change the identity hash. Report it.
                    if raw.starts_with('"') && !is_closed_quoted(raw) {
                        malformed_meta.push(pair.to_string());
                        continue;
                    }
                    meta.insert(key, unquote_meta(raw));
                }
                // `(severity)` with no value: recorded, not guessed at.
                None => malformed_meta.push(pair.to_string()),
            }
        }
    }

    Some(InlineLine::Header(InlineHeader {
        type_id,
        meta,
        text,
        malformed_meta,
    }))
}

/// Split `k: v, k2: "v, with comma"` on commas OUTSIDE quotes.
fn split_meta_pairs(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut escaped = false;
    for c in raw.chars() {
        match c {
            '\\' if in_quotes && !escaped => {
                escaped = true;
                cur.push(c);
            }
            '"' if !escaped => {
                in_quotes = !in_quotes;
                cur.push(c);
            }
            ',' if !in_quotes => {
                out.push(std::mem::take(&mut cur));
                escaped = false;
            }
            _ => {
                escaped = false;
                cur.push(c);
            }
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Whether a value that OPENS with `"` also closes, honouring escapes.
///
/// `"oops\"` ends in a quote byte but that quote is escaped, so the value
/// never actually closes — the naive "ends with a quote" test would accept it
/// and store `oops\`.
fn is_closed_quoted(v: &str) -> bool {
    let mut chars = v.chars();
    if chars.next() != Some('"') {
        return false;
    }
    let mut escaped = false;
    for c in chars {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '"' => return true,
            _ => {}
        }
    }
    false
}

/// Strip surrounding quotes and unescape `\"` — the only escape the surface
/// needs, since a value lives on one line by construction.
fn unquote_meta(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        v[1..v.len() - 1].replace("\\\"", "\"")
    } else {
        v.to_string()
    }
}

// ============================================================================
// Identity
// ============================================================================

/// Stable id for an inline comment: hash of file + type + meta + the FIRST
/// line of text.
///
/// The body is deliberately EXCLUDED. Editing a comment's prose — fixing a
/// typo, adding a continuation line — must not orphan its status and thread,
/// which is what a whole-text hash would do on every keystroke. Changing the
/// header (the type, the metadata, or the opening line) DOES mint a new
/// identity, because that is a change to what is being asked, not to how it is
/// worded.
///
/// BLAKE3, not DefaultHasher. This value is PERSISTED — it is the sidecar
/// filename and the key a team's comment history is stored under — so the
/// algorithm must be stable across Rust releases and machines. `DefaultHasher`
/// is explicitly documented as unspecified and free to change between
/// releases; adopting it here would mean a toolchain upgrade silently
/// re-identifying every comment in every project and orphaning all of their
/// status and threads at once.
///
/// Fields are LENGTH-PREFIXED so no two different inputs can serialize to the
/// same byte stream (without it, meta `{ab: c}` and `{a: bc}` would collide).
/// Whitespace is normalized first, so re-indentation is not a rename.
pub fn inline_id(
    file: &str,
    type_id: &str,
    meta: &BTreeMap<String, String>,
    first_line: &str,
) -> String {
    let mut h = blake3::Hasher::new();
    let mut feed = |s: &str| {
        h.update(&(s.len() as u64).to_le_bytes());
        h.update(s.as_bytes());
    };
    feed(&normalize(file));
    feed(type_id);
    // BTreeMap iterates in key order, so meta contributes deterministically
    // regardless of the order the author wrote the pairs in.
    for (k, v) in meta {
        feed(k);
        feed(&normalize(v));
    }
    feed(&normalize(first_line));
    // 16 hex chars is ample for per-project uniqueness and keeps the sidecar
    // filenames readable in a directory listing and a git diff.
    format!("i{}", &h.finalize().to_hex()[..16])
}

/// A content fingerprint for the element an `element` anchor points at.
///
/// ## Why a selector alone is not an identity
///
/// Spacetime's structural ids (`data-st-node`, allocated by
/// [`crate::introspect::html_ids`]) are POSITIONAL: `0.1.2` means "third
/// structural child of the second child of the root". Insert one element above
/// a commented one and every later id shifts by one.
///
/// An anchor keyed on that path does NOT orphan when the page is edited — it
/// resolves cleanly to a DIFFERENT element. A note reading "this claim is
/// unsupported" silently slides onto the next paragraph, with no warning and no
/// orphan row. That is strictly worse than losing the anchor: a lost comment is
/// visible, a relocated one reads like data while being wrong.
///
/// So the identity is the element's CONTENT — tag, its stable literal class and
/// id, and a digest of its text — and the positional selector is demoted to a
/// HINT used only to find a candidate fast. Shifting an element's position
/// changes the hint and leaves the identity untouched; changing what the element
/// SAYS changes the identity, and the comment orphans loudly through the same
/// rail every other anchor kind already uses.
///
/// This mirrors [`inline_id`] deliberately: same BLAKE3, same length-prefixed
/// feed, same reason (a persisted key must be stable across toolchains, and no
/// two distinct inputs may serialize to one byte stream). One feature must not
/// ship two identity models — that is how the weaker one goes undefended.
///
/// Text is normalized (whitespace collapsed) so re-indenting markup is not a
/// rename, and TRUNCATED to a prefix so editing the tail of a long paragraph
/// does not orphan a comment about that paragraph — the same "header, not body"
/// judgement `inline_id` makes by hashing only the first line.
pub fn element_fingerprint(tag: &str, class: &str, id_attr: &str, text: &str) -> String {
    let mut h = blake3::Hasher::new();
    let mut feed = |s: &str| {
        h.update(&(s.len() as u64).to_le_bytes());
        h.update(s.as_bytes());
    };
    feed(&normalize(tag).to_ascii_lowercase());
    feed(&normalize(class));
    feed(&normalize(id_attr));
    let text = normalize(text);
    let head: String = text.chars().take(ELEMENT_TEXT_PREFIX).collect();
    feed(&head);
    format!("e{}", &h.finalize().to_hex()[..16])
}

/// One element offered to [`resolve_element`] as a possible match.
///
/// Deliberately a plain data tuple, not a DOM handle: the SAME rule has to run
/// server-side (over projected candidates) and in the pill (over live elements),
/// and a rule that can only run in one place is how two implementations of one
/// need get started.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementCandidate {
    /// The positional structural id (`data-st-node`) — the HINT.
    #[serde(default)]
    pub selector: String,
    pub tag: String,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub id_attr: String,
    #[serde(default)]
    pub text: String,
}

impl ElementCandidate {
    /// This candidate's content fingerprint.
    pub fn fingerprint(&self) -> String {
        element_fingerprint(&self.tag, &self.class, &self.id_attr, &self.text)
    }
}

/// What resolving an element anchor against a live page produced.
#[derive(Debug, Clone, PartialEq)]
pub enum ElementResolution {
    /// Exactly one element carries this fingerprint. `selector` is refreshed —
    /// the element may have MOVED, which is precisely the case a positional
    /// anchor got wrong.
    Resolved { selector: String },
    /// Nothing on the page carries this fingerprint: the element was deleted or
    /// its content changed. An ORPHAN — loud, re-anchorable, never silent.
    Gone,
    /// Several elements are indistinguishable by content (e.g. repeated list
    /// rows). Guessing would re-introduce the exact defect fingerprints exist to
    /// kill, so this reports the tie instead of picking.
    Ambiguous { count: usize },
    /// A legacy record with no fingerprint. Its selector is reported so the page
    /// can still show something, but it is NOT a verified match.
    Unverified { selector: String },
}

/// Stamp a picked element's content fingerprint onto its anchor.
///
/// The picker runs in the browser, but the fingerprint is minted HERE. Hashing
/// client-side would mean a second BLAKE3 implementation of one rule, and the
/// moment the two disagreed by a byte every anchor written by one surface would
/// be invisible to the other — a silent, project-wide orphaning with no error
/// anywhere. So the client reports what it SEES (tag, class, id, text) and the
/// server decides what that MEANS.
///
/// A caller-supplied `fingerprint` is ignored in favour of the minted one: it is
/// untrusted input, and an anchor whose fingerprint does not match its own
/// content could never resolve.
pub fn with_element_content(anchor: Anchor, content: &ElementCandidate) -> Anchor {
    match anchor {
        Anchor::Element {
            route,
            selector,
            fingerprint: _,
            content: _,
        } => Anchor::Element {
            route,
            // The client's reported position wins only when it supplied one;
            // otherwise keep whatever the anchor already named.
            selector: if content.selector.is_empty() {
                selector
            } else {
                content.selector.clone()
            },
            fingerprint: Some(content.fingerprint()),
            content: Some(Box::new(content.clone())),
        },
        other => other,
    }
}

/// Resolve an element anchor against the elements currently on its page.
///
/// ## The rule
///
/// Match on FINGERPRINT ONLY. The stored selector is never used to pick a
/// candidate — only to describe one afterwards.
///
/// That is the whole point. Matching on the positional selector is what made a
/// comment slide onto a neighbouring element when markup shifted above it:
/// position resolves cleanly to the WRONG thing, so the failure is invisible.
/// Content either matches or it does not, so every failure is an orphan the
/// reader can see and act on.
///
/// Ambiguity is reported, never broken by position. Two identical rows are
/// genuinely indistinguishable by content; picking the one whose old index
/// matches would be exactly the positional guess this function exists to
/// refuse, and it would be wrong precisely when a row was inserted — the
/// scenario that motivated the fingerprint.
pub fn resolve_element(
    anchor: &Anchor,
    candidates: &[ElementCandidate],
) -> Option<ElementResolution> {
    let Anchor::Element {
        selector,
        fingerprint,
        ..
    } = anchor
    else {
        return None;
    };
    let Some(fingerprint) = fingerprint else {
        return Some(ElementResolution::Unverified {
            selector: selector.clone(),
        });
    };
    let mut hits = candidates
        .iter()
        .filter(|candidate| &candidate.fingerprint() == fingerprint);
    match (hits.next(), hits.count()) {
        (None, _) => Some(ElementResolution::Gone),
        (Some(hit), 0) => Some(ElementResolution::Resolved {
            selector: hit.selector.clone(),
        }),
        (Some(_), rest) => Some(ElementResolution::Ambiguous { count: rest + 1 }),
    }
}

/// One element as the SOURCE projection sees it: a node id, its authored
/// label, and the byte span of the tag that produced it.
///
/// This is deliberately NOT [`ElementCandidate`]. That type describes a LIVE
/// DOM element and carries its rendered text; this one describes authored
/// markup, where the text may be a hole (`` `$title` ``) that has no value
/// until the page runs. Collapsing the two would force one of them to lie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCandidate {
    /// The projection's node id — the same `data-st-node` path the picker
    /// captured at click time.
    pub id: String,
    /// The authored label: `"section.hero"`, `"button#buy"`, or a bare tag.
    pub label: String,
    /// Byte offsets of this element's own tag, when it was authored.
    pub span: Option<(u64, u64)>,
}

/// Where a comment's element was WRITTEN, as opposed to where it renders.
///
/// Mirrors [`ElementResolution`] on purpose: same four outcomes, same refusal
/// to guess. A reader who understands one understands the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceResolution {
    /// Exactly one authored element matches. `moved` is true when the stored
    /// positional hint pointed somewhere else — the hint went stale but the
    /// identity held, which is the case this whole design exists to survive.
    Resolved {
        id: String,
        span: (u64, u64),
        moved: bool,
    },
    /// Matched an authored element, but it carries no span: it was SYNTHESIZED
    /// rather than written (html5ever inserts wrappers). Distinct from `Gone` —
    /// we found it and it genuinely has no source address.
    Synthesized { id: String },
    /// Nothing in this page's source matches. The element was deleted, renamed,
    /// or its identifying class/id is a hole the compiler cannot resolve
    /// statically. An orphan for source purposes — loud, never silent.
    Gone,
    /// Several authored elements are indistinguishable by label (repeated rows).
    /// Picking by position would be exactly the positional guess fingerprints
    /// were introduced to kill, so this reports the tie.
    Ambiguous { count: usize },
    /// The anchor predates content identity, so nothing can be CONFIRMED. The
    /// hint is followed and the result flagged, never presented as verified.
    Unverified {
        id: String,
        span: Option<(u64, u64)>,
    },
}

/// The label the source projection would give this element.
///
/// Mirrors `introspect::html_walk::element_label` — first class token, else
/// `#id`, else the bare tag. Kept in step by
/// `the_source_label_rule_matches_the_projection`.
fn source_label(tag: &str, class: &str, id_attr: &str) -> String {
    if let Some(first) = class.split_whitespace().next()
        && !first.is_empty()
    {
        return format!("{tag}.{first}");
    }
    let id_attr = id_attr.trim();
    if !id_attr.is_empty() {
        return format!("{tag}#{id_attr}");
    }
    tag.to_string()
}

/// Resolve an element anchor to the SOURCE it was written in.
///
/// The stored `selector` (a `data-st-node` path) is a HINT and nothing more:
/// it is positional, so inserting a sibling shifts it and it will then name a
/// DIFFERENT element — whose span would be the wrong span, delivered with
/// total confidence. So the hint is always CONFIRMED against the anchor's
/// stored content before its span is used, and a hit that fails confirmation
/// is discarded rather than trusted.
///
/// Confirmation is by label, which is weaker than the DOM fingerprint: source
/// has no rendered text to hash, because the text may still be a hole. Weaker
/// evidence is handled by REFUSING more often — several same-label elements
/// report `Ambiguous`, never a pick.
pub fn resolve_element_source(
    anchor: &Anchor,
    candidates: &[SourceCandidate],
) -> Option<SourceResolution> {
    let Anchor::Element {
        selector, content, ..
    } = anchor
    else {
        return None;
    };
    let by_hint = candidates.iter().find(|c| &c.id == selector);
    let Some(content) = content else {
        // No stored content: nothing to confirm against. Follow the hint and
        // say plainly that it is unconfirmed.
        return Some(SourceResolution::Unverified {
            id: selector.clone(),
            span: by_hint.and_then(|c| c.span),
        });
    };
    let want = source_label(&content.tag, &content.class, &content.id_attr);
    let settle = |c: &SourceCandidate, moved: bool| match c.span {
        Some(span) => SourceResolution::Resolved {
            id: c.id.clone(),
            span,
            moved,
        },
        None => SourceResolution::Synthesized { id: c.id.clone() },
    };
    // Fast path: the hint still names an element whose label agrees.
    if let Some(hit) = by_hint
        && hit.label == want
    {
        return Some(settle(hit, false));
    }
    // The hint is stale (or was never right). Fall back to identity.
    let mut hits = candidates.iter().filter(|c| c.label == want);
    Some(match (hits.next(), hits.count()) {
        (None, _) => SourceResolution::Gone,
        (Some(hit), 0) => settle(hit, true),
        (Some(_), rest) => SourceResolution::Ambiguous { count: rest + 1 },
    })
}

/// How much of an element's text contributes to its fingerprint.
///
/// Long enough that two sibling paragraphs are distinguishable; short enough
/// that appending a sentence to a commented paragraph does not orphan the
/// comment about it.
pub const ELEMENT_TEXT_PREFIX: usize = 120;

/// Scan ONE source file the way every reader must: with the HTML markup
/// blocks the parser found excluded.
///
/// A PARSE FAILURE MEANS NO SCAN, not a scan without exclusions. Falling back
/// to an empty exclusion set (the obvious `unwrap_or_default`) means a single
/// unrelated syntax error elsewhere in the file turns every `//@` inside
/// `<div>…</div>` into a harvested comment — the scanner would file text that
/// RENDERS TO VISITORS as a private work item, and the pill would offer to
/// resolve the page's own copy. The file simply has no comments this pass;
/// the compiler is already reporting the syntax error through its own channel.
///
/// Both the dev-server routes and the MCP tools call this, so "what counts as
/// inline" cannot drift between the two surfaces.
pub fn scan_file(file: &str, source: &str) -> (Vec<ScannedComment>, Vec<CommentDiagnostic>) {
    // A literate document's `st` fences ARE the program, and the prose around
    // them is not Spacetime source — so the document is TANGLED first, exactly
    // as every other reader of `.st.md` does. Parsing the raw markdown would
    // fail, and (with the parse-failure rule above) silently yield no comments
    // at all: a comment written in a fence would be visible to `check` and
    // invisible to the pill, one store answering two ways.
    let tangled;
    let source = if file.ends_with(".st.md") {
        tangled = crate::literate::tangle(source).source;
        tangled.as_str()
    } else {
        source
    };
    let Ok(ast) = crate::parser::parse(source) else {
        return (Vec::new(), Vec::new());
    };
    let exclusions: Vec<(usize, usize)> = ast
        .html_blocks
        .iter()
        .map(|block| (block.span.start, block.span.end))
        .collect();
    scan_source_excluding(file, source, &exclusions)
}

/// Exact source bytes occupied by one inline comment header and continuation run.
///
/// `span` is deliberately byte-addressed because the guarded source-write rail
/// compares bytes before it removes anything. It contains ONLY the `//@` header
/// and immediately contiguous `//@>` lines (including their line endings), so
/// a nearby plain `//` comment or source statement cannot be deleted with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineCommentSpan {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

/// Return the removable byte spans for every live inline comment in `source`.
///
/// This shares the parser-derived HTML exclusions used by [`scan_file`]. A
/// parse failure returns no spans: deleting source based on a lexical fallback
/// could erase `//` text visitors can see inside markup. Callers must still
/// prove the returned bytes match their snapshot through the guarded write
/// rail before mutating a file.
pub fn inline_comment_spans(file: &str, source: &str) -> Vec<InlineCommentSpan> {
    let Ok(ast) = crate::parser::parse(source) else {
        return Vec::new();
    };
    let exclusions: Vec<(usize, usize)> = ast
        .html_blocks
        .iter()
        .map(|block| (block.span.start, block.span.end))
        .collect();
    let (scanned, _) = scan_source_excluding(file, source, &exclusions);
    let ids_by_line: BTreeMap<usize, String> = scanned
        .into_iter()
        .map(|comment| (comment.line, comment.id))
        .collect();
    let excluded = |start: usize| exclusions.iter().any(|(a, b)| start >= *a && start < *b);
    let comments: BTreeMap<usize, _> = extract_comments(source)
        .into_iter()
        .filter(|comment| !comment.is_block && !excluded(comment.start))
        .map(|comment| (comment.line, comment))
        .collect();

    ids_by_line
        .into_iter()
        .filter_map(|(line, id)| {
            let header = comments.get(&line)?;
            let mut last = header;
            let mut next_line = line + 1;
            while let Some(next) = comments.get(&next_line) {
                if !matches!(
                    parse_inline_line(&next.text),
                    Some(InlineLine::Continuation(_))
                ) {
                    break;
                }
                last = next;
                next_line += 1;
            }
            // Include the newline only when this is a standalone comment line.
            // An end-of-line `//@` must not take the code before it with it.
            let line_start = source[..header.start]
                .rfind('\n')
                .map_or(0, |newline| newline + 1);
            let end = if source[line_start..header.start].trim().is_empty()
                && source[last.end..].starts_with('\n')
            {
                last.end + 1
            } else {
                last.end
            };
            Some(InlineCommentSpan {
                id,
                file: file.to_string(),
                line,
                start: header.start,
                end,
            })
        })
        .collect()
}

/// THE authorization answer for source deletion: which byte ranges of this
/// file may be removed, recomputed from source and sidecar state.
///
/// A span is deletable only when it is an inline comment whose stored status
/// is `Resolved`. Pruning open work would delete work in progress; pruning
/// anything that is not a comment would delete code.
///
/// This lives HERE, not in the route, because the guarded write rail is
/// reachable from the public dev websocket as well as from the pill. An
/// authorization check that only one caller performs is not an authorization
/// check — it is a convention the next caller will not know about.
///
/// `file_path` is the absolute path being written; the project root is derived
/// from it by walking up to the directory that owns `.comments/`, so the rail
/// does not need the caller to tell it (and cannot be told a lie).
pub fn deletable_spans_in(
    file_path: &Path,
    source: &str,
) -> std::collections::HashSet<(usize, usize)> {
    let Some(project_root) = project_root_for(file_path) else {
        return std::collections::HashSet::new();
    };
    let relative = file_path
        .strip_prefix(&project_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string();

    let (sidecar, _) = read_sidecar(&project_root);
    let resolved: std::collections::HashSet<&str> = sidecar
        .values()
        .filter(|record| record.status == Status::Resolved)
        .map(|record| record.id.as_str())
        .collect();

    inline_comment_spans(&relative, source)
        .into_iter()
        .filter(|span| resolved.contains(span.id.as_str()))
        .map(|span| (span.start, span.end))
        .collect()
}

/// Walk up from a file to the directory that owns its `.comments/` store.
///
/// Falls back to the file's own directory so a project that has never had a
/// comment still resolves — there is simply nothing resolved in it, which is
/// the correct (empty) answer rather than an error.
fn project_root_for(file_path: &Path) -> Option<PathBuf> {
    let start = file_path.parent()?;
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(COMMENTS_DIR).is_dir() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    Some(start.to_path_buf())
}

/// Whether a path is a Spacetime source file this scanner should read.
///
/// Covers `.st` AND `.st.md`: a literate document's fences ARE the program,
/// so a comment written there is as real as one in a plain `.st` file.
/// Missing the literate case made comments visible to `check` but invisible
/// to agents over MCP — one store, two answers.
pub fn is_scannable_source(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    name.ends_with(".st") || name.ends_with(".st.md")
}

/// Whether a caller-supplied file path may be stored in an anchor.
///
/// Anchors arrive from route payloads and from MCP tool arguments — both
/// untrusted — and name a file inside ONE project. An absolute path or a `..`
/// component would let a record claim a location the server was never asked
/// to serve, and (once W4 can write back to source) point a rewrite at it.
/// Both surfaces call THIS function so their trust boundaries cannot drift.
pub fn is_valid_project_file(file: &str) -> bool {
    let path = Path::new(file);
    !file.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
}

/// Why an anchor was refused, phrased for the person who typed it.
///
/// `None` = acceptable. Inline anchors are refused from BOTH write surfaces:
/// an inline comment's identity is a content hash of the source line
/// (`inline_id`), so a record minted with a fresh id could never be matched to
/// the `//@` header it claims — the very next scan would classify it as an
/// orphan. Inline comments are created by TYPING them in the source; the
/// sidecar only carries their workflow state.
pub fn anchor_rejection(anchor: &Anchor) -> Option<String> {
    match anchor {
        Anchor::Inline { .. } => Some(
            "an inline comment is created by writing `//@type: text` in the source file — \
             its identity is derived from that line, so it cannot be minted here. Use a \
             page or file anchor instead; use an element anchor for a specific page element."
                .to_string(),
        ),
        Anchor::File { file, .. } => (!is_valid_project_file(file))
            .then(|| format!("anchor file {file:?} must be a relative path inside the project")),
        Anchor::Page { route } => route
            .is_empty()
            .then(|| "a page anchor needs a route".to_string()),
        Anchor::Element {
            route,
            selector,
            fingerprint,
            ..
        } => {
            if route.is_empty() {
                Some("an element anchor needs a route".to_string())
            } else if selector.is_empty() {
                Some("an element anchor needs a selector".to_string())
            } else if fingerprint.as_ref().is_some_and(|f| !is_valid_id(f)) {
                // A fingerprint is minted by `element_fingerprint`, so a
                // malformed one means a hand-built or corrupted payload. Reject
                // rather than store it: an unmatchable fingerprint would orphan
                // the comment on the very next scan, which reads as data loss.
                Some(
                    "an element anchor fingerprint must be a content hash minted by this \
                     toolchain"
                        .to_string(),
                )
            } else {
                None
            }
        }
        Anchor::Unsupported { kind, .. } => Some(format!(
            "anchor kind {kind:?} is newer than this toolchain and cannot be written"
        )),
    }
}

/// Whether an id is safe to use as a sidecar FILENAME.
///
/// Ids arrive from deserialized JSON and (from W2/W3) from route and MCP
/// payloads, so they are untrusted input that gets joined to a path. An id
/// like `../../etc/thing` would make the write escape `.comments/` entirely
/// and clobber an unrelated file. Restricting to the shapes this module
/// actually mints — hex hashes and ULIDs — closes that without needing a
/// path-canonicalization dance at every call site.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Trim + collapse internal whitespace runs.
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ============================================================================
// Scanning
// ============================================================================

/// A comment harvested from source, before sidecar state is merged in.
#[derive(Debug, Clone, PartialEq)]
pub struct ScannedComment {
    pub id: String,
    pub type_id: String,
    pub meta: BTreeMap<String, String>,
    pub text: String,
    pub file: String,
    pub line: usize,
    pub malformed_meta: Vec<String>,
}

/// Something the scan could not make sense of. Always a warning — see the
/// module invariant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentDiagnostic {
    pub file: String,
    pub line: usize,
    pub message: String,
}

/// Harvest every `//@` comment from one file's source.
///
/// Layered on `extract_comments`, the existing lexical harvester, rather than
/// re-scanning the text: it already knows where comments begin and end and
/// skips string literals, and a second scanner would be a second set of bugs.
///
/// NB HTML MARKUP BLOCKS. Inside an HTML block, `//` is NOT a comment — it is
/// literal text that renders on the page (the same rule that makes a bare
/// `$x` literal there). `extract_comments` is purely lexical and cannot see
/// that distinction, so a caller that HAS parsed the file must pass those
/// byte ranges to `scan_source_excluding`; harvesting them would surface page
/// copy in the pill and let an author believe a visible line was private.
pub fn scan_source(file: &str, source: &str) -> (Vec<ScannedComment>, Vec<CommentDiagnostic>) {
    scan_source_excluding(file, source, &[])
}

/// `scan_source`, ignoring `//@` lines that begin inside any excluded byte
/// range.
///
/// The ranges come from the CALLER's parse (HTML block spans), never from a
/// second scanner in here — structure is the parser's job, and re-deriving it
/// lexically is how two subtly different notions of "inside HTML" get born.
pub fn scan_source_excluding(
    file: &str,
    source: &str,
    exclusions: &[(usize, usize)],
) -> (Vec<ScannedComment>, Vec<CommentDiagnostic>) {
    let mut found = Vec::new();
    let mut diagnostics = Vec::new();

    let excluded = |start: usize| exclusions.iter().any(|(a, b)| start >= *a && start < *b);

    let comments = extract_comments(source);
    // Index line -> comment so adjacency can be tested by line arithmetic;
    // a continuation must be on the line DIRECTLY below its predecessor.
    let mut by_line: BTreeMap<usize, &crate::parser::comments::Comment> = BTreeMap::new();
    for c in &comments {
        if !c.is_block && !excluded(c.start) {
            by_line.insert(c.line, c);
        }
    }

    // The comment currently accepting continuations: (index into `found`, line).
    let mut open: Option<(usize, usize)> = None;

    for (&line, comment) in &by_line {
        match parse_inline_line(&comment.text) {
            Some(InlineLine::Header(h)) => {
                let first_line = h.text.clone();
                let mut id = inline_id(file, &h.type_id, &h.meta, &first_line);
                // Two identical headers in one file are legitimate ("//@todo:
                // add a test" beside two different functions), but they must
                // not share an id: merge would give the first record's status
                // and thread to both, and each write would clobber the other.
                // Disambiguate by OCCURRENCE, not by line — a line number
                // changes whenever anything above is edited, which would
                // re-identify comments on every unrelated edit.
                let occurrence = found
                    .iter()
                    .filter(|f: &&ScannedComment| f.id.starts_with(&id))
                    .count();
                if occurrence > 0 {
                    id = format!("{id}-{occurrence}");
                }
                found.push(ScannedComment {
                    id,
                    type_id: h.type_id,
                    meta: h.meta,
                    text: h.text,
                    file: file.to_string(),
                    line,
                    malformed_meta: h.malformed_meta,
                });
                open = Some((found.len() - 1, line));
            }
            Some(InlineLine::Continuation(body)) => {
                // Contiguity is the whole guarantee: a continuation belongs to
                // the header on the line immediately above, or to nothing.
                match open {
                    Some((idx, prev_line)) if prev_line + 1 == line => {
                        let target = &mut found[idx];
                        if body.is_empty() {
                            target.text.push_str("\n\n");
                        } else {
                            if !target.text.ends_with('\n') && !target.text.is_empty() {
                                target.text.push('\n');
                            }
                            target.text.push_str(&body);
                        }
                        open = Some((idx, line));
                    }
                    _ => {
                        diagnostics.push(CommentDiagnostic {
                            file: file.to_string(),
                            line,
                            message: "continuation without header: `//@>` must sit directly \
                                      below a `//@` header or another `//@>` line. The header \
                                      was probably deleted or moved — the text here is orphaned."
                                .to_string(),
                        });
                        open = None;
                    }
                }
            }
            None => {
                // A plain comment. It does NOT close the run implicitly — it
                // ends it, which is what zero-absorption means.
                open = None;
            }
        }
    }

    (found, diagnostics)
}

// ============================================================================
// Validation against the roster
// ============================================================================

/// Check a scanned comment's type and metadata against the declared roster.
///
/// Returns diagnostics; never fails. An unknown type is reported WITH the
/// roster's contents, because the fix is almost always a typo or a type that
/// belongs in `_prelude.st`, and naming the alternatives is what makes the
/// message actionable.
pub fn validate_against_roster(
    scanned: &ScannedComment,
    roster: &BTreeMap<String, CommentTypeDefAst>,
) -> Vec<CommentDiagnostic> {
    let mut out = Vec::new();
    let at = |message: String| CommentDiagnostic {
        file: scanned.file.clone(),
        line: scanned.line,
        message,
    };

    for m in &scanned.malformed_meta {
        out.push(at(format!(
            "malformed metadata `{m}` — write `key: value` inside the header parens"
        )));
    }

    let Some(def) = roster.get(&scanned.type_id) else {
        let known: Vec<&str> = roster.keys().map(|s| s.as_str()).collect();
        out.push(at(format!(
            "unknown comment type `{}` — declared types are: {}. Add a `%comment_type` \
             to the project's _prelude.st to introduce a new one.",
            scanned.type_id,
            if known.is_empty() {
                "(none)".to_string()
            } else {
                known.join(", ")
            }
        )));
        return out;
    };

    for key in scanned.meta.keys() {
        if def.field(key).is_none() {
            let declared: Vec<&str> = def.fields.iter().map(|f| f.name.as_str()).collect();
            out.push(at(format!(
                "`{}` accepts no field `{key}` — declared fields: {}",
                scanned.type_id,
                if declared.is_empty() {
                    "(none)".to_string()
                } else {
                    declared.join(", ")
                }
            )));
        }
    }

    for field in def.required_fields() {
        if !scanned.meta.contains_key(&field.name) {
            out.push(at(format!(
                "`{}` requires `{}` ({}) — e.g. `//@{}({}: …): …`",
                scanned.type_id,
                field.name,
                field.kind.as_str(),
                scanned.type_id,
                field.name
            )));
        }
    }

    for (key, value) in &scanned.meta {
        let Some(field) = def.field(key) else {
            continue;
        };
        if !value_matches_kind(value, field.kind) {
            out.push(at(format!(
                "`{key}` expects {}, got `{value}`",
                field.kind.as_str()
            )));
        }
    }

    out
}

/// Whether a written value is usable as the declared kind. `Str` accepts
/// anything by construction — the surface is text.
fn value_matches_kind(value: &str, kind: CommentFieldKind) -> bool {
    match kind {
        CommentFieldKind::Str => true,
        CommentFieldKind::Number => value.parse::<f64>().is_ok(),
        CommentFieldKind::Bool => matches!(value, "true" | "false"),
        CommentFieldKind::Ident => {
            !value.is_empty()
                && value
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        }
    }
}

// ============================================================================
// Sidecar store
// ============================================================================

/// The sidecar directory for a project root.
pub fn comments_dir(project_root: &Path) -> PathBuf {
    project_root.join(COMMENTS_DIR)
}

/// Read every sidecar record under a project root, keyed by id.
///
/// A record that fails to parse is reported and SKIPPED, never fatal: one
/// hand-edited or half-merged file must not blind the pill to every other
/// comment in the project.
pub fn read_sidecar(
    project_root: &Path,
) -> (BTreeMap<String, CommentRecord>, Vec<CommentDiagnostic>) {
    let mut out = BTreeMap::new();
    let mut diagnostics = Vec::new();
    let dir = comments_dir(project_root);

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        // A project with no comments is the ordinary case, not a fault.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (out, diagnostics),
        // Anything ELSE (permissions, a file where the directory should be)
        // means state exists and cannot be read. Reporting an empty store
        // there would tell the team their comments are gone.
        Err(e) => {
            diagnostics.push(CommentDiagnostic {
                file: dir.display().to_string(),
                line: 0,
                message: format!("could not read the comments directory: {e}"),
            });
            return (out, diagnostics);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let display = path.display().to_string();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                diagnostics.push(CommentDiagnostic {
                    file: display,
                    line: 0,
                    message: format!("could not read comment record: {e}"),
                });
                continue;
            }
        };
        match serde_json::from_str::<CommentRecord>(&text) {
            Ok(mut rec) => {
                // Lenient upgrade: an older record is migrated in memory and
                // persisted at the current version on its next write.
                if rec.v < SCHEMA_VERSION {
                    rec.v = SCHEMA_VERSION;
                }
                // Two files claiming one id would silently collapse to
                // whichever the OS listed last, losing a thread with no
                // diagnostic — a plausible merge artifact, so say it.
                if let Some(prev) = out.get(&rec.id) {
                    diagnostics.push(CommentDiagnostic {
                        file: display.clone(),
                        line: 0,
                        message: format!(
                            "duplicate comment id `{}` (also in another record) — keeping the \
                             first; the other file's status and thread are being ignored. \
                             This usually means a merge duplicated a record.",
                            prev.id
                        ),
                    });
                    continue;
                }
                out.insert(rec.id.clone(), rec);
            }
            Err(e) => diagnostics.push(CommentDiagnostic {
                file: display,
                line: 0,
                message: format!("malformed comment record (skipped): {e}"),
            }),
        }
    }

    (out, diagnostics)
}

/// Write one record atomically (temp file + rename).
///
/// Atomicity is per FILE, which is why there is one file per comment: a reader
/// scanning the directory mid-write sees either the old record or the new one,
/// never a truncated document. Concurrent read-modify-write of the SAME record
/// from two processes is still last-writer-wins — acceptable while both writers
/// are one developer's machine, and the reason the hosted tier owns this with a
/// real transaction instead.
pub fn write_record(project_root: &Path, record: &CommentRecord) -> std::io::Result<()> {
    // An id is a FILENAME here, and ids reach this function from JSON on disk
    // and (W2/W3) from route and MCP payloads. Refuse anything that could
    // traverse out of the directory before it is joined to a path.
    if !is_valid_id(&record.id) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "refusing to write comment record with unsafe id {:?} — ids are \
                 alphanumeric with `-`/`_`, max 64 chars",
                record.id
            ),
        ));
    }

    let dir = comments_dir(project_root);
    // A SYMLINK where `.comments/` should be would silently redirect every
    // write out of the project — create_dir_all follows it, and the records
    // land wherever it points. The store belongs to one project; refuse
    // rather than follow.
    if let Ok(meta) = std::fs::symlink_metadata(&dir)
        && meta.file_type().is_symlink()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "{} is a symlink — refusing to write comment records through it",
                dir.display()
            ),
        ));
    }
    std::fs::create_dir_all(&dir)?;
    let final_path = dir.join(format!("{}.json", record.id));
    // A UNIQUE temp file per write. Sharing one temp path per id lets two
    // concurrent writers interleave such that the first rename publishes the
    // SECOND writer's bytes while reporting success to the first — last
    // writer wins is acceptable, attributing success to the wrong write is
    // not.
    let tmp_path = dir.join(format!(
        ".{}.{}.{}.tmp",
        record.id,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let json = serde_json::to_string_pretty(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp_path, json.as_bytes())?;
    std::fs::rename(&tmp_path, &final_path)?;
    Ok(())
}

/// Delete one sidecar record by its safe id.
///
/// Dismissal is deliberately a separate store operation from `write_record`:
/// callers must opt into destroying team conversation data, and an unsafe id
/// must never become a path beneath `.comments/`.
pub fn remove_record(project_root: &Path, id: &str) -> std::io::Result<()> {
    if !is_valid_id(id) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("refusing to remove comment record with unsafe id {id:?}"),
        ));
    }

    let dir = comments_dir(project_root);
    if let Ok(meta) = std::fs::symlink_metadata(&dir)
        && meta.file_type().is_symlink()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "{} is a symlink — refusing to remove comment records through it",
                dir.display()
            ),
        ));
    }
    std::fs::remove_file(dir.join(format!("{id}.json")))
}

// ============================================================================
// Merge
// ============================================================================

/// The full picture for a project: content from source, state from sidecar.
#[derive(Debug, Clone, Default)]
pub struct CommentIndex {
    /// Every comment, id order.
    pub records: Vec<CommentRecord>,
    /// Everything the scan could not make sense of.
    pub diagnostics: Vec<CommentDiagnostic>,
    /// Sidecar records whose inline anchor no longer exists — the comment was
    /// edited or deleted in source while its state lived on. Surfaced rather
    /// than dropped so the state can be re-anchored or dismissed deliberately.
    pub orphans: Vec<CommentRecord>,
}

/// Merge scanned inline comments with sidecar state.
///
/// Inline content WINS over the stored copy of the same comment: the source is
/// where the text lives, so a record's cached text is only a convenience for
/// readers that never open the file. Stored workflow state wins over defaults,
/// since it is the part source cannot express.
///
/// `scanned_files` is THE COVERAGE OF THIS SCAN — the files actually read.
/// Orphan classification is restricted to it, because "the sidecar has an
/// inline record I did not see" means *deleted* only if the file was looked
/// at. A caller scanning one file while loading the whole project's sidecar
/// would otherwise present every live comment in every OTHER file as an
/// orphan awaiting dismissal — destroying real threads on a filtered read.
pub fn merge(
    scanned: Vec<ScannedComment>,
    sidecar: BTreeMap<String, CommentRecord>,
    scanned_files: &[String],
    now: &str,
) -> CommentIndex {
    merge_inner(scanned, sidecar, Some(scanned_files), now)
}

/// `merge` for a scan that covered the ENTIRE project — every inline record
/// with no live anchor is genuinely orphaned.
pub fn merge_full_scan(
    scanned: Vec<ScannedComment>,
    sidecar: BTreeMap<String, CommentRecord>,
    now: &str,
) -> CommentIndex {
    merge_inner(scanned, sidecar, None, now)
}

fn merge_inner(
    scanned: Vec<ScannedComment>,
    mut sidecar: BTreeMap<String, CommentRecord>,
    coverage: Option<&[String]>,
    now: &str,
) -> CommentIndex {
    let mut records = Vec::new();
    let mut diagnostics = Vec::new();

    for s in scanned {
        let record = match sidecar.remove(&s.id) {
            Some(mut stored) => {
                stored.type_id = s.type_id;
                stored.text = s.text;
                stored.meta = s.meta;
                stored.anchor = Anchor::Inline {
                    file: s.file,
                    line: s.line,
                };
                stored.inline = true;
                stored
            }
            // An inline comment with no sidecar file yet is a REAL comment,
            // open by default — writing state to disk is what claiming or
            // replying does, not what mentioning something does.
            None => CommentRecord {
                id: s.id,
                type_id: s.type_id,
                status: Status::Open,
                author: Author {
                    kind: AuthorKind::Human,
                    name: String::new(),
                },
                claimed_by: None,
                anchor: Anchor::Inline {
                    file: s.file,
                    line: s.line,
                },
                text: s.text,
                meta: s.meta,
                thread: Vec::new(),
                history: Vec::new(),
                created_at: now.to_string(),
                updated_at: None,
                v: SCHEMA_VERSION,
                inline: true,
            },
        };
        records.push(record);
    }

    // Whatever is left in the sidecar either lives independently of source
    // (page/file anchors, pill-created) or lost its inline anchor.
    let mut orphans = Vec::new();
    for (_, rec) in sidecar {
        match &rec.anchor {
            Anchor::Inline { file, line } => {
                // Only a file this scan actually READ can prove a comment is
                // gone. Outside the coverage set the record is simply not
                // this scan's business — pass it through untouched.
                let covered = match coverage {
                    None => true,
                    Some(files) => files.iter().any(|f| f == file),
                };
                if !covered {
                    records.push(rec);
                    continue;
                }
                diagnostics.push(CommentDiagnostic {
                    file: file.clone(),
                    line: *line,
                    message: format!(
                        "orphaned comment `{}`: its `//@` header is gone from source, but its \
                         status and thread survive. Re-anchor it or dismiss it.",
                        rec.id
                    ),
                });
                orphans.push(rec);
            }
            _ => records.push(rec),
        }
    }

    records.sort_by(|a, b| a.id.cmp(&b.id));
    orphans.sort_by(|a, b| a.id.cmp(&b.id));

    CommentIndex {
        records,
        diagnostics,
        orphans,
    }
}

#[cfg(test)]
mod tests;
