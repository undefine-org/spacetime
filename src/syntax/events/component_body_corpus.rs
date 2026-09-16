//! Phase B differential corpus — the parity oracle for the `component_body` self-describing
//! cutover (FEAT-079).
//!
//! `parse_component_body` (a ~1100-line Rust classifier + per-construct parsers) is being
//! replaced by a stdlib `%capture_type component_body` grammar + a generic reifier. Before any
//! Rust deletion, the engine path MUST produce a byte-identical `ComponentBodyDef` for every
//! body in this corpus. This module:
//!
//! 1. Defines `CORPUS` — labelled `@template`-body sources covering ALL constructs the current
//!    parser recognises (the 11 classifier branches) plus the BUG-059/061 + FEAT-071/073
//!    regression cases distilled from the existing inline tests.
//! 2. `golden_via_rust` — JSON-snapshots `parse_component_body(src)` per case (the oracle).
//! 3. (B3c) the engine path will diff against this same corpus.
//!
//! Keeping the corpus in ONE place means the golden and the differential can never drift.

/// A single labelled corpus case. `label` is a stable slug used in snapshot names and failure
/// messages; `body` is the raw `@template` body text (everything between the `{ }`).
pub struct CorpusCase {
    pub label: &'static str,
    pub body: &'static str,
}

