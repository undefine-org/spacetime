//! Source ingress: accept EDN wherever `.st` is accepted (PLAN-148 W5).
//!
//! Normative surface: `docs/edn-spacetime/SPEC.md` §13.
//!
//! # The seam
//!
//! Every MCP path that admits source text converges on
//! `LiveServer::put_source_with_entry` (`src/mcp/live.rs:939`) — `st_fn_put`,
//! `st_tab_open`, inline `st_mount`, and browser/workbench creation all funnel
//! through it. Patching only `st_fn_put` would miss four other ingress paths.
//!
//! # Why normalise rather than branch
//!
//! [`normalize_source`] converts EDN to `.st` text BEFORE the compile step, so
//! the ingress path itself is unchanged: `compile_function` still receives `.st`
//! and the `.st` pipeline is untouched. That keeps the zero-change guarantee
//! structural — a regular `.st` user's source never takes a different branch,
//! because there is no branch to take.
//!
//! # Detection
//!
//! EDN is recognised, not declared, so no caller has to pass a `lang:` flag:
//! a source whose first non-comment, non-whitespace character is `(`, `[` or
//! `{` is EDN. No `.st` file can begin that way — `.st` starts with a directive
//! (`@`), a selector, a metasystem form (`%`), or a comment.

use crate::syntax::SyntaxRegistry;

/// Which concrete syntax a source string is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLang {
    /// Spacetime's reference syntax.
    St,
    /// EDN — a second concrete syntax over the same `%form` registry.
    Edn,
}

/// Classify a source string.
///
/// Detection is structural and cheap: skip whitespace and `//` / `/* */`
/// comments, then look at the first meaningful character.
pub fn detect(source: &str) -> SourceLang {
    let mut rest = source.trim_start();

    loop {
        if let Some(after) = rest.strip_prefix("//") {
            rest = after
                .find('\n')
                .map_or("", |i| &after[i + 1..])
                .trim_start();
            continue;
        }
        if let Some(after) = rest.strip_prefix("/*") {
            rest = after
                .find("*/")
                .map_or("", |i| &after[i + 2..])
                .trim_start();
            continue;
        }
        if let Some(after) = rest.strip_prefix(';') {
            // An EDN line comment settles it immediately.
            let _ = after;
            return SourceLang::Edn;
        }
        break;
    }

    // `(` is the only UNAMBIGUOUS opener: no `.st` construct begins with it.
    //
    // `[` and `{` are NOT safe — `[data-st-instance] { … }` is an ordinary
    // attribute-selector scope and `{` could open a bare block. Treating them as
    // EDN would route real `.st` through the EDN reader and break it, which is
    // the one failure this detection must never have. A document map or a form
    // vector therefore needs an explicit `:st/` marker to be recognised.
    match rest.chars().next() {
        Some('(') => SourceLang::Edn,
        Some('[') | Some('{') if rest.contains(":st/") => SourceLang::Edn,
        _ => SourceLang::St,
    }
}

