//! Syntax Registry - Prefix-Indexed Form Storage
//!
//! The SyntaxRegistry stores %form patterns indexed by their prefix character.
//! This enables O(1) lookup to find candidate forms when parsing a statement.
//!
//! # Example
//!
//! ```ignore
//! // Forms are indexed by their directive_name prefix:
//! // "@data ..." → '@' → [data_form, computed_form, ...]
//! // "$name:ident ..." → '$' → [local_state_form]
//! // "&name:ident ..." → '&' → [element_ref_form]
//! ```

use std::collections::HashMap;

use crate::parser::meta_ast::{FormClause, MacroDefAst};

/// A registered form with its associated macro definition.
///
/// This combines the %form pattern with the full macro definition
/// needed for resolution (via %binds).
#[derive(Debug, Clone)]
pub struct RegisteredForm {
    /// The macro name (e.g., "data-fetch", "local-state")
    pub macro_name: String,
    /// The %form pattern for matching
    pub form: FormClause,
    /// Full macro definition for %binds resolution
    pub macro_def: MacroDefAst,
    /// Specificity score for disambiguation (higher = more specific)
    pub specificity: u32,
}

impl RegisteredForm {
    /// Create a new registered form from a macro definition.
    pub fn new(macro_def: &MacroDefAst) -> Option<Self> {
        let form = macro_def.form.as_ref()?;
        let mut specificity = calculate_specificity(form);
        // A migration's %rewrite rule pattern must outrank the retired
        // macro's (typically broader, all-optional-params) catch-all form —
        // the shim keys the auto-rewrite off `matched_macro`, so the rule
        // def has to be the def parse selects. The boost is tagged-only:
        // live macros are untouched, and among retired defs the relative
        // order still comes from the patterns themselves.
        if macro_def
            .retired
            .as_ref()
            .is_some_and(|tag| tag.rule.is_some())
        {
            specificity += RETIRED_RULE_BOOST;
        }

        Some(Self {
            macro_name: macro_def.name.clone(),
            form: form.clone(),
            macro_def: macro_def.clone(),
            specificity,
        })
    }
}

/// Specificity boost for a migration's %rewrite rule defs, so they always
/// rank above the embedded retired macro's catch-all form (which can carry
/// many optional params). Well beyond any plausible param count.
const RETIRED_RULE_BOOST: u32 = 1000;

/// Calculate specificity of a form pattern.
///
/// More specific patterns should match first:
/// - More inline elements = more specific
/// - More params = more specific
/// - More literal elements = more specific (vs captures)
fn calculate_specificity(form: &FormClause) -> u32 {
    use crate::parser::meta_ast::FormInlineElement;

    let mut score = 0u32;

    // Base score for having any pattern
    score += 10;

    // Score for inline elements
    for element in &form.inline_elements {
        match element {
            FormInlineElement::Literal(_) => score += 5, // Literals are very specific
            FormInlineElement::Capture(_, _) => score += 2, // Captures are less specific
            FormInlineElement::Comparison { .. } => score += 4, // Comparisons are fairly specific
            FormInlineElement::KeywordBlock { body_params, .. } => {
                score += 5 + body_params.len() as u32; // Keyword blocks with captures
            }
            FormInlineElement::PseudoSelector { body_params, .. } => {
                score += 4 + body_params.len() as u32; // Pseudo-selectors with captures
            }
            FormInlineElement::Group { elements, .. } => {
                score += 4 + elements.len() as u32; // A grouped clause (e.g. an optional timeout)
            }
            FormInlineElement::PseudoClass { body_params, .. } => {
                score += 4 + body_params.len() as u32; // Pseudo-classes with captures
            }
        }
    }

    // Score for params
    for param in &form.params {
        score += 3; // Each named param adds specificity
        score += param.elements.len() as u32; // More complex params add more
    }

    // Score for body capture
    if let Some(body) = &form.body_capture {
        score += 2;
        // A body that names a SPECIFIC, constrained capture type is more specific
        // than one naming a PERMISSIVE collection (`properties`, `component_body`,
        // `param_list`) that matches almost any body — including the empty one.
        // This lets two same-surface macros that differ ONLY by body shape be
        // disambiguated by the grammar alone (PLAN-038 W0: `@type Name { fields }`
        // vs `@type Name { A(T) | B }`), with NO Rust dispatch branch: the
        // constrained-body form is tried first and a non-matching body falls
        // through to the permissive form.
        if body_capture_is_constrained(body) {
            score += 1;
        }
    }

    score
}

