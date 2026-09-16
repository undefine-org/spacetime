//! Unified form-matching scorer for directive disambiguation.
//!
//! Provides a single scoring algorithm used by:
//! - Pipeline resolve (CaptureMapInput)
//! - Metasystem expand/compile (PatternArgInput)
//! - LSP (future: FormMatchInput adapter)

use std::collections::{HashMap, HashSet};

use crate::parser::ast::PatternArg;
use crate::parser::meta_ast::{
    CaptureModifier, CaptureType, FormClause, FormInlineElement, MacroDefAst,
};
use crate::syntax::CapturedValue;

// =============================================================================
// Trait
// =============================================================================

/// Answers questions about a directive invocation for form scoring.
pub trait FormMatchInput {
    /// Does the invocation provide a value named `name`?
    fn has_value(&self, name: &str) -> bool;

    /// Is the value for `name` type-compatible with `capture_type`?
    /// Returns true when no value exists (absence != type mismatch).
    fn is_compatible(&self, name: &str, capture_type: &CaptureType) -> bool;

    /// Is the value for `name` a body-like type (Block, Properties, Keyframes)?
    fn is_body_value(&self, name: &str) -> bool;

    /// Does the invocation have body content (`{ ... }` block)?
    fn has_body(&self) -> bool;

    /// Did the literal at `inline_element_index` match a positional arg?
    /// Default returns true (assume match) for implementations that don't track literals.
    fn literal_matched(&self, _inline_element_index: usize) -> bool {
        true
    }
}

// =============================================================================
// Core Scoring
// =============================================================================

/// A single scored contribution recorded while matching a form against an
/// invocation. The introspection counterpart of one branch of the scorer
/// (FEAT-087): same points, same verdict, surfaced instead of summed-and-dropped.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ScoreTerm {
    /// Human-readable label for the form element this term scored
    /// (e.g. "literal `fetch`", "$name:binding", "param color: $c").
    pub element_label: String,
    /// Points this element contributed (matches the scorer's weights exactly:
    /// +3 matched literal, +2 required capture/comparison, +1 optional/param,
    /// +10 body, -100 literal-miss, -5 uncovered body, 0 absent-ok).
    pub points: i32,
    /// What happened: "matched" | "absent-ok" | "literal-miss" | "uncovered-body"
    /// for soft outcomes; "missing" | "type-mismatch" for the hard rejects that
    /// make `total` None.
    pub verdict: String,
}

/// The full, faithful breakdown of scoring one `FormClause` against an
/// invocation. `total` is `None` iff a hard-reject term fired (`missing` /
/// `type-mismatch`), mirroring `score_form_match`'s `Option`. The fast path
/// delegates here, so they can never diverge.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ScoreExplanation {
    pub total: Option<u32>,
    pub terms: Vec<ScoreTerm>,
}

impl ScoreExplanation {
    fn rejected(mut self, term: ScoreTerm) -> Self {
        self.terms.push(term);
        self.total = None;
        self
    }
}

/// Short type name for a `CaptureType`, used only for introspection labels.
/// FEAT-089 owns the rich signature formatter; this is the minimal label form.
pub fn capture_type_label(ct: &CaptureType) -> String {
    match ct {
        CaptureType::Union(opts) => format!("({})", opts.join("|")),
        CaptureType::Balanced(c) => format!("balanced('{c}')"),
        CaptureType::PatternMatch { variant, .. } => format!("is {variant}"),
        CaptureType::Custom(s) => s.clone(),
        other => {
            // Derive a lowercase short name from the Debug spelling (Ident -> "ident").
            let dbg = format!("{other:?}");
            let head = dbg.split(['(', ' ', '{']).next().unwrap_or(&dbg);
            head.to_lowercase()
        }
    }
}

fn modifier_suffix(m: &CaptureModifier) -> &'static str {
    match m {
        CaptureModifier::Required => "",
        CaptureModifier::Optional => "?",
        CaptureModifier::ZeroOrMore => "*",
        CaptureModifier::OneOrMore => "+",
        // See metasystem::signature: counted repetition is internal to a char-class
        // terminal and does not participate in form scoring.
        CaptureModifier::Counted(_) => "",
    }
}

fn capture_label(var_name: &str, ct: &CaptureType, m: &CaptureModifier) -> String {
    format!(
        "${}:{}{}",
        var_name,
        capture_type_label(ct),
        modifier_suffix(m)
    )
}

/// Score how well a `FormClause` matches an invocation.
/// Higher score = better match. Returns `None` if a required element is missing
/// or has a type mismatch.
///
/// Delegates to [`explain_form_match`] so the fast path and the introspection
/// breakdown can never disagree (FEAT-087).
pub fn score_form_match(form: &FormClause, input: &impl FormMatchInput) -> Option<u32> {
    explain_form_match(form, input).total
}

