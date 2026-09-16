//! Literate Spacetime — Markdown as host (`.st.md`). See
//! `docs/specs/SIP-002-literate-spacetime-markdown-host.org`.
//!
//! A `.st.md` file is a Markdown document whose fenced `st` code blocks ARE
//! the program. [`tangle`] rewrites it into ordinary `.st` source BEFORE
//! parsing: prose segments become `@doc(content: …)` sections (the existing
//! `stdlib/md` rail), `st` fences splice through verbatim, and **document
//! order is preserved** — so a fence that declares markup renders its live
//! result exactly where it sits in the essay.
//!
//! The pass is pure sugar-elimination: it only synthesizes machinery that
//! already exists (`@doc`, tag literals, selector scopes). Nothing downstream
//! — parsing, form matching, `inject_doc_content`, SSG unroll, hydration —
//! changes. It runs at file-read time, keyed by extension, which is why it
//! lives here and not in `compiler.rs`: it produces `.st` source and never
//! sees an AST.
//!
//! ```text
//! document ::= segment*
//! segment  ::= prose | fence
//! fence    ::= <ticks>st [flag*] NEWLINE st-source NEWLINE <ticks>
//! flag     ::= hidden | src
//! ```
//!
//! Prose is INERT (rendered by snarkdown, never hole-interpolated) — the
//! markdown code-span backtick and the Spacetime hole backtick would
//! otherwise collide, and one sigil means one thing. A live value in prose is
//! written the Spacetime-native way: an element plus a `text <- $sig` binding
//! in a fence (which is also first-paint-correct).

use std::path::Path;

/// Class applied to every synthesized prose mount element.
pub const PROSE_CLASS: &str = "lit-prose";
/// Class applied to every synthesized `st src` verbatim source block.
pub const SRC_CLASS: &str = "lit-src";

/// Maps 1-indexed lines of tangled `.st` output back to 1-indexed lines of the
/// `.st.md` source, so diagnostics point at what the author actually wrote.
///
/// Synthesized lines (the section/`@doc` wiring, the `lit-src` block) have no
/// author-authored counterpart; they map to the line of the segment that
/// produced them, which is the useful answer for a reader.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineMap {
    /// `out_to_src[i]` is the source line for output line `i + 1`.
    out_to_src: Vec<usize>,
}

impl LineMap {
    /// Source line (1-indexed) for a 1-indexed line of tangled output.
    /// Out-of-range lines clamp to the last known mapping (or the input line
    /// when empty), so a diagnostic never panics on a synthesized tail line.
    pub fn source_line(&self, out_line: usize) -> usize {
        if self.out_to_src.is_empty() {
            return out_line.max(1);
        }
        let idx = out_line.saturating_sub(1);
        *self
            .out_to_src
            .get(idx)
            .unwrap_or_else(|| self.out_to_src.last().expect("non-empty"))
    }

    /// Number of mapped output lines.
    pub fn len(&self) -> usize {
        self.out_to_src.len()
    }

    /// True when nothing has been mapped.
    pub fn is_empty(&self) -> bool {
        self.out_to_src.is_empty()
    }

    /// Translate a byte offset in TANGLED output to the equivalent byte offset
    /// in the `.st.md` source, so a diagnostic points at the line the author
    /// actually wrote.
    ///
    /// Fence bodies splice through VERBATIM, so for the case that matters —
    /// an error inside `st` code — the column is preserved exactly and the
    /// mapped position is the true one. For a synthesized line (the `@doc`
    /// wiring) the column is meaningless, so we clamp to the start of the
    /// originating line: pointing at the right line beats pointing confidently
    /// at the wrong column.
    pub fn translate_offset(&self, out_offset: usize, tangled: &str, original: &str) -> usize {
        if self.out_to_src.is_empty() {
            return out_offset.min(original.len());
        }
        // Locate (line, column) of `out_offset` in the tangled text.
        let clamped = out_offset.min(tangled.len());
        let prefix = &tangled[..clamped];
        let out_line = prefix.matches('\n').count() + 1; // 1-indexed
        let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let column = clamped - line_start;

        let src_line = self.source_line(out_line);
        let src_line_start = line_start_offset(original, src_line);
        let src_line_len = original[src_line_start..]
            .find('\n')
            .unwrap_or(original.len() - src_line_start);
        // Verbatim splices keep the column; synthesized lines clamp to the line.
        (src_line_start + column.min(src_line_len)).min(original.len())
    }
}