/// Lift EDN source into an `StFile`, ready for the compiler.
///
/// # The printer is NOT on this path, deliberately
///
/// W5 first routed EDN through the printer — EDN → `FormMatch` → `.st` text →
/// parse → `FormMatch` → compile. That detour put the PRINTER's coverage on the
/// critical path, so a form the reader accepts but the printer cannot render
/// (`@stage`, whose body is a nested `Block`) became inexpressible in EDN for a
/// reason unrelated to its meaning — and failed with "could not be rendered as
/// .st", which is nonsense to an author who never wrote `.st`.
///
/// It also contradicted the thesis: `pipeline::compile` consumes
/// `Vec<FormMatch>`, and that is exactly WHY EDN cannot diverge from `.st`. So
/// EDN must reach the compiler as `FormMatch`, never as text. The printer is a
/// development instrument (`spacetime edn`, the corpus thesis test, readable
/// diffs) and nothing depends on it at runtime.
pub fn to_st_file(
    source: &str,
    registry: &SyntaxRegistry,
) -> Result<crate::parser::ast::StFile, String> {
    let matches =
        crate::edn::read_forms(source, registry).map_err(|e| format!("EDN source: {e}"))?;

    // A document's SIBLING members are part of the program, not decoration: a
    // `@template`'s body lives in a `ScopeKind::Construct` scope, and CSS
    // declarations produce no `FormMatch` at all (measured in W4: `text: $x`
    // yields zero matches). Dropping them here silently deleted every style
    // rule and every template body from an EDN-authored page.
    let mut scopes = read_scopes(source)?;
    scopes.extend(read_constructs(source).unwrap_or_default());

    // `(sel "…" form…)` members arrive from `read_forms` as flat matches
    // carrying a `selector` field. In `.st`, the rematch pass registers a
    // scope's member forms in BOTH places: flat in `file.matches` (selector
    // set — the printer and the event passes read it there) AND cloned into
    // `ScopeBlock::matches` (where the analysis passes — W0201's @each/@cycle
    // consumption, read-usages — scan). EDN must produce that same dual
    // shape: flat-only compiles but every source consumed inside a `sel` is
    // falsely flagged "defined but never used" (observed: a page whose
    // `messages` fed `(each :source $messages …)` was rejected at the compile
    // rung for a phantom W0201); scoped-only prints as a page with no
    // behaviour at all.
    let mut scoped: std::collections::BTreeMap<String, Vec<crate::syntax::FormMatch>> =
        Default::default();
    for fm in &matches {
        if let Some(sel) = &fm.selector {
            scoped.entry(sel.clone()).or_default().push(fm.clone());
        }
    }
    for (sel, fms) in scoped {
        match scopes.iter_mut().find(|s| {
            s.selector == sel && matches!(s.kind, crate::parser::ast::ScopeKind::Selector)
        }) {
            Some(block) => block.matches.extend(fms),
            None => scopes.push(crate::parser::ast::ScopeBlock {
                selector: sel,
                matches: fms,
                ..Default::default()
            }),
        }
    }

    // Every plane `write_document` EMITS must be read back, or a document that
    // survived the round trip on paper silently compiles as a different
    // program. Accepting `:st/imports` while dropping it was exactly that
    // (found by review): the file parsed, the imports vanished, and the failure
    // surfaced later as an unresolved macro.
    let imports = read_imports(source).unwrap_or_default();
    let html_blocks = read_html(source).unwrap_or_default();
    let raw_css_blocks = read_raw_css(source).unwrap_or_default();

    Ok(crate::parser::ast::StFile {
        matches,
        scopes,
        imports,
        html_blocks,
        raw_css_blocks,
        ..Default::default()
    })
}

/// One `:st/…` member of a document, if the source is a document map.
fn document_member<'a>(
    root: &'a clojure_reader::edn::Edn<'a>,
    key: &str,
) -> Option<&'a clojure_reader::edn::Edn<'a>> {
    use clojure_reader::edn::Edn;
    let Edn::Vector(items) = root else {
        return None;
    };
    for item in items {
        let Edn::Map(members) = item else { continue };
        if let Some(v) = members.iter().find_map(|(k, v)| match k {
            Edn::Key(name) if *name == key => Some(v),
            _ => None,
        }) {
            return Some(v);
        }
    }
    None
}

fn parse_document(source: &str) -> Option<clojure_reader::edn::Edn<'_>> {
    // Leaked deliberately: `Edn` borrows the source, and every caller wants the
    // parsed tree for the life of one ingress call.
    let wrapped: &'static str = Box::leak(format!("[{source}]").into_boxed_str());
    clojure_reader::edn::read_string(wrapped).ok()
}

/// `:st/imports` as parser imports.
fn read_imports(source: &str) -> Option<Vec<crate::parser::ast::ImportAst>> {
    use clojure_reader::edn::Edn;
    let root = parse_document(source)?;
    let member = document_member(&root, "st/imports")?;
    let (Edn::Vector(list) | Edn::List(list)) = member else {
        return None;
    };
    Some(
        list.iter()
            .filter_map(|i| match i {
                Edn::Str(p) => Some(crate::parser::ast::ImportAst {
                    path: (*p).to_string(),
                    namespace: None,
                    alias: None,
                    only: Vec::new(),
                    hiding: Vec::new(),
                    span: Default::default(),
                }),
                _ => None,
            })
            .collect(),
    )
}

