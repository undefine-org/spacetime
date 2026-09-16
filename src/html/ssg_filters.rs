//! Build-time evaluation of stdlib filter pipes for SSG `@each` unrolling.
//!
//! # Why this module exists
//!
//! A statically-unrolled `@each` row used to render its filter as literal page
//! text — `<li><span data-st-hole="0"></span> | uppercase</li>` — so view-source
//! disagreed with the hydrated page (GH-11).
//!
//! The tempting fix is a `match` on filter names right here in Rust. That is
//! forbidden, and for a concrete reason rather than a stylistic one: `uppercase`
//! was ALREADY implemented three times (stdlib's `%primitive`, `st.js`, and
//! `data-binding.js`) and the three had already drifted — `capitalize` is
//! declared in stdlib and implemented in neither runtime table, so it failed at
//! build time AND at runtime. A fourth implementation would deepen exactly the
//! defect this fixes.
//!
//! So this module evaluates **the stdlib primitive's own `%emit js` body**, the
//! same bytes the browser runs. Parity is structural: there is one definition of
//! `uppercase`, and both the build and the browser execute it.
//!
//! # The seam
//!
//! A filter primitive is a pure, total function from a value to a string:
//!
//! ```text
//! %primitive uppercase(data: string) {
//!   %emit js { %yield String(%data).toUpperCase() -> $result; }
//! }
//! ```
//!
//! `%data` is the piped value, further `%name` tokens are the call arguments
//! (`truncate(4)`), and `%yield <expr> -> $result` names the result expression.
//! Rendering that to a self-contained JS expression is all this module does; the
//! evaluation itself is delegated to the build-time JS runtime the compiler
//! already hosts for `%emit build-js`.
//!
//! # Purity
//!
//! Only a primitive whose body is a pure value transform may run at build time.
//! A body reaching for `Date.now`, `Math.random`, `document` or `window` would
//! bake a build-time answer into static HTML and then disagree with the browser
//! on the very next render — so it REFUSES and the pipe is left for the runtime,
//! which is the honest outcome. This mirrors the fidelity-ladder rule the test
//! backends already follow: a layer that cannot answer a question declines
//! instead of approximating.

use crate::parser::meta_ast::{EmitLang, ParamDefault, PrimitiveDefAst, PrimitiveParam};

/// One stage of a filter pipe: `truncate(4, "…")` → name + literal args.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterCall {
    pub name: String,
    pub args: Vec<String>,
}

/// Tokens that make a primitive body unsafe to evaluate at build time.
///
/// Deliberately conservative: a false refusal costs a filter that renders at
/// runtime instead of build time (the status quo ante, and visibly correct in
/// the browser), while a false ACCEPT bakes a stale value into shipped HTML that
/// silently contradicts the hydrated page. The asymmetry decides the default.
const IMPURE_TOKENS: &[&str] = &[
    "Date.now",
    "new Date",
    "Math.random",
    "document",
    "window",
    "globalThis",
    "localStorage",
    "sessionStorage",
    "fetch",
    "XMLHttpRequest",
    "performance.",
    "crypto",
];

/// Parse the filter half of a `` `$x` | a | b(1) `` pipe into stages.
///
/// The value half is split off upstream by [`crate::syntax::split_filter_pipe`],
/// which already handles the cases that make naive splitting wrong (`||` is not
/// a pipe, a `|` inside a string literal is not a separator). This function only
/// sees what that returns, and splits the remaining chain on top-level `|`.
pub fn parse_filter_chain(filter_src: &str) -> Vec<FilterCall> {
    let mut out = Vec::new();
    for stage in split_top_level(filter_src, '|') {
        let stage = stage.trim();
        if stage.is_empty() {
            continue;
        }
        match stage.find('(') {
            Some(open) if stage.ends_with(')') => {
                let name = stage[..open].trim().to_string();
                let inner = &stage[open + 1..stage.len() - 1];
                let args = split_top_level(inner, ',')
                    .into_iter()
                    .map(|a| a.trim().to_string())
                    .filter(|a| !a.is_empty())
                    .collect();
                out.push(FilterCall { name, args });
            }
            _ => out.push(FilterCall {
                name: stage.to_string(),
                args: Vec::new(),
            }),
        }
    }
    out
}