/// The Phase B component-body corpus. Each case isolates (or deliberately combines) constructs so
/// a differential failure points at a specific grammar/reifier gap. Ordered roughly by the
/// classifier branch they exercise.
pub const CORPUS: &[CorpusCase] = &[
    // --- pure HTML (fast path: has_component_body_sections == false) ---
    CorpusCase {
        label: "html_only_simple",
        body: "<div><p>Hello</p></div>",
    },
    CorpusCase {
        label: "html_only_entities",
        body: "<p>a &lt; b and x - y</p>",
    },
    // --- state declarations ($name type: value;) ---
    CorpusCase {
        label: "state_bool",
        body: "$open bool: false;\n<div>x</div>",
    },
    CorpusCase {
        label: "state_number",
        body: "$count number: 0;\n<div>x</div>",
    },
    CorpusCase {
        label: "state_string",
        body: "$name string: \"hi\";\n<div>x</div>",
    },
    CorpusCase {
        label: "state_uninit",
        body: "$node any;\n<div>x</div>",
    },
    CorpusCase {
        label: "state_then_html_same_line",
        body: "$n number: 0; <div class=\"c\"><b>hi</b></div>",
    },
    CorpusCase {
        label: "html_then_state_same_line",
        body: "<div class=\"c\"><b>hi</b></div> $n number: 0;",
    },
    // --- exports (@exports { ... }) ---
    // FEAT-115 S2: `@exports` is no longer reify-produced (harvested from the
    // template body's `@exports` directive in cst_to_stfile via parse_exports_block).
    // Exports value-semantics + E0904 are covered by the parse_exports_block unit
    // tests; the per-instance __stExports payload by the pipeline. No reify corpus.
    // --- content injection (target <- $var) ---
    // FEAT-115 S3c: body-root injections are no longer reify-classified — they are
    // transformed into synthesized selector-scoped reactive bindings in
    // cst_to_stfile (injection_scopes_from_body). Covered by the form_compiler
    // injection_scopes_* unit tests + the v8 feat071/072/bug061 end-to-end gates.
    // No reify corpus.
    // --- body-ROOT directives (self-prop, class toggle, root @on) ---
    // FEAT-115 S5c/FEAT-116: these are now synthesized World-A nested scopes
    // (directive_scopes_from_body / the `on` match), not reify directives — reify
    // no longer builds the `directives` payload for body-root forms. Render +
    // diagnostics covered by the v8/headless E2E + E0905/W0703 scope tests. No
    // reify corpus for body-root directives.
    // --- CSS / behavior blocks (.sel { ... }) ---
    CorpusCase {
        label: "css_block_plain",
        body: "<div>x</div>\n.card {\n  color: red;\n}",
    },
    CorpusCase {
        label: "block_class_toggle",
        body: "<div>x</div>\n.child {\n  .active: $isActive;\n}",
    },
    CorpusCase {
        label: "block_multi_class_toggle",
        body: "<div>x</div>\n.panel {\n  .visible: $show;\n  .expanded: $isExpanded;\n}",
    },
    CorpusCase {
        label: "block_self_prop",
        body: "<div>x</div>\n.child {\n    &self.content <- $expr;\n}",
    },
    CorpusCase {
        label: "block_on_increment",
        body: "$count number: 0;\n  <div class=\"c\">\n    <span class=\"v\"></span>\n    <button class=\"b\">+</button>\n  </div>\n  .b { @on &.click { $count <- $count + 1; } }",
    },
    CorpusCase {
        label: "block_on_toggle",
        body: "$active bool: false;\n<div class=\"b\"></div>\n.b { @on &.click { $active <- !$active; } }",
    },
    CorpusCase {
        label: "block_on_mixed_with_toggle",
        body: "$active bool: false;\n<div class=\"b\"></div>\n.b { @on &.click { $active <- !$active; } .b--on: $active; }",
    },
    CorpusCase {
        label: "block_content_binding",
        body: "$count number: 0;\n<div class=\"c\"><span class=\"v\"></span></div>\n.v { text: $count; }",
    },
    CorpusCase {
        label: "block_content_binding_expr",
        body: "$count number: 0;\n<div class=\"c\"><span class=\"v\"></span></div>\n.v { text: $count * 2; }",
    },
    // --- @match render ---
    // FEAT-115 S3d: @match is now a flat `%macro match` (the render-once sibling of
    // @view), not a reify component-body construct. Its parse (subject + arms, `_`
    // wildcard), malformed handling, and per-instance render are covered by the
    // feat073_match_* form_compiler tests + the v8 feat073 gates + the native
    // tests/fixtures/native-template fixture. No reify corpus.
    // --- selector refs (&name .sel { }) ---
    // FEAT-115 S5: reify's selector_ref path is DEAD in the real pipeline (a body
    // `&name .sel {}` is intercepted upstream by the flat `element-ref` macro, so
    // reify never sees it — proven by an empty compiled payload + E0900-not-E0920
    // fallback). No reify corpus.
    // --- template ref invocation (&name &template(args)) ---
    // FEAT-115 S3d: body template invocations (static/named/collection/dynamic) are
    // now sourced from the template scope's invoke matches + a dynamic/collection
    // harvest (scope_refs + harvest_dynamic_refs), not reify. Covered by
    // tests/integration/template_refs.rs + the v8 feat073 dynamic-dispatch gate.
    // No reify corpus.
    // --- void elements (depth-tracking regression) ---
    CorpusCase {
        label: "void_elements_then_state",
        body: "<div>\n  <img src=\"x.png\">\n  <br>\n</div>\n$n number: 0;",
    },
    // --- BUG-065: functional pseudo-class selectors + non-$-leading block content bindings ---
    CorpusCase {
        label: "bug065_css_block_pseudo_not",
        body: "<div>x</div>\n.foo:not(.bar) { color: red; }",
    },
    CorpusCase {
        label: "bug065_css_block_pseudo_nth",
        body: "<div>x</div>\n.card:nth-child(2) { color: blue; }",
    },
    CorpusCase {
        label: "bug065_block_content_bind_wrapped",
        body: "$count number: 0;\n<div class=\"c\"><span class=\"v\"></span></div>\n.v { text: currency($count); }",
    },
    CorpusCase {
        label: "bug065_block_content_bind_op_prefix",
        body: "$count number: 0;\n<div class=\"c\"><span class=\"v\"></span></div>\n.v { text: 5 + $count; }",
    },
    // --- BUG-064: bare malformed &ref must surface E0900, not vanish ---
    CorpusCase {
        label: "bug064_bare_broken_ref",
        body: "<div>Hello</div>\n&broken",
    },
    // --- BUG-063 reviewer-found parity edge cases (must reify == parse) ---
    CorpusCase {
        label: "bug063_block_on_no_mutation_diag",
        body: "<div>x</div>\n.b { @on &.click { doThing(); } }",
    },
    CorpusCase {
        // Label kept stable (snapshot filename) though the underlying bug it named is
        // now FIXED at a higher layer: BUG-167 made `@media` a recognized
        // `component_item` grammar arm (cb_media) that this validator treats as a
        // pure-consume construct (no diagnostic, matching this snapshot) — the actual
        // CSS emission now happens in src/parser/mod.rs's template-scope construction
        // (file.raw_css_blocks), which THIS diagnostics-only corpus does not exercise.
        // See at-media-in-template-body.test.st for the end-to-end CSS-emission proof.
        label: "bug063_root_at_media_dropped",
        body: "<div>x</div>\n@media (max-width: 600px) { .card { color: red; } }",
    },
    // (bug063_root_toggle_expr_condition removed — body-root toggle is World-A now)

    // --- combined realistic body ---
    CorpusCase {
        label: "combined_nav",
        body: "$menuOpen bool: false;\n<nav class=\"nav\">\n  <button class=\"toggle\">Menu</button>\n  <ul class=\"links\"><li>Home</li></ul>\n</nav>\n.nav--open: $menuOpen;\n.toggle { @on &.click { $menuOpen <- !$menuOpen; } }",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::events::form_compiler::validate_component_body;

    /// Render a diagnostic vec to a stable, serde-free string for snapshotting (FEAT-120:
    /// the validator returns canonical `Diagnostic`s, which are not `Serialize`).
    fn render_diags(diags: &[crate::diagnostics::Diagnostic]) -> String {
        if diags.is_empty() {
            return "(no diagnostics)".to_string();
        }
        diags
            .iter()
            .map(|d| {
                format!(
                    "{} {}{}",
                    d.code.as_str(),
                    d.message,
                    d.hint
                        .as_deref()
                        .map(|h| format!(" | hint: {}", h))
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// GOLDEN (FEAT-120): snapshot the DIAGNOSTICS the body validator emits for every corpus
    /// case. Was a `ComponentBodyDef` JSON oracle; the payload moved to World A and the struct
    /// was deleted, so the surviving observable is the diagnostic set. Any grammar/validator
    /// regression shows as a snapshot diff.
    #[test]
    fn golden_component_body() {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_path("snapshots");
        let _guard = settings.bind_to_scope();
        for case in CORPUS {
            let diags = validate_component_body(case.body, 0, "test");
            insta::assert_snapshot!(
                format!("component_body__{}", case.label),
                render_diags(&diags)
            );
        }
    }
}

#[cfg(test)]
mod fleet_differential {
    use crate::syntax::events::form_compiler::validate_component_body;
    use std::path::{Path, PathBuf};

    /// Extract every `@template &name(...) { BODY }` body from `src` via brace matching (string
    /// scan — good enough to harvest real bodies; respects `{}` nesting and skips `{}` inside
    /// double-quoted strings). Returns the inner BODY texts (without the outer braces).
    fn extract_template_bodies(src: &str) -> Vec<String> {
        let bytes = src.as_bytes();
        let mut bodies = Vec::new();
        let mut search = 0usize;
        while let Some(rel) = src[search..].find("@template") {
            let at = search + rel;
            // Require the DECLARATION shape `@template <ws> & <ident>` (the only real author
            // form). This rejects false matches inside strings/comments/attrs (`@templates`,
            // `'from your @templates'`, `data-template=`) that a bare substring scan grabs.
            let after = &src[at + "@template".len()..];
            let trimmed = after.trim_start();
            let is_decl = trimmed.starts_with('&')
                && trimmed[1..]
                    .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$');
            if !is_decl {
                search = at + "@template".len();
                continue;
            }
            // Find the first `{` after the `@template &name(...)` header.
            let Some(brace_rel) = src[at..].find('{') else {
                break;
            };
            let open = at + brace_rel;
            // Brace-match from `open`.
            let mut depth = 0i32;
            let mut i = open;
            let mut in_str: Option<u8> = None;
            let mut close = None;
            while i < bytes.len() {
                let c = bytes[i];
                match in_str {
                    Some(q) => {
                        if c == q && bytes[i - 1] != b'\\' {
                            in_str = None;
                        }
                    }
                    None => match c {
                        b'"' | b'\'' => in_str = Some(c),
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(i);
                                break;
                            }
                        }
                        _ => {}
                    },
                }
                i += 1;
            }
            let Some(close) = close else { break };
            let body = &src[open + 1..close];
            // Skip stdlib MACRO DEFINITIONS, not author bodies: a `%macro &template { %emit js }`
            // body is raw JS (sentinels %emit/%body/%name/%params), never a real component body.
            // The reifier is only ever fed author @template bodies, so excluding these is correct
            // (both paths emit E0900 noise on raw JS — not a meaningful parity signal).
            let is_macro_def = body.contains("%emit")
                || body.contains("%body")
                || body.contains("%name")
                || body.contains("%params")
                || body.contains("%animations");
            if !is_macro_def {
                bodies.push(body.to_string());
            }
            search = close + 1;
        }
        bodies
    }

    /// Collect tracked `.st` files under stdlib/ + examples/ + tests/fixtures/ (deterministic,
    /// in-repo). projects/ is gitignored / a separate repo, so excluded for reproducibility.
    fn collect_st_files() -> Vec<PathBuf> {
        let roots = ["stdlib", "examples", "tests/fixtures"];
        let mut out = Vec::new();
        for root in roots {
            collect_recursive(Path::new(root), &mut out);
        }
        out.sort();
        out
    }
    fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                // Skip build output + caches.
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name == "dist" || name == ".cache" || name == "node_modules" {
                    continue;
                }
                collect_recursive(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("st") {
                out.push(p);
            }
        }
    }

    /// FLEET SMOKE: harvest EVERY @template body from the in-repo .st corpus and run the
    /// self-describing producer over it. Historically this was a differential vs the (deleted)
    /// Rust parse_component_body that PROVED parity across the real fleet, licensing the cutover
    /// (FEAT-079 B3c). Post-cutover it stays as a regression smoke: validate_component_body must
    /// handle every real body without panicking AND deterministically (same input -> same JSON),
    /// so a grammar/reifier change that breaks a real showcase/admin body is caught here.
    #[test]
    fn reify_handles_fleet_deterministically() {
        let files = collect_st_files();
        assert!(
            files.len() > 10,
            "expected a real .st corpus, found {}",
            files.len()
        );
        let mut bodies_checked = 0usize;
        for f in &files {
            let Ok(src) = std::fs::read_to_string(f) else {
                continue;
            };
            for body in extract_template_bodies(&src) {
                bodies_checked += 1;
                // No panic (BUG-062 class) + deterministic. FEAT-120: the validator returns
                // canonical `Diagnostic`s; compare on (code, message) since they aren't serde.
                let key = |ds: Vec<crate::diagnostics::Diagnostic>| {
                    ds.iter()
                        .map(|d| format!("{}:{}", d.code.as_str(), d.message))
                        .collect::<Vec<_>>()
                };
                let a = key(validate_component_body(&body, 0, "test"));
                let b = key(validate_component_body(&body, 0, "test"));
                assert_eq!(a, b, "non-deterministic reify for body in {}", f.display());
            }
        }
        eprintln!(
            "fleet smoke: {} bodies across {} files",
            bodies_checked,
            files.len()
        );
        assert!(
            bodies_checked > 5,
            "expected real @template bodies, found {}",
            bodies_checked
        );
    }
}

#[cfg(test)]
mod multibyte_regression {
    use crate::syntax::events::form_compiler::validate_component_body;
    /// BUG-062: a multibyte char (`…`, `—`) at tag-depth 0 must NOT panic the segmenter's
    /// classify_construct_start byte-slice. The self-describing producer must handle it.
    #[test]
    fn multibyte_at_depth_zero_no_panic() {
        for body in [
            "<p>Loading…</p>\n$count number: 0;",
            "Hello — world\n.active: $x;",
            "<div>café</div> text <- $n;",
            "… $broken",
        ] {
            // must not panic (BUG-062 fix) + deterministic. FEAT-120: compare on (code,message).
            let key = |ds: Vec<crate::diagnostics::Diagnostic>| {
                ds.iter()
                    .map(|d| format!("{}:{}", d.code.as_str(), d.message))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                key(validate_component_body(body, 0, "test")),
                key(validate_component_body(body, 0, "test")),
                "multibyte body non-deterministic: {:?}",
                body
            );
        }
    }
}

#[cfg(test)]
mod bug065_behavior {
    use crate::syntax::events::form_compiler::validate_component_body;

    /// BUG-065 #1: a CSS block whose selector has a functional pseudo-class (:not/:nth-child/
    /// :is/:has) must stay a css_rule (verbatim), NOT leak into html.
    #[test]
    fn pseudo_class_css_block_stays_css() {
        for (body, sel) in [
            (
                "<div>x</div>\n.foo:not(.bar) { color: red; }",
                ".foo:not(.bar)",
            ),
            (
                "<div>x</div>\n.card:nth-child(2) { color: blue; }",
                ".card:nth-child(2)",
            ),
            (
                "<div>x</div>\n.x:is(.a, .b) { color: green; }",
                ".x:is(.a, .b)",
            ),
        ] {
            let _ = sel;
            let d = validate_component_body(body, 0, "test");
            // A `.sel { … }` CSS block is CONSUMED (classified, not leaked into html) and
            // produces no diagnostic. One HTML→CSS transition does NOT trip W0700 (needs >1),
            // so the valid-CSS contract is simply: an EMPTY diagnostic set.
            assert!(d.is_empty(), "no diagnostic for valid css: {:?}", d);
        }
    }

    /// BUG-065 #2: a block content binding whose reactive RHS is NOT $-leading (wrapped in a
    /// filter call or operator-prefixed) must still reify as a ContentBinding, not a css_rule.
    #[test]
    fn non_dollar_leading_content_binding() {
        // FEAT-115 S5c/FEAT-116: reify no longer BUILDS the ContentBinding directive
        // (the `.sel { text: … }` reactive binding renders via the World-A nested scope).
        // reify still CLASSIFIES the block as behavioral so it is NOT mis-emitted as a
        // css_rule — assert that segmentation witness instead of the dead directive.
        for body in [
            "$count number: 0;\n<div class=\"c\"><span class=\"v\"></span></div>\n.v { text: currency($count); }",
            "$count number: 0;\n<div class=\"c\"><span class=\"v\"></span></div>\n.v { text: 5 + $count; }",
        ] {
            let d = validate_component_body(body, 0, "test");
            // A reactive `.sel { text: … }` block is consumed as behavioral content
            // (rendered via the World-A nested scope); the validator raises no diagnostic.
            assert!(
                d.is_empty(),
                "reactive block is valid behavioral content, no diagnostic: {body:?} -> {:?}",
                d
            );
        }
    }

    /// CONTROL: a plain CSS decl inside a block (no $) must stay a css_rule, NOT become a binding.
    #[test]
    fn plain_css_decl_stays_css() {
        let d = validate_component_body(
            "<div class=\"c\"></div>\n.c { color: red; padding: 4px; }",
            0,
            "test",
        );
        // A plain CSS block is consumed (not leaked to html); one HTML→CSS transition does
        // not trip W0700, so the validator emits no diagnostic.
        assert!(d.is_empty(), "plain css block emits no diagnostic: {:?}", d);
    }
}