/// True when a `%form` body capture spec names a SPECIFIC capture type rather than
/// a permissive collection. Permissive bodies (`properties`, `component_body`,
/// `param_list`, `match_arm`) match a near-empty body and so should rank LOWER
/// than a constrained sibling (e.g. `variant_list`, which requires ≥1 `|`).
/// Conservative: only the empty/absent case and the known-permissive set return
/// false; anything else is treated as constrained (+1).
fn body_capture_is_constrained(body_capture: &str) -> bool {
    // Extract the capture TYPE name(s): the token(s) after a `:` in the spec.
    // e.g. `{ $fields:properties }` → "properties"; `{ $variants:variant_list }`
    // → "variant_list". A body with NO typed capture (literal-only) is not
    // constrained for this purpose.
    const PERMISSIVE: &[&str] = &[
        "properties",
        "fields",
        "component_body",
        "param_list",
        "params",
        "keyframes",
        // `mutation_actions` accepts ARBITRARY statements (including arm-shaped
        // lines, as plain text) — it is the permissive sibling of `on_arm+`
        // (PLAN-124 W3.3), which requires the strict `pat => --form;` shape.
        // The arms form must be tried first; non-arm bodies fall through.
        "mutation_actions",
    ];
    let mut saw_typed = false;
    for seg in body_capture.split(':').skip(1) {
        // The type name is the leading identifier run of this segment.
        let ty: String = seg
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if ty.is_empty() {
            continue;
        }
        saw_typed = true;
        if PERMISSIVE.contains(&ty.as_str()) {
            return false;
        }
    }
    saw_typed
}

/// Extract the prefix character from a macro's %form pattern.
///
/// The prefix is the first character of the directive_name in the %form.
/// For "@data", the prefix is '@'.
/// For "$name:ident ...", the prefix is '$'.
pub fn extract_prefix(macro_def: &MacroDefAst) -> Option<char> {
    let form = macro_def.form.as_ref()?;

    // The directive_name contains the prefix
    // e.g., "@data" → '@', "$" → '$', "&" → '&'
    form.directive_name.chars().next()
}

/// Syntax registry that stores %form patterns indexed by prefix.
///
/// Enables O(1) lookup to find candidate forms when parsing.
#[derive(Debug, Clone, Default)]
pub struct SyntaxRegistry {
    /// Maps prefix char → list of forms (sorted by specificity, descending)
    prefixes: HashMap<char, Vec<RegisteredForm>>,

    /// All registered forms (for iteration)
    all_forms: Vec<RegisteredForm>,

    /// Registered `%capture_type` definitions keyed by name (PLAN-023 W2). These are
    /// compiled into custom extractors at match time so stdlib-defined grammar productions
    /// (e.g. param_list, properties) actually drive matching.
    capture_types: HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst>,

    /// The `%capture` productions named by `%scalar_type` rows — the set of
    /// grammars whose captured value is its SOURCE TEXT rather than a record of
    /// the grammar's internal parts (FEAT-168 / PLAN-122).
    ///
    /// This rides the same bootstrap path as `capture_types` because it answers
    /// a question about the SAME objects, and the extractor that needs it runs
    /// while the meta registry is still being built — so it cannot reach for
    /// `cached_stdlib_registry()` without re-entering that lock.
    scalar_captures: std::collections::HashSet<String>,
}

