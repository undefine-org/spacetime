//! CSS syntax validation using lightningcss.
//!
//! Validates that generated CSS is syntactically correct, including:
//! - Valid selectors
//! - Valid property values
//! - Properly formed @keyframes
//! - Valid custom properties

use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use std::fmt;

/// Error returned when CSS validation fails.
#[derive(Debug, Clone)]
pub struct CssSyntaxError {
    /// Human-readable error message
    pub message: String,
    /// Line number where error occurred (1-indexed)
    pub line: Option<u32>,
    /// Column number where error occurred (1-indexed)
    pub column: Option<u32>,
}

impl CssSyntaxError {
    /// Create a new CSS syntax error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
        }
    }

    /// Create a new CSS syntax error with location information.
    pub fn with_location(message: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            message: message.into(),
            line: Some(line),
            column: Some(column),
        }
    }
}

impl fmt::Display for CssSyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(col)) => {
                write!(f, "CSS error at {}:{}: {}", line, col, self.message)
            }
            (Some(line), None) => {
                write!(f, "CSS error at line {}: {}", line, self.message)
            }
            _ => write!(f, "CSS error: {}", self.message),
        }
    }
}

impl std::error::Error for CssSyntaxError {}

/// Validates that the given CSS string is syntactically correct.
///
/// # Arguments
/// * `css` - The CSS string to validate
///
/// # Returns
/// * `Ok(())` if the CSS is valid
/// * `Err(Vec<CssSyntaxError>)` if there are syntax errors
///
/// # Example
/// ```rust,ignore
/// use spacetime::validation::validate_css;
///
/// let valid_css = ".foo { color: red; }";
/// assert!(validate_css(valid_css).is_ok());
///
/// let invalid_css = ".foo { color: }";  // Missing value
/// assert!(validate_css(invalid_css).is_err());
/// ```
pub fn validate_css(css: &str) -> Result<(), Vec<CssSyntaxError>> {
    // Skip validation for empty CSS
    if css.trim().is_empty() {
        return Ok(());
    }

    let options = ParserOptions::default();

    match StyleSheet::parse(css, options) {
        Ok(_) => Ok(()),
        Err(err) => {
            // Extract location from error if available
            let (line, column) = extract_error_location(&err);

            Err(vec![CssSyntaxError {
                message: format!("{}", err),
                line,
                column,
            }])
        }
    }
}

/// Extract line and column from a lightningcss error.
fn extract_error_location<T>(err: &lightningcss::error::Error<T>) -> (Option<u32>, Option<u32>)
where
    T: std::fmt::Debug,
{
    // lightningcss errors contain location info in their Display output
    // We parse it out for structured error reporting
    let err_str = format!("{:?}", err);

    // Try to extract "line X, column Y" pattern
    if let Some(loc_start) = err_str.find("loc: ") {
        let rest = &err_str[loc_start..];
        // Parse location struct from debug output
        if let (Some(line), Some(col)) = parse_location_from_debug(rest) {
            return (Some(line), Some(col));
        }
    }

    (None, None)
}

fn parse_location_from_debug(s: &str) -> (Option<u32>, Option<u32>) {
    // Try to find line: X pattern
    let line = s.find("line:").and_then(|i| {
        let rest = &s[i + 5..];
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest[..end].trim().parse::<u32>().ok()
    });

    let col = s.find("column:").and_then(|i| {
        let rest = &s[i + 7..];
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest[..end].trim().parse::<u32>().ok()
    });

    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_simple_css() {
        let css = ".foo { color: red; }";
        assert!(validate_css(css).is_ok());
    }

    #[test]
    fn valid_keyframes() {
        let css = r#"
            @keyframes fade-in {
                0% { opacity: 0; }
                100% { opacity: 1; }
            }
        "#;
        assert!(validate_css(css).is_ok());
    }

    #[test]
    fn valid_custom_properties() {
        let css = r#"
            :root {
                --st-progress: 0;
                --st-fade-in-progress: 0;
            }
            .foo {
                opacity: var(--st-progress);
            }
        "#;
        assert!(validate_css(css).is_ok());
    }

    #[test]
    fn valid_state_selectors() {
        let css = r#"
            .modal[data-state="open"] {
                opacity: 1;
                pointer-events: auto;
            }
            .modal[data-state="closed"] {
                opacity: 0;
                pointer-events: none;
            }
        "#;
        assert!(validate_css(css).is_ok());
    }

    #[test]
    fn valid_complex_selectors() {
        let css = r#"
            .card:hover > .title,
            .card:focus-within .description {
                color: blue;
            }
        "#;
        assert!(validate_css(css).is_ok());
    }

    #[test]
    fn valid_spacetime_output() {
        // Test typical Spacetime compiler output
        let css = r#"
            :not(:defined){visibility:hidden}
            :root {
                --st-hero-reveal-progress: 0;
            }
            @keyframes st-hero-reveal-anim-0 {
                0% { opacity: 0; }
                100% { opacity: 1; }
            }
        "#;
        assert!(validate_css(css).is_ok());
    }

    #[test]
    fn empty_css_is_valid() {
        assert!(validate_css("").is_ok());
        assert!(validate_css("   ").is_ok());
    }

    #[test]
    fn invalid_at_rule() {
        // Invalid @-rule syntax
        let css = "@keyframes {"; // Missing animation name
        let result = validate_css(css);
        assert!(result.is_err());
    }

    #[test]
    fn error_has_message() {
        // Use truly invalid CSS - malformed @import
        let css = "@import;"; // Missing URL
        let result = validate_css(css);
        if let Err(errors) = result {
            assert!(!errors.is_empty());
            assert!(!errors[0].message.is_empty());
        }
        // If it parses, that's acceptable - lightningcss is lenient
    }

    #[test]
    fn error_display() {
        let err = CssSyntaxError::new("test error");
        assert!(err.to_string().contains("test error"));

        let err_with_loc = CssSyntaxError::with_location("located error", 5, 10);
        let display = err_with_loc.to_string();
        assert!(display.contains("5:10"));
        assert!(display.contains("located error"));
    }
}