/// Byte offset of the start of 1-indexed `line` in `text` (clamped to the end).
fn line_start_offset(text: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut seen = 1usize;
    for (i, c) in text.char_indices() {
        if c == '\n' {
            seen += 1;
            if seen == line {
                return i + 1;
            }
        }
    }
    text.len()
}

/// Rewrite parse-error offsets from TANGLED coordinates into `.st.md` source
/// coordinates, so miette renders the author's own file.
///
/// Returns the errors unchanged when there is no map (a plain `.st` file).
pub fn remap_parse_errors(
    errors: &crate::parser::ParseErrors,
    map: Option<&LineMap>,
    tangled: &str,
    original: &str,
) -> crate::parser::ParseErrors {
    let Some(map) = map else {
        return errors.clone();
    };
    let remapped: Vec<crate::parser::ParseError> = errors
        .iter()
        .map(|e| {
            let start = map.translate_offset(e.offset, tangled, original);
            let end = map.translate_offset(e.offset + e.len, tangled, original);
            crate::parser::ParseError::new(e.message.clone(), start, end.saturating_sub(start))
        })
        .collect();
    crate::parser::ParseErrors::new(remapped)
}

/// Result of tangling a `.st.md` document.
#[derive(Debug, Clone)]
pub struct Tangled {
    /// Ordinary `.st` source, ready for `parse()`.
    pub source: String,
    /// Output-line to source-line mapping for diagnostics.
    pub map: LineMap,
    /// Non-fatal problems (e.g. an unterminated fence). Callers may surface
    /// them; tangling always produces usable output so an editor keeps working.
    pub warnings: Vec<String>,
}

/// True when `path` is a literate Spacetime document (`*.st.md`).
pub fn is_literate(path: &Path) -> bool {
    path.to_str()
        .map(|s| s.ends_with(".st.md"))
        .unwrap_or(false)
}

/// True when `path` is compilable Spacetime source: `*.st`, `*.st.md`, or `*.edn`.
pub fn is_spacetime_source(path: &Path) -> bool {
    // `.edn` is a peer concrete syntax (PLAN-148): any place that asks "is
    // this Spacetime source?" must answer yes for EDN too, or discovery
    // surfaces (check, build) silently skip EDN pages.
    is_literate(path)
        || path
            .extension()
            .map(|e| e == "st" || e == "edn")
            .unwrap_or(false)
}

/// Read a Spacetime source file, tangling it first when it is literate.
///
/// THE ingress helper: every entry point that used to call
/// `fs::read_to_string` on a `.st` path calls this instead, so `.st.md`
/// support is one call-site change per entry point rather than a parallel
/// pipeline.
pub fn read_source(path: &Path) -> std::io::Result<(String, Option<LineMap>)> {
    let raw = std::fs::read_to_string(path)?;
    if is_literate(path) {
        let tangled = tangle_seeded(&raw, &file_seed(path));
        Ok((tangled.source, Some(tangled.map)))
    } else {
        Ok((raw, None))
    }
}

/// Deterministic per-file namespace for a literate file's synthesized
/// segment ids: FNV-1a over the path, hex-encoded. Uniqueness matters within
/// one merged page (a host and every `.st.md` it imports); determinism keeps
/// rebuilds diff-stable. No deps — the hash is two lines.
fn file_seed(path: &Path) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in path.to_string_lossy().as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// What a fence's info string asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FenceKind {
    /// `st` — tangle, do not show the source.
    Run,
    /// `st src` — tangle AND show the source above the result.
    RunWithSource,
    /// `st hidden` — tangle, show nothing (imports, theme plumbing).
    Hidden,
    /// Any other language — inert; stays part of the prose.
    Other,
}