/// Faithful, term-by-term explanation of scoring `form` against `input`.
///
/// This is a 1:1 of [`score_form_match`]'s original branch logic: every branch
/// records a [`ScoreTerm`] with the SAME points and the SAME hard-reject
/// semantics, then returns. A hard reject (`missing` / `type-mismatch`)
/// short-circuits with `total = None`, exactly as the original `return None` did,
/// recording the rejecting term last so the breakdown shows WHY. Soft outcomes
/// (literal-miss -100, uncovered-body -5, absent-ok 0, bonuses) accumulate.
pub fn explain_form_match(form: &FormClause, input: &impl FormMatchInput) -> ScoreExplanation {
    let mut score: i32 = 0;
    let mut terms: Vec<ScoreTerm> = Vec::new();
    macro_rules! ok {
        ($label:expr, $pts:expr, $verdict:expr) => {{
            score += $pts;
            terms.push(ScoreTerm {
                element_label: $label,
                points: $pts,
                verdict: $verdict.into(),
            });
        }};
    }
    macro_rules! reject {
        ($label:expr, $verdict:expr) => {
            return ScoreExplanation {
                total: Some(score.max(0) as u32),
                terms,
            }
            .rejected(ScoreTerm {
                element_label: $label,
                points: 0,
                verdict: $verdict.into(),
            })
        };
    }

    // --- Inline elements ---
    for (elem_idx, elem) in form.inline_elements.iter().enumerate() {
        match elem {
            FormInlineElement::Capture(capture, default) => {
                let is_optional = matches!(
                    capture.modifier,
                    CaptureModifier::Optional | CaptureModifier::ZeroOrMore
                );
                let has_default = default.is_some();
                let present = input.has_value(&capture.var_name);
                let label =
                    capture_label(&capture.var_name, &capture.capture_type, &capture.modifier);

                if !is_optional && !has_default && !present {
                    reject!(label, "missing"); // required capture with no default missing
                }
                if present && !input.is_compatible(&capture.var_name, &capture.capture_type) {
                    reject!(label, "type-mismatch");
                }

                if !is_optional && !has_default && present {
                    ok!(label, 2, "matched");
                } else if (is_optional || has_default) && present {
                    ok!(label, 1, "matched");
                } else {
                    ok!(label, 0, "absent-ok");
                }
            }
            FormInlineElement::Comparison { operator, capture } => {
                let present = input.has_value(&capture.var_name);
                let label = format!(
                    "{} {}",
                    operator,
                    capture_label(&capture.var_name, &capture.capture_type, &capture.modifier)
                );
                if !present {
                    reject!(label, "missing"); // comparison captures are required
                }
                if !input.is_compatible(&capture.var_name, &capture.capture_type) {
                    reject!(label, "type-mismatch");
                }
                ok!(label, 2, "matched");
            }
            FormInlineElement::Literal(text) => {
                let label = format!("literal `{text}`");
                if input.literal_matched(elem_idx) {
                    ok!(label, 3, "matched");
                } else {
                    ok!(label, -100, "literal-miss");
                }
            }
            FormInlineElement::KeywordBlock {
                keyword,
                body_params,
                modifier,
            }
            | FormInlineElement::PseudoSelector {
                name: keyword,
                body_params,
                modifier,
            } => {
                let is_optional = matches!(
                    modifier,
                    CaptureModifier::Optional | CaptureModifier::ZeroOrMore
                );
                for param in body_params {
                    if let Some(cap) = param.capture() {
                        let present = input.has_value(&cap.var_name);
                        let label = format!("{} {{ {} }}", keyword, cap.var_name);
                        if !is_optional && !present {
                            reject!(label, "missing");
                        }
                        if present {
                            ok!(label, 1, "matched");
                        } else {
                            ok!(label, 0, "absent-ok");
                        }
                    }
                }
            }
            FormInlineElement::PseudoClass { name, body_params } => {
                for param in body_params {
                    if let Some(cap) = param.capture() {
                        let present = input.has_value(&cap.var_name);
                        let label = format!(":{} {{ {} }}", name, cap.var_name);
                        if !present {
                            reject!(label, "missing"); // pseudo-class captures are required
                        }
                        ok!(label, 1, "matched");
                    }
                }
            }
            FormInlineElement::Group { elements, modifier } => {
                let group_optional = matches!(
                    modifier,
                    CaptureModifier::Optional | CaptureModifier::ZeroOrMore
                );
                for el in elements {
                    if let FormInlineElement::Capture(capture, default) = el {
                        let is_optional = group_optional
                            || matches!(
                                capture.modifier,
                                CaptureModifier::Optional | CaptureModifier::ZeroOrMore
                            )
                            || default.is_some();
                        let present = input.has_value(&capture.var_name);
                        let label = capture_label(
                            &capture.var_name,
                            &capture.capture_type,
                            &capture.modifier,
                        );
                        if !is_optional && !present {
                            reject!(label, "missing");
                        }
                        if present {
                            ok!(label, 1, "matched");
                        } else {
                            ok!(label, 0, "absent-ok");
                        }
                    }
                }
            }
        }
    }

    // --- Params (parenthesized section) ---
    for param in &form.params {
        if let Some(cap) = param.capture() {
            let is_optional = matches!(
                cap.modifier,
                CaptureModifier::Optional | CaptureModifier::ZeroOrMore
            );
            let has_default = param.default.is_some();
            let present = input.has_value(&cap.var_name);
            let label = format!(
                "param {}: {}",
                param.name,
                capture_label(&cap.var_name, &cap.capture_type, &cap.modifier)
            );

            if !is_optional && !has_default && !present {
                reject!(label, "missing"); // required param without default, missing
            }
            if present && !input.is_compatible(&cap.var_name, &cap.capture_type) {
                reject!(label, "type-mismatch"); // type mismatch in params
            }

            if present {
                ok!(label, 1, "matched");
            } else {
                ok!(label, 0, "absent-ok");
            }
        } else {
            // Multi-element param (e.g., [Literal("when"), Capture($filter:expr)])
            for element in &param.elements {
                if let FormInlineElement::Capture(cap, _) = element {
                    let is_optional = matches!(
                        cap.modifier,
                        CaptureModifier::Optional | CaptureModifier::ZeroOrMore
                    );
                    let has_default = param.default.is_some();
                    let present = input.has_value(&cap.var_name);
                    let label = format!(
                        "param {}: {}",
                        param.name,
                        capture_label(&cap.var_name, &cap.capture_type, &cap.modifier)
                    );

                    if !is_optional && !has_default && !present {
                        reject!(label, "missing"); // required capture in multi-element param, missing
                    }
                    if present && !input.is_compatible(&cap.var_name, &cap.capture_type) {
                        reject!(label, "type-mismatch");
                    }
                    if present {
                        ok!(label, 1, "matched");
                    } else {
                        ok!(label, 0, "absent-ok");
                    }
                }
            }
        }
    }

    // --- Post-arglist inline elements (the `): T` run; PLAN-025 / BUG-045) ---
    for elem in &form.post_arg_inline {
        if let FormInlineElement::Capture(capture, default) = elem {
            let is_optional = matches!(
                capture.modifier,
                CaptureModifier::Optional | CaptureModifier::ZeroOrMore
            );
            let has_default = default.is_some();
            let present = input.has_value(&capture.var_name);
            let label = format!(
                "post-arg {}",
                capture_label(&capture.var_name, &capture.capture_type, &capture.modifier)
            );
            if !is_optional && !has_default && !present {
                reject!(label, "missing");
            }
            if present && !input.is_compatible(&capture.var_name, &capture.capture_type) {
                reject!(label, "type-mismatch");
            }
            if present {
                ok!(label, 1, "matched");
            } else {
                ok!(label, 0, "absent-ok");
            }
        }
        // A literal (e.g. `:`) in the post-arg run is structural and adds no score.
    }

    // --- Body capture disambiguation ---
    if let Some(body_spec) = &form.body_capture {
        let spec = body_spec
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim()
            .trim_start_matches('$');
        let name = spec
            .strip_suffix('*')
            .unwrap_or_else(|| spec.split(':').next().unwrap_or(spec))
            .trim();
        if input.has_value(name) || input.has_body() {
            ok!(format!("body {{ ${name} }}"), 10, "matched");
        } else {
            // The form DECLARES a `{ … }` body but the call provides NONE. Penalize
            // so a bodyless SIBLING form (same surface, no body block) wins cleanly
            // — `@host $a : http(url)` picks `host-http`, not `host-http-opts`. The
            // penalty (-1) only breaks an otherwise-exact tie; a body form whose
            // body IS present still scores +10 above. Mirrors the `uncovered-body`
            // penalty on the inverse case (body provided, no body form).
            ok!(format!("body {{ ${name} }}"), -1, "absent-body");
        }
    } else {
        let inline_names: std::collections::HashSet<&str> = form
            .inline_elements
            .iter()
            .filter_map(|e| {
                if let FormInlineElement::Capture(c, _) = e {
                    Some(c.var_name.as_str())
                } else {
                    None
                }
            })
            .collect();

        let has_uncovered_body =
            input.has_body() && !inline_names.iter().any(|name| input.is_body_value(name));

        if has_uncovered_body {
            ok!("unexpected body".to_string(), -5, "uncovered-body");
        }
    }

    ScoreExplanation {
        total: Some(score.max(0) as u32),
        terms,
    }
}

/// Score a macro definition against an invocation.
/// Handles `form: None` → `Some(0)` for backward compatibility.
pub fn score_macro_form(macro_def: &MacroDefAst, input: &impl FormMatchInput) -> Option<u32> {
    match &macro_def.form {
        Some(form) => score_form_match(form, input),
        None => Some(0),
    }
}

/// Term-by-term explanation of scoring a macro definition (introspection).
/// Mirrors [`score_macro_form`]: a formless macro yields `total: Some(0)` with no
/// terms (it matches anything at score 0).
pub fn explain_macro_form(
    macro_def: &MacroDefAst,
    input: &impl FormMatchInput,
) -> ScoreExplanation {
    match &macro_def.form {
        Some(form) => explain_form_match(form, input),
        None => ScoreExplanation {
            total: Some(0),
            terms: Vec::new(),
        },
    }
}

// =============================================================================
// Type compatibility (moved from expand.rs)
// =============================================================================