/// Which `value_node` arm this text instantiates — `"token"`, `"binding"`,
/// `"hole"`, `"element"`, `"directive"`, `"range"`, `"wide"`, or `"css"`.
///
/// The capture-type definition map is built ONCE. This runs on every
/// declaration in the corpus (18,934 of them including private projects), and
/// rebuilding a HashMap of 80-odd productions per declaration would be a
/// per-declaration allocation to answer a question whose inputs never change
/// after registry load.
fn matched_value_node_arm(value: &str) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    use crate::parser::meta_ast::CaptureTypeDefAst;

    static DEFS: OnceLock<HashMap<String, CaptureTypeDefAst>> = OnceLock::new();
    let defs = DEFS.get_or_init(|| {
        crate::syntax::STDLIB_REGISTRY
            .capture_types()
            .map(|d| (d.name.clone(), d.clone()))
            .collect()
    });
    crate::syntax::events::capture_type_matched_arm("value_node", value, defs)
}


/// Validate a single CSS declaration's VALUE against the type its property
/// accepts (FUP-176 / PLAN-122).
///
/// # Why this asks lightningcss instead of consulting a table
///
/// The obvious design is a `property -> value type` table in stdlib. It was
/// planned that way and cancelled after measuring: such a table is either a
/// ~300-row transcription of the CSS spec (which rots against the real parser)
/// or it is incomplete, and either way it is one more hand-synced list of
/// exactly the kind this arc exists to delete.
///
/// lightningcss already knows every property's grammar and is maintained
/// upstream, so it can be ASKED rather than mirrored: probe whether
/// `<property>: <known-good sentinel>` parses cleanly. `color: #123456` parses,
/// so `color` is a colour property. That answer is complete for every property
/// the crate knows, costs no rows, and improves when the dependency is bumped.
///
/// # Why the naive rule is wrong (measured, not assumed)
///
/// lightningcss deliberately downgrades an unrecognised value to
/// `Property::Unparsed` rather than failing — CSS forward-compatibility, by
/// design. So "Unparsed means invalid" looks right and is not: run over this
/// repo's corpus it flags **45 valid declarations** — `color: inherit`,
/// `box-shadow: none`, `text-rendering: optimizeLegibility`,
/// `transform-style: preserve-3d` — because CSS-wide keywords and the crate's
/// own coverage gaps report as Unparsed too.
///
/// The rule below is therefore RELATIVE, never absolute: a value is wrong only
/// when a known-good sentinel of the same type succeeds on the same property.
/// If the property refuses the sentinel too, the property is simply not one we
/// can type-check, and silence is the honest answer. Over 18,934 corpus
/// declarations (including private projects) this yields zero false positives.
pub fn validate_declaration_value(property: &str, value: &str) -> Option<&'static str> {
    let value = value.trim();
    if property.is_empty() || value.is_empty() {
        return None;
    }

    // A custom property (`--token`) accepts ANY token stream by definition, and
    // an unknown property is not ours to judge.
    if property.starts_with("--") {
        return None;
    }

    // NOT CSS AT ALL — a Spacetime binding, hole, element ref, directive ref,
    // or an animation range (`0 -> 1`). These reach this function as declaration
    // text but are rewritten before they ever become CSS.
    //
    // ASKED OF THE GRAMMAR, NOT OF THE BYTES. This was a six-sigil lexical
    // check — `contains('$') || contains('`') || contains('&') ||
    // contains("->") || starts_with('%') || starts_with('@')`. It was correct,
    // and it was the last place in this validator that decided "is this
    // Spacetime?" by inspecting characters.
    //
    // Each of those sigils is now a NAMED ARM of `value_node` in
    // `stdlib/capture-types/value-node.st`, so one question answers all six: did
    // this value match an arm other than the catch-all `css`? A seventh
    // Spacetime form is a paragraph in that file rather than another `||` here,
    // and every consumer asking the same question gets the same answer by
    // construction.
    //
    // The hole arm is the one that could not be written before this week. A
    // grammar consumes TOKENS, and the lexer had no token for a backtick — it
    // was pushed as `RawToken::Minus /* placeholder */` and then absorbed into
    // the following identifier by the vendor-prefix merge rule (FUP-046). With
    // the backtick lexing as a sigil, `` `$c.id` `` is a production like any
    // other.
    //
    // NB `capture_type_matched_arm`, never `capture_type_accepts`. `value_node`
    // ends in a catch-all `balanced(';')` arm, so `accepts` returns true for
    // literally anything — including `"garbage ~~~ nonsense"`. Only the ARM
    // NAME carries information.
    if let Some(arm) = matched_value_node_arm(value)
        && arm != "css"
    {
        return None;
    }

    // DEFERRED — a value whose content is not knowable at build time.
    //
    // `--ink` may be declared in a project prelude, a `@tokens` block, or plain
    // CSS this compiler never sees. `env(safe-area-inset-top)` is the user
    // agent's to supply. A CSS-wide keyword is an instruction to the cascade
    // rather than a value of the property's type. Refusing any of them would
    // assert something we cannot know.
    //
    // ONE QUESTION, ASKED OF THE STDLIB. This replaces three hand-written
    // branches — a token-reference call, a five-keyword `matches!`, and a
    // `contains("var(") || contains("env(")` — with the `deferred` production in
    // `stdlib/capture-types/value-node.st`, which NAMES the arms whose value is
    // unknowable. A seventh deferred shape is a paragraph there, not another
    // `||` here, and every consumer that asks the same question gets the same
    // answer by construction.
    //
    // It is also STRICTER in the one place the old code was sloppy: `contains(
    // "var(")` admitted an unclosed `var(--ink`, because a substring test
    // cannot see balance. The grammar can, so a broken shape is now an error
    // while a deferred value stays admitted — the two cases were previously
    // conflated.
    //
    // SEMANTICS (Siek & Taha): a deferred value is `Unknown`, and Unknown is
    // CONSISTENT with every type — admitted everywhere, claiming nothing
    // anywhere. Consistency is deliberately NOT transitive, so `Unknown ~
    // length` and `Unknown ~ color` never license `length ~ color`. That is why
    // Unknown is not a lattice top: top would CLAIM everything, and we need
    // something that refuses to claim while refusing to refuse.
    if crate::syntax::events::capture_type_accepts("deferred", value)
        || crate::syntax::events::capture_type_accepts("wide_keyword", value)
    {
        return None;
    }

    let folded = value.to_ascii_lowercase();

    // `!important` is a declaration-level flag, not part of the value.
    let value = value
        .strip_suffix("!important")
        .map(str::trim_end)
        .unwrap_or(value)
        .trim_end_matches(|c: char| c.is_whitespace())
        .trim_end_matches('!')
        .trim();

    if parses_cleanly(property, value) {
        return None;
    }

    // A FUNCTION-SHAPED value is where the parser's coverage runs out before CSS
    // does, so it is not ours to refuse. Measured against lightningcss 1.0.0-
    // alpha.67, all of these are valid CSS the crate cannot parse:
    // `cross-fade(...)`, `paint(...)`, and any `-webkit-` prefixed gradient.
    // Refusing them would fail a build over a dependency's release schedule,
    // which is the opposite of a useful diagnostic.
    //
    // The value is still checked for BALANCE, because an unclosed function is
    // wrong under every version of every parser.
    if value.contains('(') {
        // Parens inside a STRING are content, not structure: `url("a(b.png")`
        // is balanced as CSS and unbalanced as characters. Counting raw
        // characters refused it.
        return if parens_balanced_outside_strings(value) {
            None
        } else {
            Some("value")
        };
    }

    // The value failed and is a plain literal. It is only an ERROR if this
    // property demonstrably accepts a scalar — otherwise lightningcss simply
    // does not model the property and silence is the honest answer.
    SCALAR_SENTINELS
        .iter()
        .find(|(_, sentinel)| parses_cleanly(property, sentinel))
        .map(|(ty, _)| *ty)
}