/// Classify a fence info string (the text after the opening backticks).
///
/// Flags are DATA in the info line, not a Rust variant per case: `st`, plus
/// any of the space-separated flags `src` / `hidden`. An unknown flag is
/// ignored (forward-compatible); an unknown language is `Other`.
fn classify(info: &str) -> FenceKind {
    let mut parts = info.split_whitespace();
    match parts.next() {
        Some("st") => {
            let mut kind = FenceKind::Run;
            for flag in parts {
                match flag {
                    "src" => kind = FenceKind::RunWithSource,
                    "hidden" => kind = FenceKind::Hidden,
                    _ => {}
                }
            }
            kind
        }
        _ => FenceKind::Other,
    }
}

/// A fence opener as `(indent, tick_count, info)`, or `None` when `line` is not one.
fn fence_open(line: &str) -> Option<(usize, usize, &str)> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    // CommonMark: a fence may be indented up to 3 spaces.
    if indent > 3 {
        return None;
    }
    let ticks = trimmed.chars().take_while(|c| *c == '`').count();
    if ticks < 3 {
        return None;
    }
    let info = trimmed[ticks..].trim();
    // An info string may not itself contain a backtick (CommonMark).
    if info.contains('`') {
        return None;
    }
    Some((indent, ticks, info))
}

/// True when `line` closes a fence opened with `ticks` backticks.
fn fence_close(line: &str, ticks: usize) -> bool {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return false;
    }
    let run = trimmed.chars().take_while(|c| *c == '`').count();
    run >= ticks && trimmed[run..].trim().is_empty()
}

/// Escape text for a Spacetime double-quoted string literal.
fn escape_st_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escape text for literal display inside an HTML element.
///
/// The backtick MUST be escaped as a numeric entity: in Spacetime markup a
/// backtick opens a HOLE, so a shown-source block containing `` `$title` ``
/// would otherwise be lexed as an interpolation and rendered as an empty
/// string — silently deleting the very syntax the block is teaching. `&#96;`
/// displays as a backtick and is invisible to the hole lexer.
fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '`' => out.push_str("&#96;"),
            '\r' => {}
            _ => out.push(ch),
        }
    }
    out
}

/// Accumulates tangled output while recording each output line's source line.
struct Emitter {
    out: String,
    map: Vec<usize>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            map: Vec::new(),
        }
    }

    /// Push one output line attributed to 1-indexed `src_line`.
    fn line(&mut self, text: &str, src_line: usize) {
        self.out.push_str(text);
        self.out.push('\n');
        self.map.push(src_line.max(1));
    }

    /// Prepend a synthesized line (attributed to source line 1) ahead of all
    /// emitted output, keeping the line map aligned.
    fn prepend_synthesized(&mut self, text: &str) {
        self.out = format!("{text}\n\n{}", self.out);
        self.map.insert(0, 1);
        self.map.insert(0, 1);
    }

    /// Push a multi-line block whose first line came from `first_src_line`,
    /// attributing each subsequent line to the following source line (a
    /// verbatim splice keeps exact fidelity).
    fn block_verbatim(&mut self, text: &str, first_src_line: usize) {
        for (i, l) in text.split('\n').enumerate() {
            self.line(l.trim_end_matches('\r'), first_src_line + i);
        }
    }
}

/// Tangle a `.st.md` document into ordinary `.st` source.
///
/// Pure: no filesystem, no globals. See the module docs for the shape.
pub fn tangle(input: &str) -> Tangled {
    tangle_seeded(input, "")
}