impl SyntaxRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            prefixes: HashMap::new(),
            all_forms: Vec::new(),
            capture_types: HashMap::new(),
            scalar_captures: std::collections::HashSet::new(),
        }
    }

    /// Register a `%capture_type` definition (PLAN-023 W2). Later definitions override
    /// earlier ones by name (allows stdlib to be overridden).
    /// Record that `capture` is the production of a `%scalar_type` row.
    pub fn register_scalar_capture(&mut self, capture: String) {
        if !capture.is_empty() {
            self.scalar_captures.insert(capture);
        }
    }

    /// The productions that flatten to their source text. See the field.
    pub fn scalar_captures(&self) -> &std::collections::HashSet<String> {
        &self.scalar_captures
    }

    pub fn register_capture_type(&mut self, def: crate::parser::meta_ast::CaptureTypeDefAst) {
        self.capture_types.insert(def.name.clone(), def);
    }

    /// All registered `%capture_type` definitions.
    pub fn capture_types(
        &self,
    ) -> impl Iterator<Item = &crate::parser::meta_ast::CaptureTypeDefAst> + '_ {
        self.capture_types.values()
    }

    /// Register a macro's %form pattern.
    ///
    /// The form is indexed by its prefix character for O(1) lookup.
    pub fn register(&mut self, macro_def: &MacroDefAst) {
        let Some(mut registered) = RegisteredForm::new(macro_def) else {
            return; // No %form clause
        };

        // BUG-367: push an enclosing clause's optionality DOWN onto the captures
        // it contains, so the registry can answer "may this capture be absent?"
        //
        // `( :entering { $_enterAnim:keyframes } )?` means the capture may be
        // absent, but the `?` sits on the clause while the capture kept
        // `Required`. Every consumer that asks the registry — completion,
        // validation, the EDN printer, an agent reasoning about a form's shape —
        // inherited that wrong answer. The `%form` IS the language's
        // self-description, so a disagreement with the parser propagates
        // everywhere.
        //
        // This belongs HERE and not in the parser: `%match` patterns (migration
        // rules) are `FormClause`s built by the same code, but their optionality
        // means something different — what to MATCH, not what a form permits.
        // Propagating there marked `:entering { $enterAnim:keyframes }` optional
        // and tripped the migration validator's "optional hole with no default"
        // check on 139 tests. Only forms that reach the REGISTRY are language
        // surface.
        propagate_form_optionality(&mut registered.form);

        let Some(prefix) = extract_prefix(macro_def) else {
            return; // Can't determine prefix
        };

        // Add to prefix index
        let forms = self.prefixes.entry(prefix).or_default();
        forms.push(registered.clone());

        // Sort by specificity (descending) so more specific forms match first
        forms.sort_by(|a, b| b.specificity.cmp(&a.specificity));

        // Add to all_forms
        self.all_forms.push(registered);
    }

    /// Get all forms for a given prefix character.
    ///
    /// Returns forms sorted by specificity (most specific first).
    pub fn forms_for_prefix(&self, prefix: char) -> &[RegisteredForm] {
        self.prefixes
            .get(&prefix)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all registered prefixes.
    pub fn prefixes(&self) -> impl Iterator<Item = char> + '_ {
        self.prefixes.keys().copied()
    }

    /// Get total number of registered forms.
    pub fn form_count(&self) -> usize {
        self.all_forms.len()
    }

    /// Iterate over all registered forms.
    pub fn all_forms(&self) -> impl Iterator<Item = &RegisteredForm> {
        self.all_forms.iter()
    }

    /// Check if a prefix has any registered forms.
    pub fn has_prefix(&self, prefix: char) -> bool {
        self.prefixes.contains_key(&prefix)
    }

    /// Get a specific form by macro name.
    pub fn get_by_macro_name(&self, name: &str) -> Option<&RegisteredForm> {
        self.all_forms.iter().find(|f| f.macro_name == name)
    }

    /// Whether a macro is *body-bearing*: it carries a `$body:component_body`
    /// capture, so its body is lowered to a World-A `Construct` SCOPE (states,
    /// exports, refs, directives all sourced from the scope) and mounts
    /// per-instance. This is the registry-derived form of the FEAT-116 predicate
    /// — the closed set `{template, editable-mark, editable-block}` is no longer
    /// hardcoded in Rust; ANY stdlib macro whose `%form` declares a
    /// `component_body` body capture is body-bearing automatically (the
    /// self-describing-metasystem endgame). Constructs with a *different* body
    /// capture (`@view`/`@match` → `match_arm`, `@each` → `template_invocation`)
    /// are correctly excluded — they have arm/invocation bodies, not component
    /// bodies, and lower via their own flat-macro paths.
    pub fn is_body_bearing(&self, name: &str) -> bool {
        self.all_forms.iter().any(|f| {
            f.macro_name == name
                && f.form
                    .body_capture
                    .as_deref()
                    .is_some_and(|b| b.contains(":component_body"))
        })
    }

    /// Whether a directive captures its body as an OPAQUE `:block` (or
    /// `:skip_block`) — the macro re-compiles that body as a SEPARATE sub-program
    /// via a `%$content.js` / `%$body.js` interpolation (the `@test`/`@mount`/
    /// `@fixture`/`@given`/`@try` testing surface). Such a body's interior
    /// directives (`@on`, `local-state`, nested `.sel{}` blocks) belong to that
    /// sub-program ONLY; they must NOT also be lowered as page-level FormMatches by
    /// the outer compile, or every interior `@on` binds twice — once correctly via
    /// the macro's recompiled `%$content.js` (scoped to the mounted node) and once
    /// wrongly at file scope (bound to `document.body`, an ancestor → the user
    /// event fires both, doubling the mutation; FUP-069). Registry-derived, not a
    /// hardcoded macro-name set: ANY macro whose `%form` declares a `:block` body
    /// capture is opaque-bodied automatically. (`:component_body` is NOT opaque —
    /// it lowers to a World-A Construct scope; see [`is_body_bearing`].)
    pub fn has_opaque_block_body(&self, name: &str) -> bool {
        use crate::parser::meta_ast::{EmitLang, MacroBodyItem};
        self.all_forms.iter().any(|f| {
            if f.macro_name != name {
                return false;
            }
            // The body must be captured as `:block` / `:skip_block` AND the macro
            // must RECOMPILE it as a sub-program — i.e. a `%emit js` block
            // interpolates the captured body's `.js` accessor (`%$content.js` /
            // `%$body.js`). That `.js` re-parse+compile is exactly what makes the
            // interior a separate program whose matches must NOT also lower at page
            // scope (FUP-069). A `:block` macro that instead inlines its body via
            // `.html`/`.source` (e.g. docs `@example`) renders the block as LIVE
            // page DOM — its interior directives SHOULD reach page scope, so it is
            // correctly NOT opaque here.
            let Some(bc) = f.form.body_capture.as_deref() else {
                return false;
            };
            let is_block = bc.contains(":block") || bc.contains(":skip_block");
            if !is_block {
                return false;
            }
            // Capture name from `{ $<name>:block }` (default to common names).
            let cap_name = bc
                .split('$')
                .nth(1)
                .and_then(|s| s.split(|c: char| c == ':' || c.is_whitespace()).next())
                .unwrap_or("");
            let js_accessor = format!("%${}.js", cap_name);
            f.macro_def.body.iter().any(|item| {
                matches!(item, MacroBodyItem::Emit(e)
                    if e.lang == EmitLang::Js && e.content.contains(&js_accessor))
            })
        })
    }

    /// Whether a directive is a GLOBAL import — its macro carries a
    /// `%imports { ..., global: true }` declared-effect clause (FEAT-118 /
    /// `docs/specs/declared-effects.md`). This is the registry-derived form of
    /// the old hardcoded `name == "import"` special-case in the parser: ANY
    /// macro declaring a global `%imports` clause is an import directive, so
    /// `@import` is no longer privileged in Rust — it is the `%macro import`
    /// (stdlib/macros/presets.st) that declares the effect. Mirrors
    /// [`is_body_bearing`]: structure-as-data, not a Rust string match.
    pub fn is_global_import(&self, name: &str) -> bool {
        self.all_forms
            .iter()
            .any(|f| f.macro_name == name && f.macro_def.imports.as_ref().is_some_and(|i| i.global))
    }

    /// The capture NAME holding the module path for a global-import directive
    /// (the `module:` field of its `%imports` clause), if any. Lets the parser
    /// extract the import path generically instead of assuming the first inline
    /// arg of a hardcoded `@import`.
    pub fn global_import_module_capture(&self, name: &str) -> Option<&str> {
        self.all_forms.iter().find_map(|f| {
            if f.macro_name == name {
                f.macro_def
                    .imports
                    .as_ref()
                    .filter(|i| i.global)
                    .map(|i| i.module.as_str())
            } else {
                None
            }
        })
    }

    /// Whether a directive is a NAMESPACED import — its macro carries a
    /// `%imports { ... }` clause with `global: false` (e.g. `@use`, FEAT-118).
    /// Such an import registers the loaded module's defs under a namespace
    /// rather than flat-merging them globally.
    pub fn is_namespaced_import(&self, name: &str) -> bool {
        self.all_forms.iter().any(|f| {
            f.macro_name == name && f.macro_def.imports.as_ref().is_some_and(|i| !i.global)
        })
    }

    /// Get all forms that match a directive name (for @directive syntax).
    ///
    /// For example, get_forms_for_directive("data") returns all forms
    /// where directive_name is "@data" (or starts with "@data").
    pub fn get_forms_for_directive(&self, directive: &str) -> Vec<&RegisteredForm> {
        let at_directive = format!("@{}", directive);
        self.prefixes
            .get(&'@')
            .map(|forms| {
                forms
                    .iter()
                    .filter(|f| {
                        f.form.directive_name == at_directive
                            || f.form
                                .directive_name
                                .starts_with(&format!("{} ", at_directive))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Merge another registry into this one.
    pub fn merge(&mut self, other: SyntaxRegistry) {
        for form in other.all_forms {
            let prefix = form.form.directive_name.chars().next();
            if let Some(prefix) = prefix {
                let forms = self.prefixes.entry(prefix).or_default();
                forms.push(form.clone());
                forms.sort_by(|a, b| b.specificity.cmp(&a.specificity));
            }
            self.all_forms.push(form);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;
    use crate::parser::meta_ast::{CaptureModifier, CaptureType, FormCapture, FormInlineElement};

    fn make_form_clause(directive_name: &str) -> FormClause {
        FormClause {
            directive_name: directive_name.to_string(),
            inline_elements: vec![],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: Vec::new(),
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        }
    }

    fn make_form_clause_with_elements(
        directive_name: &str,
        elements: Vec<FormInlineElement>,
    ) -> FormClause {
        FormClause {
            directive_name: directive_name.to_string(),
            inline_elements: elements,
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: Vec::new(),
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        }
    }

    #[test]
    fn test_is_body_bearing_registry_derived() {
        // Q3: body-bearing detection is registry-DERIVED — a macro is body-bearing
        // iff some `%form` carries a `$body:component_body` capture. Build a tiny
        // registry with a component-body form and a match-arm form; only the former
        // is body-bearing. No hardcoded name set.
        let mut reg = SyntaxRegistry::new();
        let mut tmpl = make_form_clause("@template");
        tmpl.body_capture = Some("{ $body:component_body }".to_string());
        let mut viewform = make_form_clause("@view");
        viewform.body_capture = Some("{ $arms:match_arm+ }".to_string());
        let mut plain = make_form_clause("@plain");
        plain.body_capture = None;
        reg.all_forms.push(RegisteredForm {
            macro_name: "template".to_string(),
            form: tmpl,
            macro_def: Default::default(),
            specificity: 0,
        });
        reg.all_forms.push(RegisteredForm {
            macro_name: "view".to_string(),
            form: viewform,
            macro_def: Default::default(),
            specificity: 0,
        });
        reg.all_forms.push(RegisteredForm {
            macro_name: "plain".to_string(),
            form: plain,
            macro_def: Default::default(),
            specificity: 0,
        });
        assert!(
            reg.is_body_bearing("template"),
            "component_body capture ⇒ body-bearing"
        );
        assert!(
            !reg.is_body_bearing("view"),
            "match_arm body is NOT a component body"
        );
        assert!(
            !reg.is_body_bearing("plain"),
            "no body capture ⇒ not body-bearing"
        );
        assert!(
            !reg.is_body_bearing("absent"),
            "unknown name ⇒ not body-bearing"
        );
    }

    #[test]
    fn test_novel_construct_is_body_bearing_with_zero_rust() {
        // THE Q3/FEAT-116 PAYOFF, proven empirically: a construct name Rust has
        // NEVER heard of becomes body-bearing the instant its `%form` declares a
        // `$body:component_body` capture — no match-arm, no Rust edit. This is what
        // “adding a body-bearing construct is a pure-stdlib act” MEANS.
        use crate::parser::meta_ast::MetaDef;
        let novel = r#"
%macro panel {
  %scope file
  %form {
    @panel &$name:ident($params:param_list) {
      $body:component_body
    }
  }
  %registers template($name) {
    params: $params
    body: $body
  }
  %binds {
    register-template(name: $name, body: $body, params: $params) -> { $factory }
  }
}
"#;
        let parsed =
            crate::parser::parse_for_bootstrap(novel).expect("novel construct source parses");
        let mut reg = SyntaxRegistry::new();
        for def in parsed.meta_defs {
            if let MetaDef::Macro(m) = def {
                reg.register(&m);
            }
        }
        // Rust never mentions “panel” anywhere; the registry knows it is body-bearing
        // purely from the `$body:component_body` in its `%form`.
        assert!(
            reg.is_body_bearing("panel"),
            "a novel `%macro` whose %form carries `$body:component_body` is body-bearing \
             with ZERO Rust changes"
        );
        // A sibling macro with a DIFFERENT body capture is NOT swept in.
        let arms = r#"
%macro switch {
  %scope file
  %form {
    @switch &$name:ident($params:param_list) {
      $arms:match_arm+
    }
  }
}
"#;
        let parsed2 = crate::parser::parse_for_bootstrap(arms).expect("parses");
        for def in parsed2.meta_defs {
            if let MetaDef::Macro(m) = def {
                reg.register(&m);
            }
        }
        assert!(
            !reg.is_body_bearing("switch"),
            "a match-arm body is NOT a component body — the predicate discriminates by \
             capture type, not by having SOME body"
        );
    }

    #[test]
    fn test_is_global_import_registry_derived() {
        // FEAT-118: import recognition is registry-DERIVED, not a hardcoded
        // `name == "import"`. A macro named anything, declaring %imports with
        // global:true, IS a global import. Proves @import is no longer privileged.
        let src = r#"
%macro pull {
  %form {
    @pull $path:string
  }
  %imports {
    module: $path
    global: true
  }
}
"#;
        let parsed = crate::parser::parse_for_bootstrap(src).expect("parses");
        let mut reg = SyntaxRegistry::new();
        for def in parsed.meta_defs {
            if let crate::parser::meta_ast::MetaDef::Macro(m) = def {
                reg.register(&m);
            }
        }
        assert!(
            reg.is_global_import("pull"),
            "a macro declaring %imports{{global:true}} is a global import regardless of name"
        );
        assert_eq!(
            reg.global_import_module_capture("pull"),
            Some("path"),
            "the module-path capture name is read from the %imports clause"
        );

        // A namespaced (@use-style, global:false) %imports is NOT a global import.
        let scoped = r#"
%macro use {
  %form {
    @use $path:string
  }
  %imports {
    module: $path
  }
}
"#;
        let parsed2 = crate::parser::parse_for_bootstrap(scoped).expect("parses");
        for def in parsed2.meta_defs {
            if let crate::parser::meta_ast::MetaDef::Macro(m) = def {
                reg.register(&m);
            }
        }
        assert!(
            !reg.is_global_import("use"),
            "@use is namespaced (global:false), not a flat-global import"
        );

        // A macro with no %imports clause is not an import at all.
        assert!(!reg.is_global_import("pull-nonexistent"));
    }

    #[test]
    fn test_stdlib_body_bearing_set() {
        // The live stdlib registry: the body-bearing set is exactly the three
        // component-body constructs; @view/@match/@each (different body captures)
        // and leaf macros are excluded. Pins the FEAT-116 “complete current
        // population” as an OBSERVED registry fact, not a hardcoded list.
        let reg = &*crate::syntax::STDLIB_REGISTRY;
        for n in ["template", "editable-mark", "editable-block"] {
            assert!(reg.is_body_bearing(n), "{n} must be body-bearing");
        }
        for n in ["view", "match", "each", "local-state"] {
            assert!(!reg.is_body_bearing(n), "{n} must NOT be body-bearing");
        }
    }

    #[test]
    fn test_specificity_calculation() {
        // Simple form
        let simple = make_form_clause("@data");
        let simple_score = calculate_specificity(&simple);

        // Form with literal
        let with_literal = make_form_clause_with_elements(
            "@data",
            vec![FormInlineElement::Literal(":".to_string())],
        );
        let literal_score = calculate_specificity(&with_literal);

        // Form with capture
        let with_capture = make_form_clause_with_elements(
            "@data",
            vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "name".to_string(),
                    capture_type: CaptureType::Ident,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
        );
        let capture_score = calculate_specificity(&with_capture);

        // Literal should be more specific than capture
        assert!(literal_score > capture_score);

        // Both should be more specific than simple
        assert!(literal_score > simple_score);
        assert!(capture_score > simple_score);
    }

    #[test]
    fn test_forms_sorted_by_specificity() {
        let mut registry = SyntaxRegistry::new();

        // Add less specific form first
        let simple_macro = MacroDefAst {
            retired: None,
            name: "data-simple".to_string(),
            form: Some(make_form_clause("@data")),
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            span: SourceSpan::default(),
            source_file: None,
            requires: vec![],
            module: None,
            doc: None,
            ..Default::default()
        };

        // Add more specific form second
        let complex_macro = MacroDefAst {
            retired: None,
            name: "data-complex".to_string(),
            form: Some(make_form_clause_with_elements(
                "@data",
                vec![
                    FormInlineElement::Capture(
                        FormCapture {
                            var_name: "name".to_string(),
                            capture_type: CaptureType::Ident,
                            modifier: CaptureModifier::Required,
                            alias_capture: None,
                        },
                        None,
                    ),
                    FormInlineElement::Literal(":".to_string()),
                ],
            )),
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            span: SourceSpan::default(),
            source_file: None,
            requires: vec![],
            module: None,
            doc: None,
            ..Default::default()
        };

        registry.register(&simple_macro);
        registry.register(&complex_macro);

        let forms = registry.forms_for_prefix('@');
        assert_eq!(forms.len(), 2);

        // More specific (complex) should come first
        assert_eq!(forms[0].macro_name, "data-complex");
        assert_eq!(forms[1].macro_name, "data-simple");
    }
}

/// Push an enclosing clause's optionality DOWN onto the captures it contains
/// (BUG-367).
///
/// `( :entering { $_enterAnim:keyframes } )?` means the capture may be absent,
/// but the `?` sits on the GROUP while `$_enterAnim` kept
/// `CaptureModifier::Required`. Anything asking the registry "may this capture
/// be absent?" — completion, validation, the EDN printer, an agent reasoning
/// about a form's shape — then got the wrong answer for a clause the language
/// treats as optional.
///
/// The `%form` IS the language's self-description, so a disagreement between it
/// and the parser is inherited by every consumer. Fixing it once, here, is why
/// no consumer needs its own workaround.
///
/// Only genuinely-optional wrappers propagate: `Required` and `OneOrMore` mean
/// the clause must appear, so their contents stay as declared.
fn propagate_optionality(elements: &mut [crate::parser::meta_ast::FormInlineElement], enclosing_optional: bool) {
    for el in elements.iter_mut() {
        match el {
            crate::parser::meta_ast::FormInlineElement::Capture(cap, _) if enclosing_optional => {
                if matches!(cap.modifier, crate::parser::meta_ast::CaptureModifier::Required) {
                    cap.modifier = crate::parser::meta_ast::CaptureModifier::Optional;
                }
            }
            crate::parser::meta_ast::FormInlineElement::Group {
                elements: inner,
                modifier,
            } => {
                let optional = enclosing_optional
                    || !matches!(
                        modifier,
                        crate::parser::meta_ast::CaptureModifier::Required | crate::parser::meta_ast::CaptureModifier::OneOrMore
                    );
                propagate_optionality(inner, optional);
            }
            // A pseudo-selector clause carries its own modifier and is the shape
            // `( :entering { … } )?` actually produces — not a `Group`.
            crate::parser::meta_ast::FormInlineElement::PseudoSelector {
                body_params,
                modifier,
                ..
            } => {
                let optional = enclosing_optional
                    || !matches!(
                        modifier,
                        crate::parser::meta_ast::CaptureModifier::Required | crate::parser::meta_ast::CaptureModifier::OneOrMore
                    );
                for p in body_params.iter_mut() {
                    propagate_optionality(&mut p.elements, optional);
                }
            }
            crate::parser::meta_ast::FormInlineElement::KeywordBlock {
                body_params,
                modifier,
                ..
            } => {
                let optional = enclosing_optional
                    || !matches!(
                        modifier,
                        crate::parser::meta_ast::CaptureModifier::Required | crate::parser::meta_ast::CaptureModifier::OneOrMore
                    );
                for p in body_params.iter_mut() {
                    propagate_optionality(&mut p.elements, optional);
                }
            }
            crate::parser::meta_ast::FormInlineElement::PseudoClass { body_params, .. } => {
                for p in body_params.iter_mut() {
                    propagate_optionality(&mut p.elements, enclosing_optional);
                }
            }
            _ => {}
        }
    }
}


/// Apply [`propagate_optionality`] across every element list of a form clause.
fn propagate_form_optionality(form: &mut FormClause) {
    propagate_optionality(&mut form.inline_elements, false);
    propagate_optionality(&mut form.post_arg_inline, false);
    for p in form.params.iter_mut() {
        propagate_optionality(&mut p.elements, p.default.is_some());
    }
    for p in form.body_params.iter_mut() {
        propagate_optionality(&mut p.elements, false);
    }
}