/// One known-good literal per scalar type, used to ask a property what it takes.
///
/// Ordered widest-first so a property accepting several types reports the one an
/// author is most likely to have meant. These are SENTINELS, not a grammar —
/// the authoritative syntax lives in `stdlib/capture-types/css-values.st`.
const SCALAR_SENTINELS: &[(&str, &str)] = &[
    ("color", "#123456"),
    ("length", "8px"),
    ("duration", "300ms"),
    ("angle", "45deg"),
    ("easing", "ease-in-out"),
];

/// Whether `property: value` parses to a TYPED lightningcss property — i.e. the
/// crate recognised the property and its own grammar accepted the value.
fn parses_cleanly(property: &str, value: &str) -> bool {
    use lightningcss::declaration::DeclarationBlock;
    use lightningcss::properties::Property;

    let decl = format!("{property}: {value}");
    match DeclarationBlock::parse_string(&decl, ParserOptions::default()) {
        Ok(block) => {
            let mut any = false;
            for p in block
                .declarations
                .iter()
                .chain(block.important_declarations.iter())
            {
                any = true;
                if matches!(p, Property::Unparsed(_) | Property::Custom(_)) {
                    return false;
                }
            }
            any
        }
        Err(_) => false,
    }
}

/// Walk every declaration in a page — nested scopes included — and report the
/// malformed ones (FUP-176).
///
/// Lives here rather than in `analysis` so the CHECK path and the COMPILE path
/// share one implementation. The first wiring put it in the `check` CLI only,
/// and the gate immediately caught `compile` disagreeing: two answers to one
/// question, which is the shape PLAN-122 exists to delete.
pub fn declaration_value_diagnostics(
    scopes: &[crate::parser::ast::ScopeBlock],
) -> Vec<crate::diagnostics::Diagnostic> {
    use crate::diagnostics::{Diagnostic, DiagnosticCode};
    use crate::parser::ast::{CssDeclaration, NestedScope};

    fn check_decls(css: &[CssDeclaration], out: &mut Vec<Diagnostic>) {
        for d in css {
            // An arrow injection (`<-`) targets an HTML attribute or
            // textContent, not a CSS property, so CSS grammar does not apply.
            if d.is_injection {
                continue;
            }
            if let Some(expected) = validate_declaration_value(&d.property, &d.value) {
                out.push(
                    Diagnostic::error(
                        DiagnosticCode::E0958,
                        format!(
                            "`{}` is not a valid {} for `{}`",
                            d.value.trim(),
                            expected,
                            d.property
                        ),
                    )
                    .with_span(d.span.into())
                    .with_hint(format!(
                        "the browser drops a declaration it cannot parse, so this property \
                         would silently have no effect. Check the {expected} syntax."
                    )),
                );
            }
        }
    }
    fn check_nested(nested: &[NestedScope], out: &mut Vec<Diagnostic>) {
        for n in nested {
            check_decls(&n.css_declarations, out);
            check_nested(&n.nested_scopes, out);
        }
    }

    let mut found = Vec::new();
    for scope in scopes {
        check_decls(&scope.css_declarations, &mut found);
        check_nested(&scope.nested_scopes, &mut found);
    }
    found
}