/// Check if a CapturedValue is compatible with a CaptureType.
/// Returns false if the value cannot match the expected type.
pub fn is_value_compatible_with_capture(value: &CapturedValue, capture_type: &CaptureType) -> bool {
    match (value, capture_type) {
        // PatternMatch ONLY matches multi-element patterns (pattern_match capture type)
        (CapturedValue::PatternMatch { .. }, CaptureType::String) => false,
        (CapturedValue::PatternMatch { .. }, CaptureType::Ident) => false,
        (CapturedValue::PatternMatch { .. }, CaptureType::Number) => false,
        (CapturedValue::PatternMatch { .. }, CaptureType::Expr) => false,
        (CapturedValue::PatternMatch { .. }, CaptureType::PatternMatch { .. }) => true,
        (CapturedValue::PatternMatch { .. }, _) => false,

        // String can match string, ident (as literal), selector, or expr
        (CapturedValue::String(_), CaptureType::String) => true,
        (CapturedValue::String(_), CaptureType::Ident) => true,
        (CapturedValue::String(_), CaptureType::Selector) => true,
        (CapturedValue::String(_), CaptureType::Expr) => true,
        // String should NOT match numeric types or binding
        (CapturedValue::String(_), CaptureType::Number) => false,
        (CapturedValue::String(_), CaptureType::Time) => false,
        (CapturedValue::String(_), CaptureType::Duration) => false,
        (CapturedValue::String(_), CaptureType::Binding) => false,

        // Binding can match ident or binding
        (CapturedValue::Binding(_), CaptureType::Ident) => true,
        (CapturedValue::Binding(_), CaptureType::Binding) => true,

        // Ident (plain words like "hover", "exist") matches ident, string, or expr
        (CapturedValue::Ident(_), CaptureType::Ident) => true,
        (CapturedValue::Ident(_), CaptureType::String) => true,
        (CapturedValue::Ident(_), CaptureType::Expr) => true,
        // Idents should NOT match numeric types
        (CapturedValue::Ident(_), CaptureType::Number) => false,
        (CapturedValue::Ident(_), CaptureType::Time) => false,
        (CapturedValue::Ident(_), CaptureType::Duration) => false,
        // Idents can match selector (e.g., "window" as a special target)
        (CapturedValue::Ident(_), CaptureType::Selector) => true,
        // Ident with $ prefix (e.g. "$count") should match binding
        (CapturedValue::Ident(s), CaptureType::Binding) => s.starts_with('$'),

        // Numbers match number, time (as milliseconds), or expr
        (CapturedValue::Number(_), CaptureType::Number) => true,
        (CapturedValue::Number(_), CaptureType::Time) => true,
        (CapturedValue::Number(_), CaptureType::Duration) => true,
        (CapturedValue::Number(_), CaptureType::Expr) => true,

        // Expressions match expr
        (CapturedValue::Expr(_), CaptureType::Expr) => true,

        // BUG-264 strict-capture: a bare-number Expr is NOT a valid Duration/Time —
        // `refresh: 5` is a near-miss for `refresh: 5s`, and the ambiguity scorer
        // must not let the productive `:duration` form tie with the error sibling.
        // A binding/other expr (`$userInterval`) stays compatible.
        (CapturedValue::Expr(s), CaptureType::Duration | CaptureType::Time) => {
            let clean = s.trim();
            !(crate::syntax::conversions::parse_duration_with_unit(clean).is_none()
                && clean.parse::<f64>().is_ok())
        }

        // A UNION of literal words states a VOCABULARY: the captured value is
        // compatible only if it IS one of the words. Without this arm the
        // fallthrough (`_ => true`) scored `$kind:("motion")` and
        // `$kind:("value")` identically against a `kind: "motion"` capture, so
        // every kind-is-the-macro family (stdlib/macros/form.st's six @form
        // siblings) looked like an E0923 ambiguous dispatch on every
        // unambiguous declaration.
        (CapturedValue::Ident(s), CaptureType::Union(alts))
        | (CapturedValue::String(s), CaptureType::Union(alts)) => alts.iter().any(|alt| alt == s),

        // StyleProperties and Block match Properties type
        (CapturedValue::StyleProperties(_), CaptureType::Properties) => true,
        (CapturedValue::Block(_), CaptureType::Properties) => true,

        // Block should NOT match ident, string, number, time, or selector
        // (critical for form disambiguation)
        (CapturedValue::Block(_), CaptureType::Ident) => false,
        (CapturedValue::Block(_), CaptureType::String) => false,
        (CapturedValue::Block(_), CaptureType::Number) => false,
        (CapturedValue::Block(_), CaptureType::Time) => false,
        (CapturedValue::Block(_), CaptureType::Selector) => false,

        // For other combinations, be permissive
        _ => true,
    }
}

// =============================================================================
// CaptureMapInput — adapter for pipeline resolve
// =============================================================================

/// Adapter for pipeline resolve layer (HashMap<String, CapturedValue>).
pub struct CaptureMapInput<'a> {
    pub captures: &'a HashMap<String, CapturedValue>,
}

impl FormMatchInput for CaptureMapInput<'_> {
    fn has_value(&self, name: &str) -> bool {
        self.captures.contains_key(name)
    }

    fn is_compatible(&self, name: &str, capture_type: &CaptureType) -> bool {
        let Some(value) = self.captures.get(name) else {
            return true; // absence != type mismatch
        };
        // Block/Properties/Keyframes should NOT match simple scalar captures
        match value {
            CapturedValue::Block(_)
            | CapturedValue::Properties(_)
            | CapturedValue::Keyframes(_) => !matches!(
                capture_type,
                CaptureType::Ident | CaptureType::String | CaptureType::Number | CaptureType::Time
            ),
            // Every other value answers through the CANONICAL compatibility
            // table (is_value_compatible_with_capture) — not a local `_ =>
            // true`. The lenient shim let `$kind:("motion")` and
            // `$kind:("value")` both "score" against a `kind: "motion"`
            // capture, so every kind-is-the-macro dispatch (form.st's six
            // @form siblings) warned E0923 on an unambiguous call.
            _ => is_value_compatible_with_capture(value, capture_type),
        }
    }

    fn is_body_value(&self, name: &str) -> bool {
        matches!(
            self.captures.get(name),
            Some(
                CapturedValue::Block(_)
                    | CapturedValue::Properties(_)
                    | CapturedValue::Keyframes(_)
            )
        )
    }

    fn has_body(&self) -> bool {
        self.captures.values().any(|v| {
            matches!(
                v,
                CapturedValue::Block(_)
                    | CapturedValue::Properties(_)
                    | CapturedValue::Keyframes(_)
            )
        })
    }
}

/// Introspection-only input (FEAT-087): a `CaptureMapInput` that ALSO knows the
/// raw call text, so it can answer `literal_matched` honestly. The compiler's
/// hot path uses positional matching (PatternArgInput) or the default-true
/// CaptureMapInput; a probe parses a bare call string and has no positional
/// literal tracking, so without this every form's literals would score +3 and a
/// losing candidate's breakdown would lie ("literal `inline` +3" for a call that
/// never typed `inline`). Here a form literal scores as matched ONLY when its
/// token appears in the call text — a faithful, readable resolution table.
pub struct ProbeInput<'a> {
    pub captures: &'a HashMap<String, CapturedValue>,
    /// The form whose literals we are scoring (index → literal text).
    pub literal_texts: Vec<(usize, String)>,
    /// Lowercased call text, for substring presence checks.
    pub call_lc: String,
}

impl<'a> ProbeInput<'a> {
    /// Build for a specific candidate form: collect its inline-element literal
    /// texts by index so `literal_matched` can check each against the call.
    pub fn new(
        captures: &'a HashMap<String, CapturedValue>,
        form: &FormClause,
        call: &str,
    ) -> Self {
        let literal_texts = form
            .inline_elements
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e {
                FormInlineElement::Literal(s) => Some((i, s.clone())),
                _ => None,
            })
            .collect();
        ProbeInput {
            captures,
            literal_texts,
            call_lc: call.to_lowercase(),
        }
    }
    fn inner(&self) -> CaptureMapInput<'_> {
        CaptureMapInput {
            captures: self.captures,
        }
    }
}