/// Tangle with a per-file `seed` namespacing every synthesized segment id
/// (`data-lit-seg`, `data-lit-src`). A standalone page needs no namespace —
/// but when a page `@import`s other `.st.md` files, each file numbers its
/// segments from 0, and identical `[data-lit-seg="0"] { @doc(content: …) }`
/// rules collide across the merged page (CSS last-writer-wins: every prose
/// section rendered the LAST import's prose, and the host's vanished).
/// The seed keeps each file's ids distinct so every segment keeps its own
/// prose.
pub fn tangle_seeded(input: &str, seed: &str) -> Tangled {
    let lines: Vec<&str> = input.split('\n').collect();
    let mut em = Emitter::new();
    let mut warnings = Vec::new();

    let mut prose: Vec<&str> = Vec::new();
    let mut prose_start = 1usize;
    let mut seg = 0usize;

    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let src_line = i + 1;

        let Some((_indent, ticks, info)) = fence_open(line) else {
            if prose.is_empty() {
                prose_start = src_line;
            }
            prose.push(line);
            i += 1;
            continue;
        };

        let kind = classify(info);

        // Find the closing fence.
        let mut end = None;
        let mut j = i + 1;
        while j < lines.len() {
            if fence_close(lines[j], ticks) {
                end = Some(j);
                break;
            }
            j += 1;
        }
        let close = match end {
            Some(j) => j,
            None => {
                warnings.push(format!(
                    "unterminated code fence opened on line {src_line}: treating the rest of the document as its body"
                ));
                lines.len()
            }
        };

        if kind == FenceKind::Other {
            // Inert fence: it IS prose. Keep it verbatim, fence markers included,
            // so the markdown renderer shows a normal code block.
            if prose.is_empty() {
                prose_start = src_line;
            }
            for l in lines.iter().take(close.min(lines.len())).skip(i) {
                prose.push(l);
            }
            if close < lines.len() {
                prose.push(lines[close]);
            }
            i = close + 1;
            continue;
        }

        // An `st` fence ends the current prose segment.
        flush_prose(&mut em, &mut prose, prose_start, &mut seg, seed);

        let body_start = i + 1;
        let body_end = close.min(lines.len());
        let body = lines[body_start..body_end].join("\n");

        if kind == FenceKind::RunWithSource {
            let id = seg_label(seed, seg);
            seg += 1;
            em.line(
                &format!(
                    "<pre class=\"{SRC_CLASS}\" data-lit-src=\"{id}\"><code>{}</code></pre>",
                    escape_html(&body)
                ),
                src_line,
            );
            em.line("", src_line);
        }

        // The fence body splices through VERBATIM — this is the whole point:
        // a fence is real Spacetime source at its document position.
        em.block_verbatim(&body, body_start + 1);
        em.line("", body_end.max(body_start) + 1);

        i = close + 1;
    }

    flush_prose(&mut em, &mut prose, prose_start, &mut seg, seed);

    // Prose is rendered by the `md` module, so a document containing prose
    // needs `@import "stdlib/md"`. Synthesizing it here (rather than making
    // every author open with a hidden import fence) keeps the boilerplate at
    // zero AND removes a silent-failure footgun: forgetting the import would
    // otherwise leave every prose block blank with no error. Skipped when the
    // author already imports it, and when the document is all code.
    if seg > 0 && !em.out.contains("stdlib/md") {
        em.prepend_synthesized("@import \"stdlib/md\"");
    }

    Tangled {
        source: em.out,
        map: LineMap { out_to_src: em.map },
        warnings,
    }
}

/// Segment label: a bare number when unseeded (standalone pages keep their
/// historic `data-lit-seg="0"`), `seed-n` when a file seed is present.
fn seg_label(seed: &str, n: usize) -> String {
    if seed.is_empty() {
        n.to_string()
    } else {
        format!("{seed}-{n}")
    }
}