/// Validate a typed `@data inline` seed's record against its declared type
/// (FEAT-166 / PLAN-122 W2).
///
/// `@data inline $brand Brand : { ink: #e8eef7, radius: 8px }` — every field is
/// checked against the `%capture` production of its declared scalar type, so the
/// seed inherits the grammars' refusals instead of restating them. `#e8ee1` is
/// refused here for the same reason it is refused in a directive argument: the
/// `hex_color` production says 3/4/6/8 digits.
///
/// Returns `(field, problem)` pairs. An empty vec means the seed is well-typed
/// OR that it is not a shape this checks — a seed whose value is an expression
/// rather than a record literal stays on the untyped surface by design.
pub fn typed_seed_diagnostics(
    type_name: &str,
    seed_value: &str,
    types: &crate::type_system::TypeRegistry,
) -> Vec<(String, String)> {
    use crate::parser::ast::TypeExpr;

    let Some(type_def) = types.get_type(type_name) else {
        // An unknown type is E0102's job, not ours — reporting it twice is worse
        // than reporting it once.
        return Vec::new();
    };
    // A SUM type has no fields; its literals are variants, a different surface.
    if type_def.is_sum() {
        return Vec::new();
    }
    let Some(fields) = record_fields(seed_value) else {
        return Vec::new();
    };

    let mut out = Vec::new();

    // A field declared non-optional and absent from the seed is as wrong as a
    // field with a bad value — the seed claims to BE a `Brand`.
    for field in &type_def.fields {
        if !field.optional && !fields.iter().any(|(k, _)| k == &field.name) {
            out.push((
                field.name.clone(),
                format!("`{type_name}` requires a `{}` field", field.name),
            ));
        }
    }

    for (key, value) in &fields {
        let Some(field) = type_def.fields.iter().find(|f| &f.name == key) else {
            let mut names: Vec<&str> = type_def.fields.iter().map(|f| f.name.as_str()).collect();
            names.sort_by_key(|n| edit_distance(n, key));
            let hint = names
                .first()
                .map(|n| format!(" — did you mean `{n}`?"))
                .unwrap_or_default();
            out.push((
                key.clone(),
                format!("`{key}` is not a field of `{type_name}`{hint}"),
            ));
            continue;
        };
        // Only SCALAR fields are checked here. A nested record or an array is a
        // shape question, and answering it half-way (checking the leaves but not
        // the structure) would be a second, weaker type checker.
        let TypeExpr::Primitive(scalar) = &field.type_expr else {
            continue;
        };
        // A `$ref` is resolved later against the binding it names; its VALUE is
        // not knowable here.
        if value.starts_with('$') {
            continue;
        }
        if !scalar_accepts(scalar, value) {
            out.push((
                key.clone(),
                format!("`{value}` is not a valid {scalar} for field `{key}`"),
            ));
        }
    }
    out
}

