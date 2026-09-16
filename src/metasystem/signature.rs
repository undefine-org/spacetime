//! Human-scannable rendering of a macro's dispatch signature (FEAT-089).
//!
//! A `%form`'s `FormClause` is exactly what the predicate-dispatch scorer reads
//! (directive · literals · typed captures · params). This module renders it two
//! ways from the SAME walk so they cannot drift:
//!   - [`form_signature_tokens`] — a `Vec<SigToken>` (text + semantic role) for
//!     UIs that chip-style each token (the macro reference's coloured signatures).
//!   - [`format_form_signature`] — the flat string, defined AS the tokens joined,
//!     so `format_form_signature(f) == join(tokens(f))`.

use crate::parser::meta_ast::{
    CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement, FormParam,
};

/// Semantic role of one signature token, for styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SigRole {
    /// The `@directive` head.
    Directive,
    /// A `$name` capture variable.
    Binding,
    /// A `:type` annotation (with any `?`/`*`/`+` modifier).
    Type,
    /// A discriminating keyword literal (e.g. `fetch`, `from`).
    Literal,
    /// Structural punctuation literal (`:`, `;`, `<-`).
    Punct,
    /// A `name:` parameter label inside parens.
    Param,
    /// Brackets / parens / `{ … }` body marker.
    Delim,
}

/// One rendered piece of a signature.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SigToken {
    pub text: String,
    pub role: SigRole,
}

impl SigToken {
    fn new(text: impl Into<String>, role: SigRole) -> Self {
        SigToken {
            text: text.into(),
            role,
        }
    }
}

/// Short, source-faithful name for a `CaptureType` (e.g. `binding`, `expr`,
/// `typeref`, `("a"|"b")`, `balanced(';')`, a custom type's own name).
pub fn capture_type_name(ct: &CaptureType) -> String {
    match ct {
        CaptureType::Union(opts) => {
            let inner = opts
                .iter()
                .map(|o| format!("\"{o}\""))
                .collect::<Vec<_>>()
                .join("|");
            format!("({inner})")
        }
        CaptureType::Balanced(c) => format!("balanced('{c}')"),
        CaptureType::PatternMatch { variant, .. } => format!("is {variant}"),
        CaptureType::Custom(s) => s.clone(),
        other => {
            // Lowercase the Debug head: `Ident` -> "ident", `Typeref` -> "typeref".
            let dbg = format!("{other:?}");
            dbg.split(['(', ' ', '{'])
                .next()
                .unwrap_or(&dbg)
                .to_lowercase()
        }
    }
}

fn modifier_suffix(m: &CaptureModifier) -> &'static str {
    match m {
        CaptureModifier::Required => "",
        CaptureModifier::Optional => "?",
        CaptureModifier::ZeroOrMore => "*",
        CaptureModifier::OneOrMore => "+",
        // A counted class contributes no suffix to a form SIGNATURE: it belongs to
        // a char-class terminal inside a capture type, not to a form parameter, so
        // it never varies the shape two forms are compared on.
        CaptureModifier::Counted(_) => "",
    }
}

/// True for a literal that is a discriminating keyword (has a letter/digit),
/// false for pure punctuation (`:`, `;`, `<-`, `->`).
fn is_keyword_literal(s: &str) -> bool {
    s.chars().any(|c| c.is_alphanumeric())
}

/// Push the tokens for one capture: `$name`, then `:type±modifier`, then any
/// `as $alias:type` continuation.
fn push_capture(out: &mut Vec<SigToken>, cap: &FormCapture) {
    out.push(SigToken::new(
        format!("${}", cap.var_name),
        SigRole::Binding,
    ));
    out.push(SigToken::new(
        format!(
            ":{}{}",
            capture_type_name(&cap.capture_type),
            modifier_suffix(&cap.modifier)
        ),
        SigRole::Type,
    ));
    if let Some(alias) = &cap.alias_capture {
        out.push(SigToken::new("as", SigRole::Literal));
        push_capture(out, alias);
    }
}

fn push_inline(out: &mut Vec<SigToken>, el: &FormInlineElement) {
    match el {
        FormInlineElement::Literal(s) => {
            let role = if is_keyword_literal(s) {
                SigRole::Literal
            } else {
                SigRole::Punct
            };
            out.push(SigToken::new(s.clone(), role));
        }
        FormInlineElement::Capture(cap, _) => push_capture(out, cap),
        FormInlineElement::Comparison { operator, capture } => {
            out.push(SigToken::new(operator.clone(), SigRole::Punct));
            push_capture(out, capture);
        }
        FormInlineElement::KeywordBlock {
            keyword, modifier, ..
        } => {
            out.push(SigToken::new(keyword.clone(), SigRole::Literal));
            out.push(SigToken::new(
                format!("{{ … }}{}", modifier_suffix(modifier)),
                SigRole::Delim,
            ));
        }
        FormInlineElement::PseudoSelector { name, modifier, .. } => {
            out.push(SigToken::new(
                format!("(:{name} {{ … }}){}", modifier_suffix(modifier)),
                SigRole::Delim,
            ));
        }
        FormInlineElement::PseudoClass { name, .. } => {
            out.push(SigToken::new(format!(":{name} {{ … }}"), SigRole::Delim));
        }
        FormInlineElement::Group { elements, modifier } => {
            out.push(SigToken::new("(".to_string(), SigRole::Delim));
            for el in elements {
                push_inline(out, el);
            }
            out.push(SigToken::new(
                format!("){}", modifier_suffix(modifier)),
                SigRole::Delim,
            ));
        }
    }
}