impl FormMatchInput for ProbeInput<'_> {
    fn has_value(&self, name: &str) -> bool {
        self.inner().has_value(name)
    }
    fn is_compatible(&self, name: &str, capture_type: &CaptureType) -> bool {
        self.inner().is_compatible(name, capture_type)
    }
    fn is_body_value(&self, name: &str) -> bool {
        self.inner().is_body_value(name)
    }
    fn has_body(&self) -> bool {
        self.inner().has_body()
    }
    fn literal_matched(&self, inline_element_index: usize) -> bool {
        // A purely-structural literal (`:`, `;`, `<-`) is always considered present
        // — it is punctuation the parser consumed, not a discriminating keyword.
        // A word-like literal (a keyword such as `fetch`, `inline`) counts as matched
        // ONLY when it appears in the call text.
        match self
            .literal_texts
            .iter()
            .find(|(i, _)| *i == inline_element_index)
        {
            Some((_, text)) => {
                let is_keyword = text.chars().any(|c| c.is_alphanumeric());
                if !is_keyword {
                    return true;
                }
                self.call_lc.contains(&text.to_lowercase())
            }
            None => true,
        }
    }
}

// =============================================================================
// PatternArgInput — adapter for metasystem expand/compile
// =============================================================================

/// Adapter for expand/compile layers (positional + named PatternArg).
///
/// Takes the candidate FormClause to map positional args to capture names.
pub struct PatternArgInput<'a> {
    /// Positional args mapped to capture names from the form being scored
    named_map: HashMap<&'a str, &'a CapturedValue>,
    /// Whether any arg is a body-like value
    has_body_arg: bool,
    /// Tracks which literal indices in inline_elements had matching positional args
    matched_literal_indices: HashSet<usize>,
}