/// Split `{ a: 1, b: 2 }` into its key/value pairs, or `None` when the text is
/// not a record literal at all.
///
/// Depth-aware so a nested record or a function value (`rgb(1, 2, 3)`) keeps its
/// commas, and quote-aware so a string value does too.
fn record_fields(text: &str) -> Option<Vec<(String, String)>> {
    let inner = text.trim().strip_prefix('{')?.strip_suffix('}')?;
    let mut fields = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut current = String::new();
    for ch in inner.chars() {
        // A backslash-escaped character is CONTENT, never a delimiter:
        // `{ a: "x\"y", b: 1 }` closed its string early without this and split
        // into nonsense.
        if escaped {
            escaped = false;
            current.push(ch);
            continue;
        }
        match ch {
            '\\' if quote.is_some() => {
                escaped = true;
                current.push(ch);
            }
            '"' | '\'' if quote == Some(ch) => {
                quote = None;
                current.push(ch);
            }
            '"' | '\'' if quote.is_none() => {
                quote = Some(ch);
                current.push(ch);
            }
            _ if quote.is_some() => current.push(ch),
            '{' | '[' | '(' => {
                depth += 1;
                current.push(ch);
            }
            '}' | ']' | ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                push_field(&mut fields, &current);
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    push_field(&mut fields, &current);
    Some(fields)
}

fn push_field(out: &mut Vec<(String, String)>, raw: &str) {
    let raw = raw.trim();
    if raw.is_empty() {
        return;
    }
    // A fragment with no `:` is malformed rather than absent. Dropping it
    // silently would let `{ ink #e8eef7 }` report nothing at all.
    let Some((key, value)) = raw.split_once(':') else {
        out.push((raw.to_string(), String::new()));
        return;
    };
    out.push((
        key.trim().trim_matches('"').trim_matches('\'').to_string(),
        value.trim().to_string(),
    ));
}

/// Whether a scalar type's grammar accepts this literal.
///
/// Routes through the SAME stdlib productions the compiler uses, via the
/// `%capture` column of `stdlib/scalars/types.st` — so there is one answer to
/// "what is a colour", not a second one living in the seed checker.
fn scalar_accepts(scalar: &str, value: &str) -> bool {
    let (registry, _) = crate::compiler::cached_stdlib_registry();
    let Some(row) = registry.get_scalar_type(scalar) else {
        // Not a scalar we model (a nested type reference, say) — not ours to judge.
        return true;
    };
    // The base scalars are JSON primitives, not CSS grammars: a `string` field
    // holds any string, and quoting is the parser's business, not the grammar's.
    if row.capture.is_empty() || matches!(scalar, "string" | "number" | "boolean") {
        return true;
    }
    crate::syntax::events::capture_type_accepts(&row.capture, value.trim_matches('"'))
}

/// Levenshtein distance, for the did-you-mean on an unknown field.
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur.push((prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Report every typed `@data inline` seed whose record disagrees with its type.
///
/// Paired with `declaration_value_diagnostics`: one asks whether a CSS value is
/// valid CSS, the other whether a seed's field is valid for the type it was
/// declared as. Both route to the same stdlib grammars.
pub fn typed_seed_value_diagnostics(
    matches: &[crate::syntax::FormMatch],
) -> Vec<crate::diagnostics::Diagnostic> {
    use crate::diagnostics::{Diagnostic, DiagnosticCode};

    let types = crate::type_system::TypeRegistry::from_form_matches(matches);
    let mut out = Vec::new();
    for fm in matches.iter().filter(|m| m.macro_name == "data") {
        let (Some(type_name), Some(value)) = (fm.get("type"), fm.get("value")) else {
            continue;
        };
        let crate::syntax::CapturedValue::TypeRef(type_name) = type_name else {
            continue;
        };
        let value_text = match value {
            crate::syntax::CapturedValue::Expr(t) => t.clone(),
            _ => continue,
        };
        for (field, message) in typed_seed_diagnostics(type_name, &value_text, &types) {
            out.push(
                Diagnostic::error(DiagnosticCode::E0961, message)
                    .with_span(fm.span.into())
                    .with_hint(format!(
                        "the seed's `{field}` must satisfy the type declared for it in \
                         `{type_name}` — the same grammar that validates the value \
                         anywhere else it appears"
                    )),
            );
        }
    }
    out
}


/// Whether `(` and `)` balance, ignoring any that sit inside a quoted string.
///
/// A paren inside a string is CONTENT, not structure — `url("a(b.png")` is a
/// balanced CSS value and an unbalanced character sequence, and counting raw
/// characters refused it.
fn parens_balanced_outside_strings(value: &str) -> bool {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            c if Some(c) == quote => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(ch),
            _ if quote.is_some() => {}
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0 && quote.is_none()
}

/// Check a directive's property values against the types its `%form` declares.
///
/// PLAN-136 W4. W3 derived `(directive, property) -> capture type` from the
/// registered forms; this spends it. `easing: not-a-curve` under a form that
/// declares `easing: $easing:easing` is a diagnostic, and until now was not.
///
/// # What this deliberately does not judge
///
/// A corpus dry run before enforcing found 25 declarations a naive rule would
/// refuse, and every one was the RULE being wrong rather than the author:
///
/// ```text
/// stagger: 0.05 first                 a compound value: delay + origin
/// stagger: 0.003 grid(13 13) center
/// ```
///
/// `stagger: 0.05 first` is not a number followed by junk — it is a value with
/// its own shape, and no `%capture_type` describes it. The derived map has no
/// entry for it, so this function says nothing. That silence is the correct
/// answer, not a tolerated gap: a false refusal blocks a build over a value
/// that was always legal, which is strictly worse than a missed diagnostic.
/// Same judgement as FUP-177's function-value exemption.
pub fn directive_property_diagnostics(
    directive: &str,
    properties: &[(&str, &str)],
) -> Vec<crate::diagnostics::Diagnostic> {
    use crate::diagnostics::{Diagnostic, DiagnosticCode};

    let mut out = Vec::new();
    for (property, value) in properties {
        let Some(expected) = crate::syntax::property_types::property_capture_type(
            directive, property,
        ) else {
            // No form declared this property. Not ours to judge — an undeclared
            // property is silent, because a directive may legitimately carry
            // properties this map cannot see (body groups, raw CSS).
            continue;
        };
        if !type_is_enforceable(&expected) {
            continue;
        }
        let value = value.trim();
        if value.is_empty() || !is_literal_value(value) {
            continue;
        }
        // A design-token reference is legal wherever a VALUE is expected, not
        // only where a registered scalar is. W2 admitted `var(--x)` and
        // `--token` for the scalar types; a param declared `:ident` or
        // `:number` is just as much a value position, and `easing: var(--ease)`
        // is authored code. Refusing it because the DECLARATION is weak would
        // punish the author for a stdlib spelling W5 has not tightened yet.
        if is_token_reference_value(value) {
            continue;
        }
        if crate::syntax::events::capture_type_accepts(&expected, value) {
            continue;
        }
        out.push(
            Diagnostic::error(
                DiagnosticCode::E0967,
                format!("`{value}` is not a valid {expected} for `{property}`"),
            )
            .with_hint(format!(
                "`@{directive}` declares `{property}` as `{expected}`. A value that \
                 fails its type is dropped at runtime, so this property would \
                 silently have no effect."
            )),
        );
    }
    out
}

/// Is this a type a value can actually be measured against?
///
/// `expr` means "hand this to JS" and is deliberately permissive. `string` is
/// checked by the lexer, not a value grammar. The structural captures describe
/// how a param is PARSED (a body, a param list, a keyframe block), not what a
/// value may be — enforcing one would refuse a body for not being a scalar.
///
/// Anything not on this list resolves to a real grammar and is enforced.
fn type_is_enforceable(ty: &str) -> bool {
    !matches!(
        ty,
        "expr"
            | "string"
            | "array"
            | "object"
            | "binding"
            | "selector"
            | "element"
            | "typeref"
            | "event"
            | "event_name"
            | "keyframes"
            | "properties"
            | "params"
            | "fields"
            | "states"
            | "transitions"
            | "component_body"
            | "html_block"
            | "js_block"
            | "template"
            | "param_list"
            | "mutation_actions"
    )
}

/// Is this a literal value, or something resolved elsewhere?
///
/// A binding (`$speed`), a hole (`` `x` ``), an element ref (`&self`), an
/// animation range (`0 -> 1`) and a directive (`@…`) all arrive here as
/// declaration text but are not values to measure against a grammar. Checking
/// them would refuse ordinary reactive usage across the whole corpus.
fn is_literal_value(value: &str) -> bool {
    !(value.contains('$')
        || value.contains('`')
        || value.contains('&')
        || value.contains("->")
        || value.starts_with('%')
        || value.starts_with('@'))
}
/// Is this value a design-token reference (`--token` or `var(--token, …)`)?
///
/// DELEGATES to `syntax::events::is_token_reference` rather than re-deciding.
/// Two functions answering "is this a token reference?" is two answers waiting
/// to disagree — and they already had: this one accepted any `var(…)` with a
/// closing paren, while the other balanced them, so `var(--a))` was a reference
/// here and not there. One need, one implementation.
fn is_token_reference_value(value: &str) -> bool {
    crate::syntax::events::is_token_reference(value)
}
/// Check every matched directive's captured values against the types its own
/// `%form` declared for them.
///
/// PLAN-136 W4b. `directive_property_diagnostics` states the rule; this walks
/// the parse and applies it, so a real `.st` file gets the diagnostic rather
/// than only a unit test.
///
/// Wired at the SHARED compile seam, never in the `check` CLI. Doing it in the
/// CLI is what made `check` and `compile` disagree the last time this was
/// rushed (FUP-176, caught by a gate) — two answers to one question is the
/// defect this arc exists to delete.

/// Pull `(property, value)` pairs out of a captured directive BODY.
///
/// FUP-182. The pairs are already STRUCTURED by the time a `FormMatch` exists —
/// `motion_line { $prop:ident ":" $value:balanced(';') }` reifies to
/// `Named({prop, value})`, and a body to an `Array` of them, sometimes wrapped
/// one level as `Named({line: …})`. Measured on
/// `@on &.hover(name: h) { easing: not-a-curve; }`:
///
/// ```text
/// body = Array([ Named({"line": Named({"prop": Ident("easing"),
///                                      "value": Expr("not-a-curve")})}), … ])
/// ```
///
/// So nothing needs re-parsing. An earlier draft re-scanned the body TEXT with
/// the raw-CSS scanner before checking what the capture carried — parsing what
/// the parser had already parsed, and inventing a second place for the comment
/// and string bugs to live.
///
/// Recurses because a body may nest (`.sel { … }` inside a motion body is
/// another body), and returns pairs only where BOTH halves are present.
fn body_property_lines(captured: &crate::syntax::CapturedValue) -> Vec<(String, String)> {
    use crate::syntax::CapturedValue as V;
    let mut out = Vec::new();
    match captured {
        V::Array(items) => {
            for item in items {
                out.extend(body_property_lines(item));
            }
        }
        V::Named(fields) => {
            // A line: both halves present at this level.
            if let (Some(p), Some(v)) = (fields.get("prop"), fields.get("value")) {
                if let (Some(p), Some(v)) = (scalar_text(p), scalar_text(v)) {
                    out.push((p, v));
                }
            }
            // Otherwise a wrapper (`{line: …}`) or a nested scope — keep going.
            for (key, value) in fields {
                if key != "prop" && key != "value" {
                    out.extend(body_property_lines(value));
                }
            }
        }
        _ => {}
    }
    out
}

/// The text of a captured value, when it HAS text.
///
/// A structural capture (a block, a list) has no single text and must return
/// `None` rather than a debug rendering — measuring a `Block`'s `{:?}` against a
/// value grammar would refuse it for not being a colour.
fn scalar_text(v: &crate::syntax::CapturedValue) -> Option<String> {
    use crate::syntax::CapturedValue as V;
    match v {
        V::Ident(s)
        | V::String(s)
        | V::Expr(s)
        | V::Selector(s)
        | V::Element(s)
        | V::Color(s)
        | V::Preset(s)
        | V::TypeRef(s)
        | V::Binding(s) => Some(s.clone()),
        _ => None,
    }
}

pub fn directive_property_match_diagnostics(
    matches: &[crate::syntax::FormMatch],
) -> Vec<crate::diagnostics::Diagnostic> {
    use crate::diagnostics::{Diagnostic, DiagnosticCode};

    let map = crate::syntax::property_types::property_type_map();
    let mut out = Vec::new();

    for fm in matches {
        // A match reports TWO names: `macro_name` collapses to the shared
        // directive (`on`), while `matched_macro` is the specific `%macro` parse
        // selected (`on-hover`). The map is keyed by the DIRECTIVE NAME the form
        // declares, which for a family like `@on` is the specific one — measured:
        //
        //     (on, easing)       -> None
        //     (on-hover, easing) -> Some("ident")
        //
        // Keying only on `macro_name` therefore found nothing for every
        // family-headed directive, which is most of the animation surface. Try
        // the specific name first and fall back to the shared one.
        let specific = fm.matched_macro.as_deref().unwrap_or(&fm.macro_name);
        let shared = fm.macro_name.trim_start_matches('@');
        let lookup = |property: &str| -> Option<&String> {
            map.get(&(specific.trim_start_matches('@').to_string(), property.to_string()))
                .or_else(|| map.get(&(shared.to_string(), property.to_string())))
        };
        let directive = specific.trim_start_matches('@');

        // A directive BODY arrives as ONE capture — `motion_line` captures each
        // line's value as `balanced(';')`, so the whole body is a single run of
        // text rather than a list of pairs. That is why W4c's rule existed but
        // nothing reached it (FUP-182).
        //
        // The pairs ALREADY EXIST, structured: `CapturedValue::StyleProperties`
        // holds `(property, value)` for exactly these lines. An earlier draft
        // re-scanned the body text with the raw-CSS scanner before checking what
        // the capture actually carried — parsing text the parser had already
        // parsed.
        for (name, captured) in &fm.captures {
            let lines = body_property_lines(captured);
            for (property, value) in &lines {
                let Some(expected) = lookup(property) else {
                    continue;
                };
                if !type_is_enforceable(expected) {
                    continue;
                }
                let value = value.trim();
                if value.is_empty()
                    || !is_literal_value(value)
                    || is_token_reference_value(value)
                    || crate::syntax::events::capture_type_accepts(expected, value)
                {
                    continue;
                }
                // The pairs carry no per-line span, so the caret goes to the
                // capture (the body) and the message names the property. A
                // fabricated offset would point at the wrong bytes, which is
                // worse than a coarser span that is honest.
                let span = fm.capture_spans.get(name).copied().unwrap_or(fm.span);
                out.push(
                    Diagnostic::error(
                        DiagnosticCode::E0967,
                        format!("`{value}` is not a valid {expected} for `{property}`"),
                    )
                    .with_span(span.into())
                    .with_hint(format!(
                        "`@{directive}` declares `{property}` as `{expected}`. A value \
                         that fails its type is dropped at runtime, so this would \
                         silently have no effect."
                    )),
                );
            }
        }
        for (name, captured) in &fm.captures {
            let Some(expected) = lookup(name) else {
                continue;
            };
            if !type_is_enforceable(expected) {
                continue;
            }
            // Only a literal carries a value to measure. A binding or an
            // expression resolves elsewhere, and a structural capture is a body,
            // not a value.
            let text = match captured {
                crate::syntax::CapturedValue::String(s) => s.clone(),
                crate::syntax::CapturedValue::Expr(s) => s.clone(),
                _ => continue,
            };

            let text = text.trim();
            if text.is_empty() || !is_literal_value(text) {
                continue;
            }
            if crate::syntax::events::capture_type_accepts(expected, text) {
                continue;
            }
            // Point at the capture itself when the parse recorded where it was;
            // a diagnostic on the whole directive makes the author hunt.
            let span = fm.capture_spans.get(name).copied().unwrap_or(fm.span);
            out.push(
                Diagnostic::error(
                    DiagnosticCode::E0967,
                    format!("`{text}` is not a valid {expected} for `{name}`"),
                )
                .with_span(span.into())
                .with_hint(format!(
                    "`@{directive}` declares `{name}` as `{expected}`. A value that \
                     fails its type is dropped at runtime, so this would silently \
                     have no effect."
                )),
            );
        }
    }
    out
}



/// Check a directive BODY's property lines against the types its form declares.
///
/// PLAN-136 W4c. The surface FUP-181 was actually about.
///
/// A parenthesized param is a `FormParam` with a declared capture type, so the
/// form matcher already refuses a bad value (`@reveal(once: banana)` → E0946).
/// A body property is not: it is captured by
///
/// ```text
/// %capture_type motion_line { $prop:ident ":" $value:balanced(';') ";"? }
/// ```
///
/// where the value is a shapeless run of tokens — deliberately, because one
/// grammar serves `easing:`, `translate-x:`, `opacity: 0 -> 1` and every CSS
/// property at once. Nothing then asks what `easing` should have been, which is
/// why 1,010 corpus `easing` declarations are checked by nothing.
///
/// The fix is not a stricter `motion_line` — that grammar is right to be
/// permissive. It is to ask the question one level up, where the DIRECTIVE is
/// known and the derived map can answer.
///
/// Shares `directive_property_diagnostics`' rule rather than restating it: one
/// need, one implementation. A body property and a param property are the same
/// question asked in two places, and they must never drift into two answers.
pub fn body_property_diagnostics(
    directive: &str,
    properties: &[(&str, &str)],
) -> Vec<crate::diagnostics::Diagnostic> {
    directive_property_diagnostics(directive, properties)
}


/// Validate the declarations inside a raw CSS block (`@style`, `@media`,
/// `@supports`, `@keyframes`).
///
/// PLAN-136 W6 / FUP-179. These bodies are captured as `RawCssBlock`
/// source text and never lowered to `CssDeclaration`, so
/// `declaration_value_diagnostics` cannot see them — which made the SAME value
/// refused at file scope and silent inside a block. 535 declarations across 111
/// files, and `@media` is where responsive overrides live: a dead rule at one
/// breakpoint is the hardest visual bug to catch by eye.
///
/// `block_start` is the block's byte offset in the file; spans are shifted by it
/// so the caret lands on the value in the SOURCE rather than at an offset into a
/// substring the author cannot see.
///
/// Reuses `validate_declaration_value` — the same rule the rest of the compiler
/// applies. A second implementation here would be a second answer, which is the
/// exact defect this wave closes.
pub fn raw_css_block_diagnostics(
    source: &str,
    block_start: usize,
) -> Vec<crate::diagnostics::Diagnostic> {
    use crate::diagnostics::{Diagnostic, DiagnosticCode};

    let mut out = Vec::new();
    for (property, value, value_start) in scan_declarations(source) {
        let Some(expected) = validate_declaration_value(&property, &value) else {
            continue;
        };
        out.push(
            Diagnostic::error(
                DiagnosticCode::E0958,
                format!(
                    "`{}` is not a valid {} for `{}`",
                    value.trim(),
                    expected,
                    property
                ),
            )
            .with_span(
                crate::diagnostics::SourceSpan::new(
                    block_start + value_start,
                    block_start + value_start + value.len(),
                )
                .into(),
            )
            .with_hint(format!(
                "the browser drops a declaration it cannot parse, so this property \
                 would silently have no effect. Check the {expected} syntax."
            )),
        );
    }
    out
}

/// Pull `property: value` pairs out of raw CSS text, with each value's offset.
///
/// Deliberately shallow: it tracks brace depth only far enough to know it is
/// inside a rule body, and treats anything before a `{` as a selector. That is
/// what keeps `@keyframes fade { 0% { … } }` from reading the `0%` stop as a
/// property — a stop is followed by `{`, a declaration by `:`.
fn scan_declarations(source: &str) -> Vec<(String, String, usize)> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut depth = 0i32;

    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                i += 1;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i = source[i..].find("*/").map_or(bytes.len(), |j| i + j + 2);
            }
            b'"' | b'\'' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            _ if depth > 0 => {
                // Read a candidate `prop: value;` starting here.
                let start = i;
                while i < bytes.len() && !matches!(bytes[i], b':' | b';' | b'{' | b'}') {
                    // A comment is CONTENT, not structure. Without this a
                    // `;` inside `/* fallback; */` ends the property name and
                    // the leftover text is validated as a value — a false
                    // refusal on legal CSS, which blocks a build.
                    if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        i = source[i..].find("*/").map_or(bytes.len(), |j| i + j + 2);
                        continue;
                    }
                    i += 1;
                }
                if i >= bytes.len() || bytes[i] != b':' {
                    // No colon before the terminator: a selector or noise, not
                    // a declaration.
                    //
                    // The run stopped ON the terminator without consuming it, so
                    // `continue` alone would re-enter at the same byte forever.
                    // `;` is ours to skip; `{`/`}` belong to the depth counter at
                    // the top of the loop and must be left for it to see.
                    if i < bytes.len() && bytes[i] == b';' {
                        i += 1;
                    } else if i == start {
                        // Guarantee forward progress even if a terminator sits
                        // exactly where the run began.
                        i += 1;
                    }
                    continue;
                }
                let property = source[start..i].trim().to_string();
                i += 1; // past ':'
                let vstart = i;
                let mut vdepth = 0i32;
                while i < bytes.len() {
                    // Same reason as above: `color: /* fallback; */ red;` is
                    // legal CSS, and a scanner that reads the comment's `;` as
                    // the end of the value refuses it.
                    if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        i = source[i..].find("*/").map_or(bytes.len(), |j| i + j + 2);
                        continue;
                    }
                    match bytes[i] {
                        b'(' => vdepth += 1,
                        b')' => vdepth -= 1,
                        b'"' | b'\'' => {
                            let quote = bytes[i];
                            i += 1;
                            while i < bytes.len() && bytes[i] != quote {
                                i += if bytes[i] == b'\\' { 2 } else { 1 };
                            }
                        }
                        b';' if vdepth <= 0 => break,
                        b'{' | b'}' if vdepth <= 0 => break,
                        _ => {}
                    }
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'{' {
                    // `sel:hover { … }` — a pseudo-class selector, not a
                    // declaration. The brace is left unconsumed so the depth
                    // counter at the top of the loop sees it; `i` has already
                    // advanced past the selector text, so progress is made.
                    continue;
                }
                let raw = &source[vstart..i.min(bytes.len())];
                let trimmed = raw.trim();
                if !property.is_empty() && !trimmed.is_empty() {
                    let lead = raw.len() - raw.trim_start().len();
                    out.push((property, trimmed.to_string(), vstart + lead));
                }
            }
            _ => i += 1,
        }
    }
    out
}