/// `:st/html` as parser html blocks, converting `[:st/hole N]` markers back to
/// the private-use sentinels the pipeline expects.
fn read_html(source: &str) -> Option<Vec<crate::parser::ast::HtmlBlockAst>> {
    use crate::parser::ast::{HtmlBlockAst, HtmlInjection};
    use clojure_reader::edn::Edn;
    let root = parse_document(source)?;
    let member = document_member(&root, "st/html")?;
    let (Edn::Vector(list) | Edn::List(list)) = member else {
        return None;
    };

    let mut out = Vec::new();
    for b in list {
        // Inline string injection nodes. `st/raw` is deliberately opaque;
        // `st/html` is a reactive HTML string; `st/md` is a reactive MARKDOWN
        // expression (its value is rendered by snarkdown). All three carry the
        // payload string in the same shape; the tag picks the injection mode.
        if let Edn::Vector(parts) | Edn::List(parts) = b
            && let [Edn::Key(tag), Edn::Str(src)] = parts.as_slice()
            && (*tag == "st/raw" || *tag == "st/html" || *tag == "st/md")
        {
            out.push(HtmlBlockAst {
                skeleton: crate::edn::codec::unescape_edn(src),
                injection: match *tag {
                    "st/html" => HtmlInjection::Reactive,
                    "st/md" => HtmlInjection::Markdown,
                    _ => HtmlInjection::Raw,
                },
                ..Default::default()
            });
            continue;
        }

        let Edn::Map(fields) = b else { continue };
        let mut block = HtmlBlockAst::default();
        for (k, v) in fields.iter() {
            match (k, v) {
                (Edn::Key(n), Edn::Str(sk)) if *n == "skeleton" => {
                    block.skeleton = crate::edn::doc::parse_hole_markers(sk);
                }
                (Edn::Key(n), Edn::Vector(hs) | Edn::List(hs)) if *n == "holes" => {
                    for h in hs {
                        if let Edn::Vector(parts) = h {
                            if let [Edn::Key(tag), Edn::Str(e)] = parts.as_slice() {
                                if *tag == "st/expr" {
                                    block.holes.push((*e).to_string());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if !block.skeleton.is_empty() {
            out.push(block);
        }
    }
    Some(out)
}

/// `:st/css` as raw CSS blocks.
fn read_raw_css(source: &str) -> Option<Vec<crate::parser::ast::RawCssBlock>> {
    use clojure_reader::edn::Edn;
    let root = parse_document(source)?;
    let member = document_member(&root, "st/css")?;
    let (Edn::Vector(list) | Edn::List(list)) = member else {
        return None;
    };
    Some(
        list.iter()
            .filter_map(|c| match c {
                Edn::Vector(parts) => match parts.as_slice() {
                    [Edn::Key(tag), Edn::Str(src)] if *tag == "st/raw" => {
                        // `Edn::Str` is the raw, still-escaped slice — every
                        // other consumer decodes through `unescape_edn`
                        // (codec.rs), and CSS is no different: an authored
                        // `content: "\"ok\""` must not land in the stylesheet
                        // with its backslashes still on.
                        Some(crate::parser::ast::RawCssBlock {
                            source: crate::edn::codec::unescape_edn(src),
                            span: Default::default(),
                        })
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect(),
    )
}

/// The `:st/constructs` member, as `ScopeKind::Construct` scopes.
///
/// A construct body (`@template &card($c) { <article>…</article> }`) parks its
/// markup in a sibling scope; `print_file` reads it back from there. Carrying
/// the member is what makes an EDN-authored template renderable at all.
fn read_constructs(source: &str) -> Option<Vec<crate::parser::ast::ScopeBlock>> {
    use crate::parser::ast::ScopeKind;
    use clojure_reader::edn::Edn;

    let wrapped = format!("[{source}]");
    let root = clojure_reader::edn::read_string(&wrapped).ok()?;
    let Edn::Vector(items) = root else {
        return None;
    };

    let mut out = Vec::new();
    for item in &items {
        let Edn::Map(members) = item else { continue };
        let Some(list) = members.iter().find_map(|(k, v)| match k {
            Edn::Key(name) if *name == "st/constructs" => Some(v),
            _ => None,
        }) else {
            continue;
        };
        let (Edn::Vector(cs) | Edn::List(cs)) = list else {
            continue;
        };

        for c in cs {
            let Edn::Map(fields) = c else { continue };
            let mut block = crate::parser::ast::ScopeBlock::default();
            let mut kind = String::new();

            for (k, v) in fields.iter() {
                match (k, v) {
                    (Edn::Key(n), Edn::Str(val)) if *n == "kind" => kind = (*val).to_string(),
                    (Edn::Key(n), Edn::Str(val)) if *n == "selector" => {
                        block.selector = crate::edn::codec::unescape_edn(val);
                    }
                    // UNESCAPED, like every other string that leaves the EDN
                    // plane (BUG-377 decoded the expr path the same way).
                    //
                    // A construct body is the ONE string that travels edn → ast
                    // → `.st` SOURCE, so a missed decode is not a wrong value in
                    // a field: it is a backslash written into the program text.
                    // `data-role=\"`$m.role`\"` printed literally, the HTML
                    // tokenizer read `\` as the start of an unquoted attribute
                    // value, and the template runtime's attribute pass — which
                    // matches a double-quoted value — never fired. The hole
                    // reached the DOM as the text `$m.role`, so every
                    // `[data-role='model']` rule missed and the console said
                    // only `Uninterpolated variables: ["$m.role"]`.
                    //
                    // Single-quoted bodies hid this: they carry no escape, so
                    // the raw and decoded forms are identical and the missing
                    // call was invisible until a document used the quoting the
                    // runtime actually requires.
                    (Edn::Key(n), Edn::Str(val)) if *n == "html" => {
                        block.html = crate::edn::codec::unescape_edn(val);
                    }
                    (Edn::Key(n), Edn::Map(decls)) if *n == "decls" => {
                        for (dk, dv) in decls.iter() {
                            let prop = match dk {
                                Edn::Str(p) => (*p).to_string(),
                                Edn::Key(p) => (*p).to_string(),
                                _ => continue,
                            };
                            let (value, is_injection) = match dv {
                                Edn::Str(val) => ((*val).to_string(), false),
                                Edn::Vector(parts) => match parts.as_slice() {
                                    [Edn::Key(tag), Edn::Str(val)] if *tag == "st/inject" => {
                                        ((*val).to_string(), true)
                                    }
                                    _ => continue,
                                },
                                _ => continue,
                            };
                            block
                                .css_declarations
                                .push(crate::parser::ast::CssDeclaration {
                                    property: prop,
                                    value,
                                    is_injection,
                                    span: Default::default(),
                                });
                        }
                    }
                    _ => {}
                }
            }

            if !kind.is_empty() && !block.selector.is_empty() {
                block.kind = ScopeKind::Construct(kind);
                out.push(block);
            }
        }
    }

    Some(out)
}

/// The `:st/scopes` member of a document, as parser scopes.
///
/// Returns None when the source is not a document map (a bare form sequence has
/// no scopes), so the caller falls back to an empty list rather than failing —
/// a form-only document is legal.
fn read_scopes(source: &str) -> Result<Vec<crate::parser::ast::ScopeBlock>, String> {
    use clojure_reader::edn::Edn;

    let wrapped = format!("[{source}]");
    let Ok(root) = clojure_reader::edn::read_string(&wrapped) else {
        return Ok(Vec::new());
    };
    let Edn::Vector(items) = root else {
        return Ok(Vec::new());
    };

    let mut malformed: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for item in &items {
        let Edn::Map(members) = item else { continue };
        let Some(scopes) = members.iter().find_map(|(k, v)| match k {
            Edn::Key(name) if *name == "st/scopes" => Some(v),
            _ => None,
        }) else {
            continue;
        };
        let (Edn::Vector(list) | Edn::List(list)) = scopes else {
            continue;
        };

        for s in list {
            let Edn::Map(fields) = s else { continue };
            let mut block = crate::parser::ast::ScopeBlock::default();

            for (k, v) in fields.iter() {
                match (k, v) {
                    (Edn::Key(n), Edn::Str(sel)) if *n == "selector" => {
                        block.selector = (*sel).to_string();
                    }
                    (Edn::Key(n), Edn::Map(decls)) if *n == "decls" => {
                        for (dk, dv) in decls.iter() {
                            let prop = match dk {
                                Edn::Str(p) => (*p).to_string(),
                                Edn::Key(p) => (*p).to_string(),
                                _ => continue,
                            };
                            // `[:st/inject "…"]` is the `<-` arrow; a bare string
                            // is the `:` CSS surface. Collapsing the two would
                            // reroute a reactive binding (BUG-091).
                            let (value, is_injection) = match dv {
                                Edn::Str(val) => ((*val).to_string(), false),
                                Edn::Vector(parts) => match parts.as_slice() {
                                    [Edn::Key(tag), Edn::Str(val)] if *tag == "st/inject" => {
                                        ((*val).to_string(), true)
                                    }
                                    _ => continue,
                                },
                                _ => continue,
                            };
                            block
                                .css_declarations
                                .push(crate::parser::ast::CssDeclaration {
                                    property: prop,
                                    value,
                                    is_injection,
                                    span: Default::default(),
                                });
                        }
                    }
                    _ => {}
                }
            }

            // A scope with no selector is a TYPO, not an empty scope:
            // `{:selectr ".x" :decls {…}}` silently lost the whole rule, so a
            // misspelled key read as "my styles vanished". Collect it as an
            // error the caller reports.
            if block.selector.is_empty() {
                malformed.push(format!("{s:?}"));
                continue;
            }
            out.push(block);
        }
    }

    if !malformed.is_empty() {
        return Err(format!(
            "EDN source: {} scope entry/entries carry no `:selector` — a typo'd key \
             would otherwise drop the rule silently: {}",
            malformed.len(),
            malformed.join(", ")
        ));
    }

    Ok(out)
}

/// Normalise `.st` source, or report that a source is EDN.
///
/// `.st` passes through byte-identical — the identity path is literally a move,
/// so an ordinary user cannot be affected by this function existing. EDN returns
/// `None`: it has no `.st` text form on the ingress path, by design (see
/// [`to_st_file`]).
pub fn normalize_source(source: &str, registry: &SyntaxRegistry) -> Result<String, String> {
    match detect(source) {
        SourceLang::St => Ok(source.to_string()),
        SourceLang::Edn => {
            // Kept for the CLI (`spacetime st <f.edn>`), where rendering to `.st`
            // IS the point. Ingress uses `to_st_file` and never comes here.
            //
            // Renders through the WHOLE-DOCUMENT path (BUG-370). `print_forms`
            // sees matches only, so a `@template` — whose body is a
            // ComponentBody marker pointing at a sibling scope — refused to
            // print at all: "its payload lives in the sibling
            // @template:<name> scope". `to_st_file` + `print_file` carry those
            // siblings, so the CLI can render any document the reader accepts.
            let file = to_st_file(source, registry)?;
            crate::edn::print_file(&file, registry)
                .map_err(|e| format!("EDN source could not be rendered as .st: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::STDLIB_REGISTRY;

    #[test]
    fn detects_st_sources() {
        assert_eq!(detect(r#"@data inline $x : 0;"#), SourceLang::St);
        assert_eq!(detect(".card { color: red; }"), SourceLang::St);
        assert_eq!(detect("%macro foo { }"), SourceLang::St);
        assert_eq!(detect("// a comment\n@mcp-input;"), SourceLang::St);
        assert_eq!(detect("/* block */\n.a { }"), SourceLang::St);
        assert_eq!(detect(""), SourceLang::St);
    }

    #[test]
    fn detects_edn_sources() {
        assert_eq!(detect("(data-inline $x [:st/expr \"0\"])"), SourceLang::Edn);
        assert_eq!(detect("[:st/forms (mcp-input)]"), SourceLang::Edn);
        assert_eq!(detect("{:st/forms []}"), SourceLang::Edn);
        // An attribute-selector scope is `.st`, NOT an EDN vector.
        assert_eq!(detect("[data-st-instance] { }"), SourceLang::St);
        assert_eq!(detect("\n\n  (mcp-input)"), SourceLang::Edn);
    }

    #[test]
    fn st_passes_through_byte_identical() {
        // The zero-change guarantee, as an assertion: a `.st` source is returned
        // EXACTLY as given, so no `.st` user can be affected by EDN support.
        let src = "@data inline $x : 0;\n.card { color: red; }\n";
        assert_eq!(normalize_source(src, &STDLIB_REGISTRY).unwrap(), src);
    }

    #[test]
    fn edn_normalises_to_parseable_st() {
        let st = normalize_source(
            r#"(data-inline :name $x :value [:st/expr "0"])"#,
            &STDLIB_REGISTRY,
        )
        .expect("normalise");

        assert!(st.contains("@data"), "{st}");
        assert!(st.contains("inline"), "{st}");
        // The real bar: the output must be genuine `.st`, not merely text.
        let ast = crate::parse(&st).unwrap_or_else(|e| panic!("not valid .st: {st}\n{e:?}"));
        assert_eq!(ast.matches.len(), 1);
        assert_eq!(ast.matches[0].matched_macro.as_deref(), Some("data-inline"));
    }

    #[test]
    fn edn_and_st_ingress_reach_the_same_program() {
        // W5's proof obligation in miniature: the two ingress spellings converge
        // on the same matches, so they compile to the same bundle.
        let from_st = crate::parse(r#"@data inline $x : 0;"#).unwrap();
        let normalised = normalize_source(
            r#"(data-inline :name $x :value [:st/expr "0"])"#,
            &STDLIB_REGISTRY,
        )
        .unwrap();
        let from_edn = crate::parse(&normalised).unwrap();

        assert_eq!(from_st.matches.len(), from_edn.matches.len());
        assert_eq!(
            from_st.matches[0].matched_macro,
            from_edn.matches[0].matched_macro
        );
        assert_eq!(
            from_st.matches[0].captures.get("name"),
            from_edn.matches[0].captures.get("name")
        );
    }

    #[test]
    fn broken_edn_fails_with_a_usable_message() {
        let err = normalize_source("(no-such-macro)", &STDLIB_REGISTRY).unwrap_err();
        assert!(err.contains("EDN source"), "{err}");
        assert!(err.contains("unknown macro"), "{err}");
    }

    // A construct body is the one string that travels edn → ast → `.st` SOURCE,
    // so a missed unescape is a backslash written into the program text rather
    // than a wrong value in a field.
    //
    // `data-role=\"…\"` printed literally as `data-role=\"…\"`. The HTML
    // tokenizer then read `\` as the start of an unquoted attribute value, and
    // the template runtime's attribute pass — which matches a DOUBLE-quoted
    // value — never fired, so the hole reached the DOM as literal text.
    //
    // Single quotes carry no escape, which is why every existing fixture
    // passed while this was broken: raw and decoded were identical.
    #[test]
    fn a_double_quoted_attribute_in_a_construct_body_survives_the_round_trip() {
        let edn = r#"{:st/imports []
 :st/forms [(template :name b :params [:st/paramlist ["m" :binding false nil nil false]] :body [:st/component-body])]
 :st/constructs [{:kind "template" :selector "@template:b" :html "<p data-r=\"`$m.r`\">x</p>" :decls {}}]}"#;

        let st = normalize_source(edn, &STDLIB_REGISTRY).unwrap();

        assert!(
            st.contains(r#"data-r="`$m.r`""#),
            "the attribute lost its quoting: {st}"
        );
        assert!(
            !st.contains(r#"data-r=\""#),
            "an EDN escape was written into the .st source: {st}"
        );
    }

    // The decoder must not corrupt text it does not understand: a body holding
    // a regex or a Windows path keeps its backslash.
    #[test]
    fn an_unknown_escape_in_a_construct_body_is_carried_through() {
        let edn = r#"{:st/imports []
 :st/forms [(template :name b :params [:st/paramlist ] :body [:st/component-body])]
 :st/constructs [{:kind "template" :selector "@template:b" :html "<p data-re=\"\\d+\">x</p>" :decls {}}]}"#;

        let st = normalize_source(edn, &STDLIB_REGISTRY).unwrap();

        assert!(st.contains(r"\d+"), "a regex escape was eaten: {st}");
    }
}