fn push_param(out: &mut Vec<SigToken>, p: &FormParam) {
    out.push(SigToken::new(format!("{}:", p.name), SigRole::Param));
    if let Some(cap) = p.capture() {
        push_capture(out, cap);
    } else {
        // Multi-element param (e.g. `when: $sig is Variant { … }`): render its
        // elements inline.
        for el in &p.elements {
            push_inline(out, el);
        }
    }
}

/// Structured tokens for a macro form's dispatch signature.
pub fn form_signature_tokens(form: &FormClause) -> Vec<SigToken> {
    let mut out: Vec<SigToken> = Vec::new();

    let dir = if form.directive_name.starts_with('@') {
        form.directive_name.clone()
    } else {
        format!("@{}", form.directive_name)
    };
    out.push(SigToken::new(dir, SigRole::Directive));

    for el in &form.inline_elements {
        push_inline(&mut out, el);
    }

    if !form.params.is_empty() {
        out.push(SigToken::new("(", SigRole::Delim));
        for (i, p) in form.params.iter().enumerate() {
            if i > 0 {
                out.push(SigToken::new(",", SigRole::Delim));
            }
            push_param(&mut out, p);
        }
        out.push(SigToken::new(")", SigRole::Delim));
    }

    for el in &form.post_arg_inline {
        push_inline(&mut out, el);
    }

    if form.body_capture.is_some() || !form.body_params.is_empty() {
        out.push(SigToken::new("{ … }", SigRole::Delim));
    }

    out
}

/// The flat signature string, defined as the tokens joined by spaces so it can
/// never disagree with [`form_signature_tokens`].
pub fn format_form_signature(form: &FormClause) -> String {
    form_signature_tokens(form)
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;

    fn cap(name: &str, ct: CaptureType, m: CaptureModifier) -> FormInlineElement {
        FormInlineElement::Capture(
            FormCapture {
                var_name: name.into(),
                capture_type: ct,
                modifier: m,
                alias_capture: None,
            },
            None,
        )
    }

    fn form(inline: Vec<FormInlineElement>, params: Vec<FormParam>, body: bool) -> FormClause {
        FormClause {
            directive_name: "data".into(),
            inline_elements: inline,
            params,
            post_arg_inline: vec![],
            body_capture: if body { Some("$b".into()) } else { None },
            body_params: vec![],
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        }
    }

    #[test]
    fn signature_renders_fetch_form() {
        // @data fetch $name:binding $type:typeref? : $src:expr ;
        let f = form(
            vec![
                FormInlineElement::Literal("fetch".into()),
                cap("name", CaptureType::Binding, CaptureModifier::Required),
                cap("type", CaptureType::Typeref, CaptureModifier::Optional),
                FormInlineElement::Literal(":".into()),
                cap("src", CaptureType::Expr, CaptureModifier::Required),
                FormInlineElement::Literal(";".into()),
            ],
            vec![],
            false,
        );
        assert_eq!(
            format_form_signature(&f),
            "@data fetch $name :binding $type :typeref? : $src :expr ;"
        );
    }

    #[test]
    fn flat_string_equals_joined_tokens() {
        let f = form(
            vec![
                FormInlineElement::Literal("query".into()),
                cap("n", CaptureType::Binding, CaptureModifier::Required),
            ],
            vec![],
            true,
        );
        let joined = form_signature_tokens(&f)
            .iter()
            .map(|t| t.text.clone())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(joined, format_form_signature(&f));
    }

    #[test]
    fn keyword_vs_punct_roles() {
        let f = form(
            vec![
                FormInlineElement::Literal("fetch".into()),
                FormInlineElement::Literal(":".into()),
            ],
            vec![],
            false,
        );
        let toks = form_signature_tokens(&f);
        let fetch = toks.iter().find(|t| t.text == "fetch").unwrap();
        let colon = toks.iter().find(|t| t.text == ":").unwrap();
        assert_eq!(fetch.role, SigRole::Literal);
        assert_eq!(colon.role, SigRole::Punct);
    }

    #[test]
    fn directive_head_and_body_marker() {
        let f = form(vec![], vec![], true);
        let toks = form_signature_tokens(&f);
        assert_eq!(toks.first().unwrap().role, SigRole::Directive);
        assert_eq!(toks.first().unwrap().text, "@data");
        assert!(
            toks.iter()
                .any(|t| t.role == SigRole::Delim && t.text.contains('{'))
        );
    }
}