/// Emit accumulated prose as a mount element plus an `@doc(content:)` scope.
/// Whitespace-only prose emits nothing — blank lines between fences are not a
/// segment.
fn flush_prose(em: &mut Emitter, prose: &mut Vec<&str>, start: usize, seg: &mut usize, seed: &str) {
    let text = prose.join("\n");
    prose.clear();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    let id = seg_label(seed, *seg);
    *seg += 1;
    em.line(
        &format!("<section class=\"{PROSE_CLASS}\" data-lit-seg=\"{id}\"></section>"),
        start,
    );
    em.line(
        &format!(
            "[data-lit-seg=\"{id}\"] {{ @doc(content: \"{}\") }}",
            escape_st_string(trimmed)
        ),
        start,
    );
    em.line("", start);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Build a fenced block without embedding literal backtick runs in the
    /// test source (keeps the file readable and heredoc-safe).
    fn fence(ticks: usize, info: &str, body: &str) -> String {
        let t = "`".repeat(ticks);
        format!("{t}{info}\n{body}\n{t}\n")
    }

    #[test]
    fn detects_literate_extension() {
        assert!(is_literate(&PathBuf::from("a/counter.st.md")));
        assert!(!is_literate(&PathBuf::from("a/counter.st")));
        assert!(!is_literate(&PathBuf::from("a/README.md")));
        assert!(is_spacetime_source(&PathBuf::from("x.st.md")));
        assert!(is_spacetime_source(&PathBuf::from("x.st")));
        assert!(!is_spacetime_source(&PathBuf::from("x.md")));
    }

    #[test]
    fn prose_becomes_a_doc_section() {
        let t = tangle("# Title\n\nSome **prose**.\n");
        assert!(
            t.source
                .contains("<section class=\"lit-prose\" data-lit-seg=\"0\"></section>"),
            "mount element missing: {}",
            t.source
        );
        assert!(
            t.source.contains(
                "[data-lit-seg=\"0\"] { @doc(content: \"# Title\\n\\nSome **prose**.\") }"
            ),
            "doc call wrong: {}",
            t.source
        );
    }

    #[test]
    fn st_fence_splices_verbatim() {
        let doc = format!("intro\n\n{}", fence(3, "st", ".a { color: red; }"));
        let t = tangle(&doc);
        assert!(t.source.contains(".a { color: red; }"), "{}", t.source);
        assert!(
            !t.source.contains("@doc(content: \".a {"),
            "code must not be wrapped as prose: {}",
            t.source
        );
    }

    #[test]
    fn document_order_is_preserved() {
        let doc = format!(
            "first\n\n{}\nsecond\n",
            fence(3, "st", "<div class=\"demo\"></div>")
        );
        let t = tangle(&doc);
        let prose0 = t.source.find("data-lit-seg=\"0\"").expect("prose 0");
        let demo = t.source.find("class=\"demo\"").expect("demo");
        let prose1 = t.source.find("data-lit-seg=\"1\"").expect("prose 1");
        assert!(prose0 < demo && demo < prose1, "order wrong: {}", t.source);
    }

    #[test]
    fn src_flag_emits_verbatim_source_block_before_the_result() {
        let t = tangle(&fence(3, "st src", "<div class=\"demo\"></div>"));
        let src = t.source.find("class=\"lit-src\"").expect("src block");
        let live = t.source.rfind("class=\"demo\"").expect("live markup");
        assert!(src < live, "source block must precede result: {}", t.source);
        assert!(
            t.source.contains("&lt;div class=&quot;demo&quot;"),
            "shown source must be escaped data: {}",
            t.source
        );
    }

    #[test]
    fn shown_source_escapes_backticks_so_holes_are_not_eaten() {
        // A backtick opens a HOLE in Spacetime markup. Without escaping, the
        // `lit-src` block teaching `` `$title` `` would be lexed as an
        // interpolation and render EMPTY — deleting the syntax being taught.
        let t = tangle(&fence(3, "st src", "<h4>`$title`</h4>"));
        assert!(
            t.source.contains("&#96;$title&#96;"),
            "backticks must be entity-escaped in shown source: {}",
            t.source
        );
        // The RUN copy keeps its real backticks — it is the program.
        assert!(
            t.source.contains("<h4>`$title`</h4>"),
            "spliced copy must keep real holes: {}",
            t.source
        );
    }

    #[test]
    fn hidden_flag_runs_without_showing_source() {
        let t = tangle(&fence(3, "st hidden", "@data inline $n : 0;"));
        assert!(t.source.contains("@data inline $n : 0;"), "{}", t.source);
        assert!(!t.source.contains("lit-src"), "{}", t.source);
    }

    #[test]
    fn non_st_fences_stay_inert_prose() {
        let t = tangle(&fence(3, "spacetime", ".a { @on &.click { $n <- 1; } }"));
        assert!(t.source.contains("@doc(content:"), "{}", t.source);
        assert!(
            t.source.contains("\\n.a { @on &.click"),
            "quoted code must live inside the doc string: {}",
            t.source
        );
        assert!(
            !t.source.starts_with(".a {"),
            "must not splice as code: {}",
            t.source
        );
    }

    #[test]
    fn escapes_quotes_backslashes_and_newlines_in_prose() {
        let t = tangle("He said \"hi\" \\ there\n\nnext\n");
        assert!(t.source.contains("\\\"hi\\\""), "{}", t.source);
        assert!(t.source.contains("\\\\ there"), "{}", t.source);
        let doc_line = t
            .source
            .lines()
            .find(|l| l.starts_with("[data-lit-seg="))
            .expect("doc line");
        assert!(
            doc_line.ends_with(") }"),
            "doc call must stay one line: {doc_line}"
        );
    }

    #[test]
    fn handles_four_tick_fences_and_nested_backticks() {
        let inner = format!("{}\n// {} not a close", ".a { color: red; }", "`".repeat(3));
        let t = tangle(&fence(4, "st", &inner));
        assert!(t.source.contains("not a close"), "{}", t.source);
        assert!(t.source.contains(".a { color: red; }"), "{}", t.source);
    }

    #[test]
    fn unterminated_fence_warns_and_still_tangles() {
        let open = "`".repeat(3);
        let t = tangle(&format!("{open}st\n.a {{ color: red; }}\n"));
        assert_eq!(t.warnings.len(), 1, "{:?}", t.warnings);
        assert!(t.warnings[0].contains("unterminated"), "{:?}", t.warnings);
        assert!(t.source.contains(".a { color: red; }"), "{}", t.source);
    }

    #[test]
    fn crlf_input_does_not_leak_carriage_returns() {
        let t = tangle("# Title\r\n\r\nprose\r\n");
        assert!(!t.source.contains('\r'), "CR leaked: {:?}", t.source);
    }

    #[test]
    fn blank_lines_between_fences_are_not_a_segment() {
        let doc = format!(
            "{}\n\n{}",
            fence(3, "st", ".a { color: red; }"),
            fence(3, "st", ".b { color: blue; }")
        );
        let t = tangle(&doc);
        assert!(!t.source.contains("lit-prose"), "{}", t.source);
    }

    #[test]
    fn line_map_points_diagnostics_at_authored_lines() {
        // 1: prose / 2: blank / 3: fence open / 4: code / 5: fence close
        let doc = format!("prose\n\n{}", fence(3, "st", ".a { color: red; }"));
        let t = tangle(&doc);
        let out_line = t
            .source
            .lines()
            .position(|l| l.contains(".a { color: red; }"))
            .expect("spliced line")
            + 1;
        assert_eq!(
            t.map.source_line(out_line),
            4,
            "map: {:?} / source:\n{}",
            t.map,
            t.source
        );
        assert_eq!(t.map.source_line(1), 1);
        assert!(t.map.source_line(9_999) >= 1, "out-of-range must clamp");
        assert!(!t.map.is_empty() && t.map.len() >= 3);
    }

    #[test]
    fn prose_documents_auto_import_the_md_module() {
        // Prose renders via `stdlib/md`; forgetting the import would leave every
        // prose block silently blank, so the tangler synthesizes it.
        let t = tangle("# Title\n\nprose\n");
        assert!(
            t.source.starts_with("@import \"stdlib/md\""),
            "auto-import missing: {}",
            t.source
        );
    }

    #[test]
    fn author_import_is_not_duplicated() {
        let doc = format!(
            "prose\n\n{}",
            fence(3, "st hidden", "@import \"stdlib/md\"")
        );
        let t = tangle(&doc);
        assert_eq!(
            t.source.matches("stdlib/md").count(),
            1,
            "import duplicated: {}",
            t.source
        );
    }

    #[test]
    fn all_code_documents_do_not_gain_an_import() {
        let t = tangle(&fence(3, "st", ".a { color: red; }"));
        assert!(
            !t.source.contains("stdlib/md"),
            "code-only doc must not gain the md import: {}",
            t.source
        );
    }

    #[test]
    fn translate_offset_maps_a_fence_error_to_the_authored_line_and_column() {
        // W4: the parser sees TANGLED text, so its byte offsets address lines
        // the author never wrote. A fence body splices VERBATIM, so both line
        // AND column must survive the round trip.
        let original = format!(
            "# Title\n\nprose\n\n{}st\n<div class=\"ok\"></div>\n.broken {{ BAD }}\n{}\n",
            "`".repeat(3),
            "`".repeat(3)
        );
        let t = tangle(&original);
        // Find `BAD` in the tangled output, translate back, and confirm we land
        // on the same token in the ORIGINAL document.
        let out_off = t.source.find("BAD").expect("token in tangled output");
        let src_off = t.map.translate_offset(out_off, &t.source, &original);
        assert_eq!(
            &original[src_off..src_off + 3],
            "BAD",
            "translated offset must land on the same token; got {:?}",
            &original[src_off.saturating_sub(10)..(src_off + 10).min(original.len())]
        );
        // And that position is line 7 of the authored file.
        let line = original[..src_off].matches('\n').count() + 1;
        assert_eq!(line, 7, "authored line wrong");
    }

    #[test]
    fn translate_offset_is_identity_without_a_map() {
        let empty = LineMap::default();
        assert_eq!(empty.translate_offset(5, "abcdefgh", "abcdefgh"), 5);
        // Clamps rather than panicking when the original is shorter.
        assert_eq!(empty.translate_offset(99, "abcdefgh", "abc"), 3);
    }

    #[test]
    fn remap_parse_errors_is_a_no_op_for_plain_st_files() {
        use crate::parser::{ParseError, ParseErrors};
        let errs = ParseErrors::new(vec![ParseError::new("boom", 4, 2)]);
        let out = remap_parse_errors(&errs, None, "tangled", "original");
        assert_eq!(out.first().offset, 4);
        assert_eq!(out.first().len, 2);
    }

    #[test]
    fn remap_parse_errors_rewrites_offsets_into_source_coordinates() {
        use crate::parser::{ParseError, ParseErrors};
        let original = format!(
            "prose line\n\n{}st\n.broken {{ BAD }}\n{}\n",
            "`".repeat(3),
            "`".repeat(3)
        );
        let t = tangle(&original);
        let out_off = t.source.find("BAD").expect("token");
        let errs = ParseErrors::new(vec![ParseError::new("expected value", out_off, 3)]);
        let remapped = remap_parse_errors(&errs, Some(&t.map), &t.source, &original);
        let off = remapped.first().offset;
        assert_eq!(&original[off..off + 3], "BAD", "remap missed the token");
        assert_eq!(remapped.first().len, 3, "span length must survive");
    }

    #[test]
    fn empty_document_tangles_to_nothing() {
        let t = tangle("");
        assert!(t.source.trim().is_empty());
        assert!(t.warnings.is_empty());
    }
}