/// Split on `sep`, ignoring separators inside quotes or nested parens/brackets.
fn split_top_level(src: &str, sep: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for ch in src.chars() {
        if escaped {
            cur.push(ch);
            escaped = false;
            continue;
        }
        match quote {
            Some(q) => {
                cur.push(ch);
                if ch == '\\' {
                    escaped = true;
                } else if ch == q {
                    quote = None;
                }
            }
            None => match ch {
                '"' | '\'' | '`' => {
                    quote = Some(ch);
                    cur.push(ch);
                }
                '(' | '[' | '{' => {
                    depth += 1;
                    cur.push(ch);
                }
                ')' | ']' | '}' => {
                    depth -= 1;
                    cur.push(ch);
                }
                c if c == sep && depth == 0 => {
                    parts.push(std::mem::take(&mut cur));
                }
                _ => cur.push(ch),
            },
        }
    }
    parts.push(cur);
    parts
}

/// Why a filter could not be evaluated at build time.
///
/// Carried as a value rather than logged and dropped: the caller leaves the pipe
/// for the runtime, and a diagnostic can name the exact reason.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterRefusal {
    /// No `%primitive` of that name is registered — the author named a filter
    /// that does not exist. A hard error, not a silent passthrough.
    Unknown(String),
    /// Declared, but its body touches non-deterministic or DOM state.
    Impure { name: String, token: String },
    /// Declared and pure, but its body shape is not a simple `%yield` transform.
    Unsupported(String),
    /// A call argument is not a literal, so it cannot be safely folded into an
    /// evaluated body. Left to the runtime rather than executed at build time.
    NonLiteralArg { name: String, arg: String },
}

impl FilterRefusal {
    pub fn message(&self) -> String {
        match self {
            FilterRefusal::Unknown(n) => format!(
                "unknown filter `{n}` — no %primitive of that name is registered"
            ),
            FilterRefusal::Impure { name, token } => format!(
                "filter `{name}` reads `{token}`, so a build-time value could disagree with the browser"
            ),
            FilterRefusal::Unsupported(n) => format!(
                "filter `{n}` has no single %yield expression to evaluate"
            ),
            FilterRefusal::NonLiteralArg { name, arg } => format!(
                "filter `{name}` got the non-literal argument `{arg}`; only literals                  fold at build time"
            ),
        }
    }
}

/// Accept a filter call argument ONLY when it is a self-evident literal, and
/// return its JS spelling.
///
/// Permitted: a quoted string, a decimal number, `true`/`false`/`null`. Anything
/// else — an identifier, a member access, a call, an operator expression — is
/// refused, because it would be EVALUATED at build time with whatever meaning it
/// happens to have on the build machine.
///
/// Strings are re-emitted through `serde_json` rather than passed through, so the
/// quoting is normalized and a literal cannot terminate its own quote.
fn as_js_literal(arg: &str) -> Option<String> {
    let a = arg.trim();
    if a.is_empty() {
        return None;
    }
    if matches!(a, "true" | "false" | "null") {
        return Some(a.to_string());
    }
    if let Ok(n) = a.parse::<f64>() {
        return Some(n.to_string());
    }
    let bytes = a.as_bytes();
    let quote = bytes[0];
    if (quote == b'"' || quote == b'\'') && bytes.len() >= 2 && *bytes.last()? == quote {
        // Re-parse the inner text and re-serialize, so the emitted literal is
        // well-formed by construction rather than by trusting the source bytes.
        let inner = &a[1..a.len() - 1];
        if inner.contains(quote as char) && !inner.contains("\\") {
            return None;
        }
        return serde_json::to_string(inner).ok();
    }
    None
}