impl<'a> PatternArgInput<'a> {
    pub fn new(form: &'a FormClause, args: &'a [PatternArg]) -> Self {
        let mut named_map = HashMap::new();
        let mut has_body_arg = false;
        let mut matched_literal_indices = HashSet::new();

        // Collect positional args (those with empty name or __pos/__alias__ prefix)
        let positional_args: Vec<&'a PatternArg> = args
            .iter()
            .filter(|arg| {
                arg.name.is_empty()
                    || arg.name.starts_with("__pos")
                    || arg.name.starts_with("__alias__")
            })
            .collect();

        let mut positional_idx = 0;

        // Walk inline elements in order, mirroring bind_args logic.
        // Literals consume matching positional args; captures map the next positional.
        for (elem_idx, elem) in form.inline_elements.iter().enumerate() {
            match elem {
                FormInlineElement::Literal(expected) => {
                    // Consume the matching positional arg (like bind_args does)
                    if positional_idx < positional_args.len() {
                        let arg_text = match &positional_args[positional_idx].value {
                            CapturedValue::Ident(s) | CapturedValue::String(s) => s.as_str(),
                            _ => "",
                        };
                        if arg_text == expected {
                            positional_idx += 1;
                            matched_literal_indices.insert(elem_idx);
                        }
                    }
                }
                FormInlineElement::Capture(c, _)
                | FormInlineElement::Comparison { capture: c, .. } => {
                    if positional_idx < positional_args.len() {
                        named_map
                            .insert(c.var_name.as_str(), &positional_args[positional_idx].value);
                        positional_idx += 1;
                    }
                }
                FormInlineElement::KeywordBlock { body_params, .. }
                | FormInlineElement::PseudoSelector { body_params, .. }
                | FormInlineElement::PseudoClass { body_params, .. } => {
                    for param in body_params {
                        if let Some(cap) = param.capture()
                            && positional_idx < positional_args.len()
                        {
                            named_map.insert(
                                cap.var_name.as_str(),
                                &positional_args[positional_idx].value,
                            );
                            positional_idx += 1;
                        }
                    }
                }
                FormInlineElement::Group { elements, .. } => {
                    // A grouped clause maps its captures in order, mirroring
                    // how the matcher tries them as a unit.
                    for el in elements {
                        match el {
                            FormInlineElement::Literal(expected) => {
                                if positional_idx < positional_args.len() {
                                    let arg_text = match &positional_args[positional_idx].value {
                                        CapturedValue::Ident(s) | CapturedValue::String(s) => {
                                            s.as_str()
                                        }
                                        _ => "",
                                    };
                                    if arg_text == expected {
                                        positional_idx += 1;
                                        matched_literal_indices.insert(elem_idx);
                                    }
                                }
                            }
                            FormInlineElement::Capture(c, _)
                            | FormInlineElement::Comparison { capture: c, .. } => {
                                if positional_idx < positional_args.len() {
                                    named_map.insert(
                                        c.var_name.as_str(),
                                        &positional_args[positional_idx].value,
                                    );
                                    positional_idx += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Collect capture names from params in order
        let param_names: Vec<&str> = form
            .params
            .iter()
            .filter_map(|p| p.capture().map(|c| c.var_name.as_str()))
            .collect();

        // Continue with remaining positional args for parenthesized params
        for param_name in &param_names {
            if positional_idx < positional_args.len() {
                named_map.insert(*param_name, &positional_args[positional_idx].value);
                positional_idx += 1;
            }
        }

        // Build param name → capture var_name mapping so named args using the
        // param's external name (e.g., "on") are also accessible via the capture's
        // internal var_name (e.g., "trigger") which the scorer checks.
        let param_name_to_capture: HashMap<&str, &str> = form
            .params
            .iter()
            .filter_map(|p| {
                let cap = p.capture()?;
                // Only create mapping when param name differs from capture var_name
                if p.name != cap.var_name {
                    Some((p.name.as_str(), cap.var_name.as_str()))
                } else {
                    None
                }
            })
            .collect();

        // Process named args (overwrite any positional mapping)
        for arg in args {
            if !arg.name.is_empty()
                && !arg.name.starts_with("__pos")
                && !arg.name.starts_with("__alias__")
            {
                named_map.insert(arg.name.as_str(), &arg.value);
                // Also insert under the capture var_name if the arg name is a param name
                if let Some(&capture_name) = param_name_to_capture.get(arg.name.as_str()) {
                    named_map.insert(capture_name, &arg.value);
                }
            }

            if arg.value.is_body_like() {
                has_body_arg = true;
            }
        }

        Self {
            named_map,
            has_body_arg,
            matched_literal_indices,
        }
    }
}

impl FormMatchInput for PatternArgInput<'_> {
    fn has_value(&self, name: &str) -> bool {
        self.named_map.contains_key(name)
    }

    fn is_compatible(&self, name: &str, capture_type: &CaptureType) -> bool {
        let Some(value) = self.named_map.get(name) else {
            return true; // absence != type mismatch
        };
        is_value_compatible_with_capture(value, capture_type)
    }

    fn is_body_value(&self, name: &str) -> bool {
        self.named_map.get(name).is_some_and(|v| v.is_body_like())
    }

    fn has_body(&self) -> bool {
        self.has_body_arg
    }

    fn literal_matched(&self, inline_element_index: usize) -> bool {
        self.matched_literal_indices.contains(&inline_element_index)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::meta_ast::{FormCapture, FormParam, ParamDefault};

    // =========================================================================
    // Mock FormMatchInput for pure scoring tests
    // =========================================================================

    struct MockInput {
        values: HashMap<String, MockValue>,
        body: bool,
    }

    #[derive(Clone)]
    #[allow(dead_code)]
    enum MockValue {
        Ident,
        Number,
        String,
        Block,
        Properties,
        Keyframes,
        Expr,
        Preset,
    }

    impl MockInput {
        fn new() -> Self {
            Self {
                values: HashMap::new(),
                body: false,
            }
        }

        fn with(mut self, name: &str, val: MockValue) -> Self {
            self.values.insert(name.to_string(), val);
            self
        }

        fn with_body(mut self) -> Self {
            self.body = true;
            self
        }
    }

    impl FormMatchInput for MockInput {
        fn has_value(&self, name: &str) -> bool {
            self.values.contains_key(name)
        }

        fn is_compatible(&self, name: &str, capture_type: &CaptureType) -> bool {
            let Some(val) = self.values.get(name) else {
                return true;
            };
            match val {
                MockValue::Block | MockValue::Properties | MockValue::Keyframes => !matches!(
                    capture_type,
                    CaptureType::Ident
                        | CaptureType::String
                        | CaptureType::Number
                        | CaptureType::Time
                ),
                MockValue::Number => matches!(
                    capture_type,
                    CaptureType::Number
                        | CaptureType::Time
                        | CaptureType::Duration
                        | CaptureType::Expr
                ),
                MockValue::Preset => matches!(capture_type, CaptureType::Preset),
                _ => true,
            }
        }

        fn is_body_value(&self, name: &str) -> bool {
            matches!(
                self.values.get(name),
                Some(MockValue::Block | MockValue::Properties | MockValue::Keyframes)
            )
        }

        fn has_body(&self) -> bool {
            self.body
        }
    }

    // =========================================================================
    // Helpers to build FormClause values
    // =========================================================================

    fn capture(name: &str, ct: CaptureType) -> FormCapture {
        FormCapture {
            var_name: name.to_string(),
            capture_type: ct,
            modifier: CaptureModifier::Required,
            alias_capture: None,
        }
    }

    fn optional_capture(name: &str, ct: CaptureType) -> FormCapture {
        FormCapture {
            var_name: name.to_string(),
            capture_type: ct,
            modifier: CaptureModifier::Optional,
            alias_capture: None,
        }
    }

    fn inline_cap(name: &str, ct: CaptureType) -> FormInlineElement {
        FormInlineElement::Capture(capture(name, ct), None)
    }

    fn inline_opt(name: &str, ct: CaptureType) -> FormInlineElement {
        FormInlineElement::Capture(optional_capture(name, ct), None)
    }

    fn literal(s: &str) -> FormInlineElement {
        FormInlineElement::Literal(s.to_string())
    }

    fn form(
        inline: Vec<FormInlineElement>,
        params: Vec<FormParam>,
        body_capture: Option<&str>,
    ) -> FormClause {
        FormClause {
            directive_name: "test".to_string(),
            inline_elements: inline,
            params,
            post_arg_inline: vec![],
            body_capture: body_capture.map(|s| s.to_string()),
            body_params: vec![],
            body_groups: Vec::new(),
            span: Default::default(),
        }
    }

    fn simple_param(name: &str, cap_name: &str, ct: CaptureType) -> FormParam {
        FormParam {
            name: name.to_string(),
            elements: vec![FormInlineElement::Capture(capture(cap_name, ct), None)],
            default: None,
        }
    }

    fn param_with_default(name: &str, cap_name: &str, ct: CaptureType) -> FormParam {
        FormParam {
            name: name.to_string(),
            elements: vec![FormInlineElement::Capture(capture(cap_name, ct), None)],
            default: Some(ParamDefault::String("default".to_string())),
        }
    }

    #[allow(dead_code)]
    fn optional_param(name: &str, cap_name: &str, ct: CaptureType) -> FormParam {
        FormParam {
            name: name.to_string(),
            elements: vec![FormInlineElement::Capture(
                optional_capture(cap_name, ct),
                None,
            )],
            default: None,
        }
    }

    // =========================================================================
    // Group 1: Core scoring algorithm
    // =========================================================================

    #[test]
    fn test_1_1_required_inline_capture_missing() {
        let f = form(vec![inline_cap("name", CaptureType::Ident)], vec![], None);
        let input = MockInput::new();
        assert_eq!(score_form_match(&f, &input), None);
    }

    #[test]
    fn test_1_2_required_inline_capture_present() {
        let f = form(vec![inline_cap("name", CaptureType::Ident)], vec![], None);
        let input = MockInput::new().with("name", MockValue::Ident);
        assert_eq!(score_form_match(&f, &input), Some(2));
    }

    #[test]
    fn test_1_3_optional_inline_capture_missing() {
        let f = form(vec![inline_opt("name", CaptureType::Ident)], vec![], None);
        let input = MockInput::new();
        assert_eq!(score_form_match(&f, &input), Some(0));
    }

    #[test]
    fn test_1_4_optional_inline_capture_present() {
        let f = form(vec![inline_opt("name", CaptureType::Ident)], vec![], None);
        let input = MockInput::new().with("name", MockValue::Ident);
        assert_eq!(score_form_match(&f, &input), Some(1));
    }

    #[test]
    fn test_1_5_multiple_captures_mixed() {
        let f = form(
            vec![
                inline_cap("event", CaptureType::Ident),
                inline_opt("name", CaptureType::Ident),
            ],
            vec![],
            None,
        );
        let input = MockInput::new().with("event", MockValue::Ident);
        assert_eq!(score_form_match(&f, &input), Some(2));
    }

    #[test]
    fn test_1_6_multiple_captures_all_present() {
        let f = form(
            vec![
                inline_cap("event", CaptureType::Ident),
                inline_opt("name", CaptureType::Ident),
            ],
            vec![],
            None,
        );
        let input = MockInput::new()
            .with("event", MockValue::Ident)
            .with("name", MockValue::Ident);
        assert_eq!(score_form_match(&f, &input), Some(3));
    }

    #[test]
    fn test_1_7_type_mismatch_rejects() {
        let f = form(vec![inline_cap("name", CaptureType::Ident)], vec![], None);
        let input = MockInput::new().with("name", MockValue::Block);
        assert_eq!(score_form_match(&f, &input), None);
    }

    #[test]
    fn test_1_8_type_compatible_passes() {
        let f = form(vec![inline_cap("name", CaptureType::Ident)], vec![], None);
        let input = MockInput::new().with("name", MockValue::Ident);
        assert_eq!(score_form_match(&f, &input), Some(2));
    }

    #[test]
    fn test_1_9_literal_adds_to_score() {
        let f = form(
            vec![
                inline_cap("a", CaptureType::Ident),
                literal(":"),
                inline_cap("b", CaptureType::Ident),
            ],
            vec![],
            None,
        );
        let input = MockInput::new()
            .with("a", MockValue::Ident)
            .with("b", MockValue::Ident);
        // 2 (a:ident) + 3 (literal ":") + 2 (b:ident) = 7
        assert_eq!(score_form_match(&f, &input), Some(7));
    }

    #[test]
    fn test_1_10_comparison_capture_required() {
        let f = form(
            vec![FormInlineElement::Comparison {
                operator: ">=".to_string(),
                capture: capture("count", CaptureType::Number),
            }],
            vec![],
            None,
        );
        let input = MockInput::new();
        assert_eq!(score_form_match(&f, &input), None);
    }

    #[test]
    fn test_1_11_comparison_capture_present() {
        let f = form(
            vec![FormInlineElement::Comparison {
                operator: ">=".to_string(),
                capture: capture("count", CaptureType::Number),
            }],
            vec![],
            None,
        );
        let input = MockInput::new().with("count", MockValue::Number);
        assert_eq!(score_form_match(&f, &input), Some(2));
    }

    // =========================================================================
    // Group 2: Param scoring
    // =========================================================================

    #[test]
    fn test_2_1_named_param_present() {
        let f = form(
            vec![],
            vec![simple_param("color", "color", CaptureType::Ident)],
            None,
        );
        let input = MockInput::new().with("color", MockValue::Ident);
        assert_eq!(score_form_match(&f, &input), Some(1));
    }

    #[test]
    fn test_2_2_named_param_missing_with_default() {
        let f = form(
            vec![],
            vec![param_with_default("color", "color", CaptureType::Ident)],
            None,
        );
        let input = MockInput::new();
        // has default → not rejected
        assert!(score_form_match(&f, &input).is_some());
    }

    #[test]
    fn test_2_3_named_param_missing_required() {
        let f = form(
            vec![],
            vec![simple_param("color", "color", CaptureType::Ident)],
            None,
        );
        let input = MockInput::new();
        assert_eq!(score_form_match(&f, &input), None);
    }

    #[test]
    fn test_2_4_named_param_type_mismatch() {
        let f = form(
            vec![],
            vec![simple_param("color", "color", CaptureType::Ident)],
            None,
        );
        let input = MockInput::new().with("color", MockValue::Block);
        assert_eq!(score_form_match(&f, &input), None);
    }

    #[test]
    fn test_2_5_multiple_params_partial() {
        let f = form(
            vec![],
            vec![
                simple_param("required", "required", CaptureType::Ident),
                param_with_default("opt1", "opt1", CaptureType::Ident),
                param_with_default("opt2", "opt2", CaptureType::Ident),
            ],
            None,
        );
        let input = MockInput::new().with("required", MockValue::Ident);
        let score = score_form_match(&f, &input);
        assert!(score.is_some());
        assert_eq!(score.unwrap(), 1); // only required matched
    }

    #[test]
    fn test_2_6_multiple_params_all_present() {
        let f = form(
            vec![],
            vec![
                simple_param("required", "required", CaptureType::Ident),
                param_with_default("opt1", "opt1", CaptureType::Ident),
                param_with_default("opt2", "opt2", CaptureType::Ident),
            ],
            None,
        );
        let input = MockInput::new()
            .with("required", MockValue::Ident)
            .with("opt1", MockValue::Ident)
            .with("opt2", MockValue::Ident);
        let score = score_form_match(&f, &input);
        assert!(score.is_some());
        assert_eq!(score.unwrap(), 3); // all three matched
    }

    // =========================================================================
    // Group 3: Body capture disambiguation
    // =========================================================================

    #[test]
    fn test_3_1_body_capture_match_bonus() {
        let f = form(vec![], vec![], Some("$children*"));
        let input = MockInput::new().with("children", MockValue::Block);
        let score = score_form_match(&f, &input).unwrap();
        assert!(score >= 10);
    }

    #[test]
    fn test_3_2_body_capture_via_has_body() {
        let f = form(vec![], vec![], Some("$children*"));
        let input = MockInput::new().with_body();
        let score = score_form_match(&f, &input).unwrap();
        assert!(score >= 10);
    }

    #[test]
    fn test_3_3_no_body_capture_no_body_ok() {
        let f = form(vec![inline_cap("name", CaptureType::Ident)], vec![], None);
        let input = MockInput::new().with("name", MockValue::Ident);
        // no penalty
        assert_eq!(score_form_match(&f, &input), Some(2));
    }

    #[test]
    fn test_3_4_no_body_capture_body_present_penalty() {
        let f = form(vec![inline_cap("name", CaptureType::Ident)], vec![], None);
        let input = MockInput::new().with("name", MockValue::Ident).with_body();
        let score = score_form_match(&f, &input).unwrap();
        // 2 for name, -5 for uncovered body → clamped to 0
        assert!(score < 2);
    }

    #[test]
    fn test_3_5_typed_body_capture() {
        let f = form(vec![], vec![], Some("$keyframes:keyframes"));
        let input = MockInput::new().with("keyframes", MockValue::Keyframes);
        let score = score_form_match(&f, &input).unwrap();
        assert!(score >= 10);
    }

    // =========================================================================
    // Group 4: Disambiguation — same directive, different forms
    // =========================================================================

    #[test]
    fn test_4_1_glow_vs_glow_with_children() {
        // Form A: @fill glow(color, radius)
        let form_a = form(
            vec![],
            vec![
                simple_param("color", "color", CaptureType::Ident),
                simple_param("radius", "radius", CaptureType::Number),
            ],
            None,
        );
        // Form B: @fill glow(color, radius) { $children* }
        let form_b = form(
            vec![],
            vec![
                simple_param("color", "color", CaptureType::Ident),
                simple_param("radius", "radius", CaptureType::Number),
            ],
            Some("$children*"),
        );

        let input = MockInput::new()
            .with("color", MockValue::Ident)
            .with("radius", MockValue::Number)
            .with("children", MockValue::Block)
            .with_body();

        let score_a = score_form_match(&form_a, &input);
        let score_b = score_form_match(&form_b, &input);
        assert!(score_b.unwrap() > score_a.unwrap());
    }

    #[test]
    fn test_4_2_glow_vs_glow_with_children_no_body() {
        let form_a = form(
            vec![],
            vec![
                simple_param("color", "color", CaptureType::Ident),
                simple_param("radius", "radius", CaptureType::Number),
            ],
            None,
        );
        let form_b = form(
            vec![],
            vec![
                simple_param("color", "color", CaptureType::Ident),
                simple_param("radius", "radius", CaptureType::Number),
            ],
            Some("$children*"),
        );

        let input = MockInput::new()
            .with("color", MockValue::Ident)
            .with("radius", MockValue::Number);

        let score_a = score_form_match(&form_a, &input).unwrap();
        let score_b = score_form_match(&form_b, &input).unwrap();
        // A should score at least as well (no body penalty)
        assert!(score_a >= score_b);
    }

    #[test]
    fn test_4_3_on_animation_vs_on_mutation() {
        // Form A: @on $event:ident $name:ident(...)
        let form_a = form(
            vec![
                inline_cap("event", CaptureType::Ident),
                inline_cap("name", CaptureType::Ident),
            ],
            vec![],
            None,
        );
        // Form B: @on $event:ident { $body:mutation_actions }
        let form_b = form(
            vec![inline_cap("event", CaptureType::Ident)],
            vec![],
            Some("$body:mutation_actions"),
        );

        // Input: has event + body, no name → A should be None, B should match
        let input = MockInput::new().with("event", MockValue::Ident).with_body();

        assert_eq!(score_form_match(&form_a, &input), None); // "name" missing
        assert!(score_form_match(&form_b, &input).is_some());
    }

    #[test]
    fn test_4_4_on_animation_wins_with_name() {
        let form_a = form(
            vec![
                inline_cap("event", CaptureType::Ident),
                inline_cap("name", CaptureType::Ident),
            ],
            vec![],
            Some("$keyframes:keyframes"),
        );
        let form_b = form(
            vec![inline_cap("event", CaptureType::Ident)],
            vec![],
            Some("$body:mutation_actions"),
        );

        let input = MockInput::new()
            .with("event", MockValue::Ident)
            .with("name", MockValue::Ident)
            .with("keyframes", MockValue::Keyframes)
            .with_body();

        let score_a = score_form_match(&form_a, &input).unwrap();
        let score_b = score_form_match(&form_b, &input).unwrap();
        assert!(score_a > score_b);
    }

    #[test]
    fn test_4_5_generic_fill_vs_specific_glow() {
        // Form A: @fill $effect:ident (generic — 1 inline capture)
        let form_a = form(vec![inline_cap("effect", CaptureType::Ident)], vec![], None);
        // Form B: @fill glow(color, radius, intensity) (specific — 3 params)
        let form_b = form(
            vec![],
            vec![
                simple_param("color", "color", CaptureType::Ident),
                simple_param("radius", "radius", CaptureType::Number),
                simple_param("intensity", "intensity", CaptureType::Number),
            ],
            None,
        );

        let input = MockInput::new()
            .with("effect", MockValue::Ident)
            .with("color", MockValue::Ident)
            .with("radius", MockValue::Number)
            .with("intensity", MockValue::Number);

        let score_a = score_form_match(&form_a, &input).unwrap();
        let score_b = score_form_match(&form_b, &input).unwrap();
        assert!(score_b > score_a); // 3 params (3pts) > 1 inline (2pts)
    }

    #[test]
    fn test_4_6_three_way_disambiguation() {
        // Animation form: @on $event:ident $name:ident?(...)
        let form_anim = form(
            vec![
                inline_cap("event", CaptureType::Ident),
                inline_opt("name", CaptureType::Ident),
            ],
            vec![],
            Some("$keyframes:keyframes"),
        );
        // Mutation form: @on $event:ident { $body:mutation_actions }
        let form_mutation = form(
            vec![inline_cap("event", CaptureType::Ident)],
            vec![],
            Some("$body:mutation_actions"),
        );
        // Visible form: same as animation but with "visible" event (no structural diff here)
        // We test that animation-type input picks animation over mutation
        let input = MockInput::new()
            .with("event", MockValue::Ident)
            .with("name", MockValue::Ident)
            .with("keyframes", MockValue::Keyframes)
            .with_body();

        let score_anim = score_form_match(&form_anim, &input).unwrap();
        let score_mutation = score_form_match(&form_mutation, &input).unwrap();
        assert!(score_anim > score_mutation);
    }

    #[test]
    fn test_4_7_specialized_assertion_outscores_generic_then() {
        // Generic form: @then $selector:selector should $assertion:ident $expected:expr?
        let form_generic = form(
            vec![
                inline_cap("selector", CaptureType::Selector),
                literal("should"),
                inline_cap("assertion", CaptureType::Ident),
                inline_opt("expected", CaptureType::Expr),
            ],
            vec![],
            None,
        );
        // Specialized form: @then $selector:selector should have_timeline $name:string
        let form_specialized = form(
            vec![
                inline_cap("selector", CaptureType::Selector),
                literal("should"),
                literal("have_timeline"),
                inline_cap("name", CaptureType::String),
            ],
            vec![],
            None,
        );

        let input = MockInput::new()
            .with("selector", MockValue::String)
            .with("assertion", MockValue::Ident)
            .with("expected", MockValue::String)
            .with("name", MockValue::String);

        let score_generic = score_form_match(&form_generic, &input).unwrap();
        let score_specialized = score_form_match(&form_specialized, &input).unwrap();

        // Generic: selector(+2) + "should"(+3) + assertion(+2) + expected?(+1) = 8
        assert_eq!(score_generic, 8);
        // Specialized: selector(+2) + "should"(+3) + "have_timeline"(+3) + name(+2) = 10
        assert_eq!(score_specialized, 10);
        // Specialized MUST win
        assert!(score_specialized > score_generic);
    }

    // =========================================================================
    // Group 5: score_macro_form wrapper
    // =========================================================================

    #[test]
    fn test_5_1_formless_macro_returns_zero() {
        let m = MacroDefAst {
            form: None,
            ..Default::default()
        };
        let input = MockInput::new();
        assert_eq!(score_macro_form(&m, &input), Some(0));
    }

    #[test]
    fn test_5_2_form_macro_delegates() {
        let f = form(vec![inline_cap("name", CaptureType::Ident)], vec![], None);
        let m = MacroDefAst {
            form: Some(f.clone()),
            ..Default::default()
        };
        let input = MockInput::new().with("name", MockValue::Ident);
        assert_eq!(score_macro_form(&m, &input), score_form_match(&f, &input));
    }

    // =========================================================================
    // Group 6: CaptureMapInput adapter
    // =========================================================================

    #[test]
    fn test_6_1_has_value_finds_key() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), CapturedValue::Ident("foo".to_string()));
        let input = CaptureMapInput { captures: &map };
        assert!(input.has_value("name"));
    }

    #[test]
    fn test_6_2_has_value_missing() {
        let map = HashMap::new();
        let input = CaptureMapInput { captures: &map };
        assert!(!input.has_value("name"));
    }

    #[test]
    fn test_6_3_is_compatible_block_vs_ident() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), CapturedValue::Block(vec![]));
        let input = CaptureMapInput { captures: &map };
        assert!(!input.is_compatible("name", &CaptureType::Ident));
    }

    #[test]
    fn test_6_4_is_compatible_ident_vs_ident() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), CapturedValue::Ident("x".to_string()));
        let input = CaptureMapInput { captures: &map };
        assert!(input.is_compatible("name", &CaptureType::Ident));
    }

    #[test]
    fn test_6_5_is_compatible_absent_returns_true() {
        let map = HashMap::new();
        let input = CaptureMapInput { captures: &map };
        assert!(input.is_compatible("name", &CaptureType::Ident));
    }

    #[test]
    fn test_6_6_is_body_value_block() {
        let mut map = HashMap::new();
        map.insert("children".to_string(), CapturedValue::Block(vec![]));
        let input = CaptureMapInput { captures: &map };
        assert!(input.is_body_value("children"));
    }

    #[test]
    fn test_6_7_is_body_value_ident() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), CapturedValue::Ident("x".to_string()));
        let input = CaptureMapInput { captures: &map };
        assert!(!input.is_body_value("name"));
    }

    #[test]
    fn test_6_8_has_body_with_block() {
        let mut map = HashMap::new();
        map.insert("children".to_string(), CapturedValue::Block(vec![]));
        let input = CaptureMapInput { captures: &map };
        assert!(input.has_body());
    }

    #[test]
    fn test_6_9_has_body_no_block() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), CapturedValue::Ident("x".to_string()));
        let input = CaptureMapInput { captures: &map };
        assert!(!input.has_body());
    }

    #[test]
    fn test_6_10_full_scoring_matches_old_behavior() {
        // Replicate glow-with-children test from resolve.rs
        // Form A: glow(color, radius) — no body
        let form_a = form(
            vec![],
            vec![
                simple_param("color", "color", CaptureType::Ident),
                simple_param("radius", "radius", CaptureType::Number),
            ],
            None,
        );
        // Form B: glow(color, radius) { $children* }
        let form_b = form(
            vec![],
            vec![
                simple_param("color", "color", CaptureType::Ident),
                simple_param("radius", "radius", CaptureType::Number),
            ],
            Some("$children*"),
        );

        let mut captures = HashMap::new();
        captures.insert(
            "color".to_string(),
            CapturedValue::Ident("cyan".to_string()),
        );
        captures.insert("radius".to_string(), CapturedValue::Number(10.0));
        captures.insert("children".to_string(), CapturedValue::Block(vec![]));

        let input = CaptureMapInput {
            captures: &captures,
        };

        let macro_a = MacroDefAst {
            form: Some(form_a),
            ..Default::default()
        };
        let macro_b = MacroDefAst {
            form: Some(form_b),
            ..Default::default()
        };

        let score_a = score_macro_form(&macro_a, &input);
        let score_b = score_macro_form(&macro_b, &input);

        // B (with body_capture) should win when children present
        assert!(score_b.unwrap() > score_a.unwrap());
    }

    // =========================================================================
    // Group 7: PatternArgInput adapter
    // =========================================================================

    #[test]
    fn test_7_1_positional_args_mapped_to_names() {
        let f = form(
            vec![
                inline_cap("event", CaptureType::Ident),
                inline_cap("name", CaptureType::Ident),
            ],
            vec![],
            None,
        );
        let args = vec![
            PatternArg {
                name: "".to_string(),
                value: CapturedValue::Ident("click".to_string()),
            },
            PatternArg {
                name: "".to_string(),
                value: CapturedValue::Ident("fade".to_string()),
            },
        ];
        let input = PatternArgInput::new(&f, &args);
        assert!(input.has_value("event"));
        assert!(input.has_value("name"));
    }

    #[test]
    fn test_7_2_named_args_found_directly() {
        let f = form(vec![], vec![], None);
        let args = vec![PatternArg {
            name: "color".to_string(),
            value: CapturedValue::Ident("cyan".to_string()),
        }];
        let input = PatternArgInput::new(&f, &args);
        assert!(input.has_value("color"));
    }

    #[test]
    fn test_7_3_mixed_positional_and_named() {
        let f = form(vec![inline_cap("event", CaptureType::Ident)], vec![], None);
        let args = vec![
            PatternArg {
                name: "".to_string(),
                value: CapturedValue::Ident("click".to_string()),
            },
            PatternArg {
                name: "duration".to_string(),
                value: CapturedValue::Number(600.0),
            },
        ];
        let input = PatternArgInput::new(&f, &args);
        assert!(input.has_value("event"));
        assert!(input.has_value("duration"));
    }

    #[test]
    fn test_7_4_type_compat_delegates_to_expand_fn() {
        let f = form(vec![inline_cap("name", CaptureType::Ident)], vec![], None);
        // Block (body) vs Ident should be incompatible (critical for form disambiguation)
        let args = vec![PatternArg {
            name: "".to_string(),
            value: CapturedValue::Block(vec![]),
        }];
        let input = PatternArgInput::new(&f, &args);
        assert!(!input.is_compatible("name", &CaptureType::Ident));
    }

    #[test]
    fn test_7_5_has_body_from_block_arg() {
        let f = form(vec![], vec![], None);
        let args = vec![PatternArg {
            name: "body".to_string(),
            value: CapturedValue::Block(vec![]),
        }];
        let input = PatternArgInput::new(&f, &args);
        assert!(input.has_body());
    }

    #[test]
    fn test_7_6_has_body_from_properties_arg() {
        let f = form(vec![], vec![], None);
        let args = vec![PatternArg {
            name: "styles".to_string(),
            value: CapturedValue::StyleProperties(vec![("color".to_string(), "red".to_string())]),
        }];
        let input = PatternArgInput::new(&f, &args);
        assert!(input.has_body());
    }

    #[test]
    fn test_7_7_no_body_with_scalar_args() {
        let f = form(vec![], vec![], None);
        let args = vec![
            PatternArg {
                name: "name".to_string(),
                value: CapturedValue::Ident("foo".to_string()),
            },
            PatternArg {
                name: "count".to_string(),
                value: CapturedValue::Number(5.0),
            },
        ];
        let input = PatternArgInput::new(&f, &args);
        assert!(!input.has_body());
    }

    // =========================================================================
    // Group 9: Cross-adapter consistency
    // =========================================================================

    #[test]
    fn test_9_1_all_adapters_same_winner() {
        // Two forms: glow (no body) vs glow-with-children (body)
        let form_a = form(
            vec![],
            vec![simple_param("color", "color", CaptureType::Ident)],
            None,
        );
        let form_b = form(
            vec![],
            vec![simple_param("color", "color", CaptureType::Ident)],
            Some("$children*"),
        );

        // CaptureMapInput — with body
        let mut captures = HashMap::new();
        captures.insert(
            "color".to_string(),
            CapturedValue::Ident("cyan".to_string()),
        );
        captures.insert("children".to_string(), CapturedValue::Block(vec![]));
        let cap_input = CaptureMapInput {
            captures: &captures,
        };
        let cap_a = score_form_match(&form_a, &cap_input);
        let cap_b = score_form_match(&form_b, &cap_input);
        assert!(
            cap_b.unwrap() > cap_a.unwrap(),
            "CaptureMapInput: B should win with body"
        );

        // PatternArgInput — with body
        let args = vec![
            PatternArg {
                name: "color".to_string(),
                value: CapturedValue::Ident("cyan".to_string()),
            },
            PatternArg {
                name: "children".to_string(),
                value: CapturedValue::Block(vec![]),
            },
        ];
        let pat_input_a = PatternArgInput::new(&form_a, &args);
        let pat_input_b = PatternArgInput::new(&form_b, &args);
        let pat_a = score_form_match(&form_a, &pat_input_a);
        let pat_b = score_form_match(&form_b, &pat_input_b);
        assert!(
            pat_b.unwrap() > pat_a.unwrap(),
            "PatternArgInput: B should win with body"
        );
    }

    #[test]
    fn test_9_2_all_adapters_same_winner_no_body() {
        let form_a = form(
            vec![],
            vec![simple_param("color", "color", CaptureType::Ident)],
            None,
        );
        let form_b = form(
            vec![],
            vec![simple_param("color", "color", CaptureType::Ident)],
            Some("$children*"),
        );

        // CaptureMapInput — no body
        let mut captures = HashMap::new();
        captures.insert(
            "color".to_string(),
            CapturedValue::Ident("cyan".to_string()),
        );
        let cap_input = CaptureMapInput {
            captures: &captures,
        };
        let cap_a = score_form_match(&form_a, &cap_input).unwrap();
        let cap_b = score_form_match(&form_b, &cap_input).unwrap();
        assert!(cap_a >= cap_b, "CaptureMapInput: A should win without body");

        // PatternArgInput — no body
        let args = vec![PatternArg {
            name: "color".to_string(),
            value: CapturedValue::Ident("cyan".to_string()),
        }];
        let pat_input_a = PatternArgInput::new(&form_a, &args);
        let pat_input_b = PatternArgInput::new(&form_b, &args);
        let pat_a = score_form_match(&form_a, &pat_input_a).unwrap();
        let pat_b = score_form_match(&form_b, &pat_input_b).unwrap();
        assert!(pat_a >= pat_b, "PatternArgInput: A should win without body");
    }

    // =========================================================================
    // FEAT-087: explain_form_match faithfulness + breakdown
    // =========================================================================

    /// The delegation invariant: the fast path total MUST equal the explainer's
    /// total for every form/input, so the introspection can never lie about a
    /// real dispatch outcome.
    #[test]
    fn explain_total_equals_score_for_all_branches() {
        let lit = |s: &str| FormInlineElement::Literal(s.to_string());
        let cap = |n: &str, ct: CaptureType| FormInlineElement::Capture(capture(n, ct), None);
        let forms = vec![
            // literal + required capture
            form(
                vec![lit("fetch"), cap("src", CaptureType::Expr)],
                vec![],
                None,
            ),
            // required param + optional inline
            form(
                vec![FormInlineElement::Capture(
                    FormCapture {
                        var_name: "opt".into(),
                        capture_type: CaptureType::Ident,
                        modifier: CaptureModifier::Optional,
                        alias_capture: None,
                    },
                    None,
                )],
                vec![simple_param("color", "c", CaptureType::Color)],
                Some("$children*"),
            ),
            // body-taking form
            form(vec![cap("name", CaptureType::Ident)], vec![], Some("$body")),
        ];
        let inputs = vec![
            MockInput::new(),
            MockInput::new()
                .with("src", MockValue::Expr)
                .with("name", MockValue::Ident),
            MockInput::new().with("c", MockValue::Ident).with_body(),
            MockInput::new().with("name", MockValue::Number).with_body(),
        ];
        for (fi, f) in forms.iter().enumerate() {
            for (ii, inp) in inputs.iter().enumerate() {
                let fast = score_form_match(f, inp);
                let ex = explain_form_match(f, inp);
                assert_eq!(
                    fast, ex.total,
                    "delegation invariant broken at form {fi} input {ii}: fast={fast:?} explain={:?} terms={:?}",
                    ex.total, ex.terms
                );
            }
        }
    }

    /// A matched literal records +3 "matched"; a present required capture +2.
    #[test]
    fn explain_records_literal_and_capture_terms() {
        let f = form(
            vec![
                FormInlineElement::Literal("fetch".to_string()),
                FormInlineElement::Capture(capture("src", CaptureType::Expr), None),
            ],
            vec![],
            None,
        );
        let input = MockInput::new().with("src", MockValue::Expr);
        let ex = explain_form_match(&f, &input);
        assert_eq!(ex.total, Some(5));
        let lit = ex
            .terms
            .iter()
            .find(|t| t.element_label.contains("fetch"))
            .unwrap();
        assert_eq!((lit.points, lit.verdict.as_str()), (3, "matched"));
        let src = ex
            .terms
            .iter()
            .find(|t| t.element_label.contains("$src"))
            .unwrap();
        assert_eq!((src.points, src.verdict.as_str()), (2, "matched"));
    }

    /// A missing required capture hard-rejects: total None, last term "missing".
    #[test]
    fn explain_missing_required_rejects() {
        let f = form(
            vec![FormInlineElement::Capture(
                capture("src", CaptureType::Expr),
                None,
            )],
            vec![],
            None,
        );
        let ex = explain_form_match(&f, &MockInput::new());
        assert_eq!(ex.total, None);
        assert_eq!(ex.terms.last().unwrap().verdict, "missing");
    }
}