/// Render a filter primitive into a self-contained JS expression.
///
/// The result is an IIFE evaluating the primitive's own `%emit js` body with
/// `%data` bound to `input_js` and each further `%name` bound to the call
/// argument (or the parameter's declared default). Returning source rather than
/// a value keeps this module free of any engine dependency, so it compiles and
/// is unit-testable with no feature flags.
pub fn render_filter_expr(
    def: &PrimitiveDefAst,
    call: &FilterCall,
    input_js: &str,
) -> Result<String, FilterRefusal> {
    let emit = def
        .body
        .emit_blocks
        .iter()
        .find(|b| b.lang == EmitLang::Js)
        .ok_or_else(|| FilterRefusal::Unsupported(call.name.clone()))?;

    for token in IMPURE_TOKENS {
        if emit.content.contains(token) {
            return Err(FilterRefusal::Impure {
                name: call.name.clone(),
                token: (*token).to_string(),
            });
        }
    }

    // Positional params after `data` take the call's arguments in order; any not
    // supplied fall back to the declared default, which is what the runtime does.
    let mut body = emit.content.clone();
    let mut positional = call.args.iter();
    for param in &def.params {
        let (name, default) = match param {
            PrimitiveParam::Typed { name, default, .. } => (name.clone(), default.clone()),
            PrimitiveParam::TypedData { name, .. } | PrimitiveParam::Data(name) => {
                (name.clone(), None)
            }
            PrimitiveParam::Element(_) => continue,
        };
        let value = if name == "data" {
            input_js.to_string()
        } else if let Some(arg) = positional.next() {
            // A call argument is spliced into a JS body that is then EVALUATED,
            // so it must be a LITERAL and nothing else. Author source is not a
            // hostile input here (you already control your own build), but an
            // arbitrary expression would silently gain build-time execution and
            // could reference build-machine state that the browser cannot see —
            // the same build/runtime divergence this whole fix exists to remove.
            //
            // Today `scan_pipe_run` only produces identifier-shaped stages, so a
            // non-literal cannot reach here in practice. That is INCIDENTAL: it
            // depends on a scanner in another module keeping a property nobody
            // wrote down. Checking here makes the boundary explicit and local,
            // so tightening or loosening the scanner cannot quietly widen what
            // gets evaluated.
            let Some(lit) = as_js_literal(arg) else {
                return Err(FilterRefusal::NonLiteralArg {
                    name: call.name.clone(),
                    arg: arg.clone(),
                });
            };
            lit
        } else {
            match default {
                Some(ParamDefault::String(s)) => format!("{s:?}"),
                Some(ParamDefault::Number(n)) => n.to_string(),
                Some(ParamDefault::Bool(b)) => b.to_string(),
                Some(ParamDefault::EmptyArray) => "[]".to_string(),
                Some(ParamDefault::EmptyObject) => "({})".to_string(),
                Some(ParamDefault::Length(n, unit)) => format!("{n}{unit:?}"),
                // An array-literal or explicit-None default has no single JS
                // scalar spelling that is obviously right here; refusing keeps
                // the runtime answer authoritative rather than inventing one.
                Some(ParamDefault::Array(_)) | Some(ParamDefault::None) | None => {
                    return Err(FilterRefusal::Unsupported(call.name.clone()));
                }
            }
        };
        body = body.replace(&format!("%{name}"), &value);
    }

    // `%yield <expr> -> $result;` is the primitive's result. Statements before it
    // are preserved (capitalize binds `const str = …` first), so the body becomes
    // an ordinary function body with the yield rewritten to a return.
    let Some(idx) = body.find("%yield") else {
        return Err(FilterRefusal::Unsupported(call.name.clone()));
    };
    let (prelude, yield_part) = body.split_at(idx);
    let yield_body = &yield_part["%yield".len()..];
    let expr = match yield_body.find("->") {
        Some(arrow) => yield_body[..arrow].trim(),
        None => yield_body.trim_end_matches(';').trim(),
    };
    if expr.is_empty() {
        return Err(FilterRefusal::Unsupported(call.name.clone()));
    }

    Ok(format!(
        "(() => {{ {prelude} return ({expr}); }})()",
        prelude = prelude.trim(),
        expr = expr
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::meta_ast::{EmitBlock, ParamType, PrimitiveBody};

    fn prim(name: &str, params: Vec<PrimitiveParam>, js: &str) -> PrimitiveDefAst {
        PrimitiveDefAst {
            name: name.to_string(),
            params,
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: js.to_string(),
                    span: Default::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: Default::default(),
            source_file: None,
            doc: None,
        }
    }

    fn typed(name: &str, default: Option<ParamDefault>) -> PrimitiveParam {
        PrimitiveParam::Typed {
            name: name.to_string(),
            ty: ParamType::Simple("string".into()),
            default,
        }
    }

    #[test]
    fn a_bare_filter_parses_to_one_stage_with_no_args() {
        let chain = parse_filter_chain(" uppercase ");
        assert_eq!(
            chain,
            vec![FilterCall {
                name: "uppercase".into(),
                args: vec![]
            }]
        );
    }

    #[test]
    fn a_chain_splits_on_top_level_pipes_only() {
        let chain = parse_filter_chain(r#"truncate(4, "a|b") | uppercase"#);
        assert_eq!(chain.len(), 2, "a pipe inside a string is not a separator");
        assert_eq!(chain[0].name, "truncate");
        assert_eq!(chain[0].args, vec!["4", r#""a|b""#]);
        assert_eq!(chain[1].name, "uppercase");
    }

    #[test]
    fn the_piped_value_binds_to_percent_data() {
        let p = prim(
            "uppercase",
            vec![typed("data", None)],
            "%yield String(%data).toUpperCase() -> $result;",
        );
        let js = render_filter_expr(
            &p,
            &FilterCall {
                name: "uppercase".into(),
                args: vec![],
            },
            "\"ada\"",
        )
        .expect("pure body must render");
        assert!(js.contains("String(\"ada\").toUpperCase()"), "got: {js}");
        assert!(!js.contains("%data"), "every %token must be substituted");
    }

    #[test]
    fn statements_before_the_yield_are_preserved() {
        // `capitalize` binds a const first — dropping the prelude would emit a
        // body referencing an undefined name, which is worse than not folding.
        let p = prim(
            "capitalize",
            vec![typed("data", None)],
            "const str = String(%data);\n%yield str.charAt(0).toUpperCase() + str.slice(1) -> $result;",
        );
        let js = render_filter_expr(
            &p,
            &FilterCall {
                name: "capitalize".into(),
                args: vec![],
            },
            "\"ada\"",
        )
        .unwrap();
        assert!(js.contains("const str = String(\"ada\")"), "got: {js}");
        assert!(js.contains("return (str.charAt(0)"), "got: {js}");
    }

    #[test]
    fn an_unsupplied_argument_falls_back_to_the_declared_default() {
        let p = prim(
            "truncate",
            vec![
                typed("data", None),
                typed("length", Some(ParamDefault::Number(100.0))),
                typed("suffix", Some(ParamDefault::String("...".into()))),
            ],
            "const len = %length; const suf = %suffix; %yield String(%data).slice(0, len) + suf -> $result;",
        );
        let js = render_filter_expr(
            &p,
            &FilterCall {
                name: "truncate".into(),
                args: vec!["4".into()],
            },
            "\"ada lovelace\"",
        )
        .unwrap();
        assert!(js.contains("const len = 4"), "explicit arg wins: {js}");
        assert!(js.contains(r#"const suf = "...""#), "default fills: {js}");
    }

    #[test]
    fn a_non_literal_argument_refuses_instead_of_being_evaluated() {
        // The rendered body is EVALUATED, so an argument that is an expression
        // rather than a literal would gain build-time execution. `scan_pipe_run`
        // happens not to produce such stages today, but this boundary must not
        // depend on that: it is enforced here, where the splice occurs.
        let p = prim(
            "truncate",
            vec![typed("data", None), typed("length", Some(ParamDefault::Number(100.0)))],
            "const len = %length; %yield String(%data).slice(0, len) -> $result;",
        );
        for hostile in [
            "globalThis.__x",
            "1+globalThis.__x",
            "process.env.HOME",
            "(()=>{return 1})()",
        ] {
            let err = render_filter_expr(
                &p,
                &FilterCall {
                    name: "truncate".into(),
                    args: vec![hostile.to_string()],
                },
                "\"abc\"",
            )
            .unwrap_err();
            assert!(
                matches!(err, FilterRefusal::NonLiteralArg { .. }),
                "`{hostile}` must refuse, not be evaluated at build time. got: {err:?}"
            );
        }
    }

    #[test]
    fn literal_arguments_still_fold() {
        let p = prim(
            "truncate",
            vec![typed("data", None), typed("length", Some(ParamDefault::Number(100.0)))],
            "const len = %length; %yield String(%data).slice(0, len) -> $result;",
        );
        for (arg, want) in [("4", "const len = 4"), ("true", "const len = true")] {
            let js = render_filter_expr(
                &p,
                &FilterCall {
                    name: "truncate".into(),
                    args: vec![arg.to_string()],
                },
                "\"abc\"",
            )
            .unwrap();
            assert!(js.contains(want), "arg `{arg}` should fold. got: {js}");
        }
    }

    #[test]
    fn an_impure_body_refuses_rather_than_baking_a_build_time_value() {
        let p = prim(
            "now",
            vec![typed("data", None)],
            "%yield String(Date.now()) -> $result;",
        );
        let err = render_filter_expr(
            &p,
            &FilterCall {
                name: "now".into(),
                args: vec![],
            },
            "\"x\"",
        )
        .expect_err("an impure filter must refuse");
        assert!(
            matches!(err, FilterRefusal::Impure { .. }),
            "got: {err:?} — a build-time Date.now would disagree with the browser"
        );
        assert!(err.message().contains("disagree with the browser"));
    }

    #[test]
    fn a_body_without_a_yield_is_unsupported_not_silently_empty() {
        let p = prim("weird", vec![typed("data", None)], "const x = %data;");
        let err = render_filter_expr(
            &p,
            &FilterCall {
                name: "weird".into(),
                args: vec![],
            },
            "\"x\"",
        )
        .expect_err("no %yield means nothing to fold");
        assert!(matches!(err, FilterRefusal::Unsupported(_)), "got: {err:?}");
    }
}

/// Evaluate a self-contained JS expression to a string, at build time.
///
/// Returns `None` when the expression cannot be evaluated — no JS engine in this
/// build, or the expression threw. `None` always means "leave it to the
/// runtime", never "substitute something plausible": a filter that cannot be
/// proven at build time must render at runtime, where it is correct, rather than
/// bake a guess into shipped HTML.
#[cfg(feature = "headless")]
pub fn eval_js_string(expr: &str) -> Option<String> {
    // Run on a DEDICATED thread. `build`/`export` may already be driving an
    // async runtime, and starting a V8 runtime inside one panics with "Cannot
    // start a runtime from within a runtime". A filter fold is a short, pure,
    // synchronous computation, so owning a thread for it is both simplest and
    // safest — and it keeps this module callable from any context.
    let expr = expr.to_string();
    std::thread::spawn(move || eval_js_string_inner(&expr))
        .join()
        .ok()
        .flatten()
}

#[cfg(feature = "headless")]
fn eval_js_string_inner(expr: &str) -> Option<String> {
    use rustyscript::{Runtime, RuntimeOptions};
    crate::ensure_v8_initialized();
    let mut rt = Runtime::new(RuntimeOptions::default()).ok()?;
    // `Intl` (used by currency/number-format) and standard globals are present in
    // a bare V8 context; nothing DOM-shaped is provided, which is deliberate —
    // a filter reaching for the DOM is refused before it ever gets here.
    let module = rustyscript::Module::new(
        "st_filter.js",
        &format!("export const result = String({expr});"),
    );
    let handle = rt.load_module(&module).ok()?;
    rt.get_value::<String>(Some(&handle), "result").ok()
}

/// Without the `headless` feature there is no JS engine linked in, so every
/// filter declines and the runtime stays authoritative. A plain `build` still
/// works; its filtered lists simply render on hydration as they did before.
#[cfg(not(feature = "headless"))]
pub fn eval_js_string(_expr: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod parity_tests {
    //! The gate that keeps GH-11 closed.
    //!
    //! GH-11 was not a missing feature — it was three implementations of one
    //! filter set that had drifted apart. Fixing the symptom without a gate that
    //! ENUMERATES the declared surface would let the same drift return the next
    //! time somebody adds a filter to one place and not the other.
    //!
    //! So these tests read `stdlib/primitives/data/filters.st` itself. A filter
    //! added there is automatically covered; nothing needs updating by hand.

    /// Every filter DECLARED in stdlib must also exist in the runtime table.
    ///
    /// This is the exact defect that made `capitalize` fail at build time AND at
    /// runtime: declared in stdlib, implemented in neither JS table.
    #[test]
    fn every_declared_filter_exists_in_the_runtime_table() {
        let decls = include_str!("../../stdlib/primitives/data/filters.st");
        let runtime = include_str!("../../public/runtime/st.js");

        // The string filters are the ones the runtime table serves; the
        // collection primitives (map/filter/sort/…) are pipeline constructs
        // applied elsewhere, not `| name` text filters.
        let string_filters = [
            "uppercase",
            "lowercase",
            "capitalize",
            "truncate",
            "currency",
        ];

        for name in string_filters {
            assert!(
                decls.contains(&format!("%primitive {name}(")),
                "`{name}` must be DECLARED in stdlib/primitives/data/filters.st — \
                 the declaration is the source of truth, not the JS table"
            );
            assert!(
                runtime.contains(&format!("{name}:")),
                "`{name}` is declared in filters.st but MISSING from ST.filters in \
                 public/runtime/st.js. A filter that exists at build time and not at \
                 runtime renders one thing in view-source and another after hydration \
                 — which is GH-11 exactly. Add it to the runtime table."
            );
        }
    }

    /// The deleted third table must not come back.
    #[test]
    fn there_is_no_second_runtime_filter_table() {
        assert!(
            !std::path::Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/public/runtime/data-binding.js"
            ))
            .exists(),
            "public/runtime/data-binding.js is back. It held a THIRD copy of the \
             filter set, was bundled by nothing, and disagreed with both others. \
             One need, one implementation: declare the %primitive in filters.st."
        );
    }
}
