//! Visual consistency lint system.
//!
//! Detects visual/accessibility issues in Spacetime files:
//! - W201: Low contrast text (WCAG 2.1 AA)
//! - W202: Invisible element (opacity:0 without reveal animation)
//! - W203: Untranslated content (text without data-t when @locale active)
//! - W204: Antipattern: inline event handler (use Spacetime @on bindings)
//! - W205: Unresolved {locale} template

use crate::color::{Color, contrast_ratio};
use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::parser::{ScopeBlock, SourceSpan};
use crate::syntax::FormMatch;

// ============================================================
// W201: Low Contrast Text
// ============================================================

/// Check for low contrast text based on CSS declarations in scopes.
/// Looks at color/background-color pairs and computes WCAG 2.1 contrast ratio.
fn check_low_contrast(scopes: &[ScopeBlock], diagnostics: &mut Vec<Diagnostic>) {
    for scope in scopes {
        let mut fg_color: Option<Color> = None;
        let mut bg_color: Option<Color> = None;
        let mut font_size_pt: Option<f64> = None;
        let mut is_bold = false;

        for decl in &scope.css_declarations {
            match decl.property.as_str() {
                "color" => {
                    fg_color = parse_css_color(&decl.value);
                }
                "background-color" | "background" => {
                    bg_color = parse_css_color(&decl.value);
                }
                "font-size" => {
                    font_size_pt = parse_font_size_pt(&decl.value);
                }
                "font-weight" => {
                    is_bold = decl.value.trim() == "bold"
                        || decl
                            .value
                            .trim()
                            .parse::<u32>()
                            .map(|w| w >= 700)
                            .unwrap_or(false);
                }
                _ => {}
            }
        }

        if let (Some(fg), Some(bg)) = (fg_color, bg_color) {
            let ratio = contrast_ratio(&fg, &bg);
            let is_large_text = font_size_pt.map(|pt| pt >= 18.0).unwrap_or(false)
                || (is_bold && font_size_pt.map(|pt| pt >= 14.0).unwrap_or(false));

            let threshold = if is_large_text { 3.0 } else { 4.5 };

            if ratio < threshold {
                diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W201,
                        format!(
                            "Low contrast text in '{}': ratio {:.2}:1 (minimum {:.1}:1 for {} text)",
                            scope.selector,
                            ratio,
                            threshold,
                            if is_large_text { "large" } else { "normal" }
                        ),
                    )
                    .with_span(scope.span.into())
                    .with_hint(format!(
                        "WCAG 2.1 AA requires {}:1 contrast. Current ratio is {:.2}:1",
                        threshold, ratio
                    )),
                );
            }
        }

        // Recurse into nested scopes
        check_low_contrast_nested(&scope.nested_scopes, diagnostics);
    }
}

fn check_low_contrast_nested(
    nested: &[crate::parser::NestedScope],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for ns in nested {
        let mut fg_color: Option<Color> = None;
        let mut bg_color: Option<Color> = None;
        let mut font_size_pt: Option<f64> = None;
        let mut is_bold = false;

        for decl in &ns.css_declarations {
            match decl.property.as_str() {
                "color" => {
                    fg_color = parse_css_color(&decl.value);
                }
                "background-color" | "background" => {
                    bg_color = parse_css_color(&decl.value);
                }
                "font-size" => {
                    font_size_pt = parse_font_size_pt(&decl.value);
                }
                "font-weight" => {
                    is_bold = decl.value.trim() == "bold"
                        || decl
                            .value
                            .trim()
                            .parse::<u32>()
                            .map(|w| w >= 700)
                            .unwrap_or(false);
                }
                _ => {}
            }
        }

        if let (Some(fg), Some(bg)) = (fg_color, bg_color) {
            let ratio = contrast_ratio(&fg, &bg);
            let is_large_text = font_size_pt.map(|pt| pt >= 18.0).unwrap_or(false)
                || (is_bold && font_size_pt.map(|pt| pt >= 14.0).unwrap_or(false));

            let threshold = if is_large_text { 3.0 } else { 4.5 };

            if ratio < threshold {
                diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W201,
                        format!(
                            "Low contrast text in '{}': ratio {:.2}:1 (minimum {:.1}:1 for {} text)",
                            ns.selector,
                            ratio,
                            threshold,
                            if is_large_text { "large" } else { "normal" }
                        ),
                    )
                    .with_span(ns.span.into())
                    .with_hint(format!(
                        "WCAG 2.1 AA requires {}:1 contrast. Current ratio is {:.2}:1",
                        threshold, ratio
                    )),
                );
            }
        }

        check_low_contrast_nested(&ns.nested_scopes, diagnostics);
    }
}

/// Parse a CSS color value string into a Color.
/// Supports hex (#RGB, #RRGGBB), rgb(), hsl(), and named colors.
/// Parse any CSS colour for the contrast lint (FUP-164 / PLAN-122).
///
/// This was the SIXTH shadow classifier: a private cascade of `starts_with`
/// tests over `#`/`rgb`/`hsl`/`oklch` plus EIGHT hardcoded named colours, out of
/// the 148 CSS defines. A page whose text was `rebeccapurple` on `cornsilk`
/// simply went unlinted — not passed, not failed, just invisible, because the
/// contrast check silently skips a colour it cannot read.
///
/// One parser now answers this, the same one behind the declaration validator
/// and the editor swatches. The alpha channel is preserved: `contrast_ratio`
/// wants it, and the old named-colour arm returned `transparent` as fully
/// opaque black, which is the wrong answer rather than no answer.
fn parse_css_color(value: &str) -> Option<Color> {
    // FEAT-109 Tier 0. This was twelve lines of lightningcss plumbing, identical
    // to `src/lsp/colors.rs::parse_css_color` but for building an f64 colour
    // instead of an f32 one — two copies of the same conversion, each able to
    // drift from the canonical parser and from each other.
    //
    // `parse_color_to_normalized_rgba` is that plumbing now, so a colour means
    // the same numbers to the contrast linter, the LSP swatch, and compile-time
    // colour algebra.
    let [r, g, b, a] = crate::color::parse_color_to_normalized_rgba(value).ok()?;
    Some(Color::rgba(r * 255.0, g * 255.0, b * 255.0, a))
}

#[cfg(test)]
mod colour_coverage {
    use super::parse_css_color;

    /// The 140 named colours the old cascade could not see.
    ///
    /// It knew eight. Everything else returned `None`, and the contrast lint
    /// SKIPS a colour it cannot read — so a page with unreadable text in any
    /// other named colour was never linted, and the suite was green throughout.
    /// That is the failure mode this cutover exists to remove.
    #[test]
    fn named_colours_beyond_the_hardcoded_eight() {
        for name in [
            "rebeccapurple",
            "cornsilk",
            "darkslategray",
            "lightgoldenrodyellow",
            "mediumaquamarine",
            "papayawhip",
        ] {
            assert!(
                parse_css_color(name).is_some(),
                "`{name}` is a CSS named colour and must be readable by the lint"
            );
        }
    }

    /// `transparent` used to come back as fully opaque BLACK — a wrong answer,
    /// which is worse than no answer, because the lint then computes a contrast
    /// ratio against a colour that is not on the page.
    #[test]
    fn transparent_keeps_its_alpha() {
        let c = parse_css_color("transparent").expect("transparent is a colour");
        let alpha = match c {
            crate::color::Color::RGB { a, .. }
            | crate::color::Color::HSL { a, .. }
            | crate::color::Color::OKLCH { a, .. } => a,
        };
        assert!(alpha.abs() < 0.01, "transparent must have alpha 0, got {alpha}");
    }

    #[test]
    fn modern_colour_syntax_is_readable() {
        for value in [
            "oklch(70% 0.1 200)",
            "color-mix(in oklch, #fff, #000)",
            "rgb(1 2 3 / 50%)",
            "hsl(210deg 50% 40%)",
        ] {
            assert!(
                parse_css_color(value).is_some(),
                "`{value}` must be readable by the lint"
            );
        }
    }

    #[test]
    fn a_non_colour_is_still_refused() {
        for value in ["#e8ee1", "not-a-colour", "12px", ""] {
            assert!(
                parse_css_color(value).is_none(),
                "`{value}` is not a colour"
            );
        }
    }
}

/// Parse a CSS font-size value to points.
/// Approximation: 1px ≈ 0.75pt, 1rem ≈ 16px ≈ 12pt
fn parse_font_size_pt(value: &str) -> Option<f64> {
    let v = value.trim();
    if let Some(px) = v.strip_suffix("px") {
        px.trim().parse::<f64>().ok().map(|px| px * 0.75)
    } else if let Some(pt) = v.strip_suffix("pt") {
        pt.trim().parse::<f64>().ok()
    } else if let Some(rem) = v.strip_suffix("rem") {
        rem.trim().parse::<f64>().ok().map(|rem| rem * 12.0)
    } else if let Some(em) = v.strip_suffix("em") {
        em.trim().parse::<f64>().ok().map(|em| em * 12.0)
    } else {
        None
    }
}

// ============================================================
// W202: Invisible Animated Element
// ============================================================

/// Check for elements with opacity: 0 that lack a reveal animation.
fn check_invisible_elements(scopes: &[ScopeBlock], diagnostics: &mut Vec<Diagnostic>) {
    for scope in scopes {
        let has_opacity_zero = scope
            .css_declarations
            .iter()
            .any(|d| d.property == "opacity" && d.value.trim() == "0");

        if has_opacity_zero {
            let has_reveal_animation = scope_has_opacity_animation(scope);
            if !has_reveal_animation {
                diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W202,
                        format!(
                            "Element '{}' has opacity: 0 but no reveal animation",
                            scope.selector
                        ),
                    )
                    .with_span(scope.span.into())
                    .with_hint(
                        "Add @scroll, @on &.visible, or @on load animation to reveal this element"
                            .to_string(),
                    ),
                );
            }
        }

        // Check nested scopes
        check_invisible_nested(&scope.nested_scopes, scope, diagnostics);
    }
}

fn check_invisible_nested(
    nested: &[crate::parser::NestedScope],
    _parent: &ScopeBlock,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for ns in nested {
        let has_opacity_zero = ns
            .css_declarations
            .iter()
            .any(|d| d.property == "opacity" && d.value.trim() == "0");

        if has_opacity_zero {
            let has_reveal = nested_has_opacity_animation(ns);
            if !has_reveal {
                diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W202,
                        format!(
                            "Element '{}' has opacity: 0 but no reveal animation",
                            ns.selector
                        ),
                    )
                    .with_span(ns.span.into())
                    .with_hint(
                        "Add @scroll, @on &.visible, or @on load animation to reveal this element"
                            .to_string(),
                    ),
                );
            }
        }
    }
}

/// Check if a scope has FormMatches that animate opacity (scroll, on-visible, on-load).
fn scope_has_opacity_animation(scope: &ScopeBlock) -> bool {
    for fm in &scope.matches {
        if is_opacity_animation(fm) {
            return true;
        }
    }
    // Check nested scopes' matches too
    for ns in &scope.nested_scopes {
        if nested_has_opacity_animation(ns) {
            return true;
        }
    }
    false
}

fn nested_has_opacity_animation(ns: &crate::parser::NestedScope) -> bool {
    for fm in &ns.matches {
        if is_opacity_animation(fm) {
            return true;
        }
    }
    false
}

/// Check if a FormMatch represents an animation that targets opacity.
fn is_opacity_animation(fm: &FormMatch) -> bool {
    let name = fm.macro_name.as_str();
    // Common animation-driving macros
    if matches!(
        name,
        "scroll" | "on-visible" | "on-load" | "on" | "reduced-motion" | "loop"
    ) {
        // Check if any properties mention opacity
        if let Some(props) = fm.get_properties("properties") {
            return props.iter().any(|p| p.name == "opacity");
        }
        if let Some(keyframes) = fm.get_keyframes("keyframes") {
            return keyframes.iter().any(|kf| kf.property == "opacity");
        }
        // Animations without explicit properties may animate opacity generically
        return true;
    }
    false
}

// ============================================================
// W203: Untranslated Content
// ============================================================

/// Check for untranslated text content when @locale is declared.
fn check_untranslated_content(
    matches: &[FormMatch],
    html_content: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check if @locale is declared
    let has_locale = matches.iter().any(|fm| fm.macro_name == "locale");
    if !has_locale {
        return;
    }

    // Parse HTML content for text elements without data-t
    if let Some(html) = html_content {
        check_html_for_untranslated(html, diagnostics);
    }
}

/// Scan HTML for text elements that should have data-t attributes.
fn check_html_for_untranslated(html: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Simple regex-free HTML scanning for text content
    let mut pos = 0;
    let bytes = html.as_bytes();

    while pos < bytes.len() {
        // Find next opening tag
        if bytes[pos] == b'<' {
            if let Some(tag_end) = html[pos..].find('>') {
                let tag_content = &html[pos + 1..pos + tag_end];

                // Skip closing tags, comments, doctype
                if tag_content.starts_with('/')
                    || tag_content.starts_with('!')
                    || tag_content.starts_with('?')
                {
                    pos += tag_end + 1;
                    continue;
                }

                // Extract tag name
                let tag_name = tag_content
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_lowercase();

                // Skip excluded elements
                if matches!(
                    tag_name.as_str(),
                    "code" | "script" | "style" | "pre" | "svg" | "math"
                ) {
                    // Skip to closing tag
                    if let Some(close) = html[pos..].find(&format!("</{}>", tag_name)) {
                        pos += close + tag_name.len() + 3;
                        continue;
                    }
                }

                // Check for translate="no" or aria-hidden="true"
                let has_translate_no = tag_content.contains("translate=\"no\"")
                    || tag_content.contains("translate='no'");
                let has_aria_hidden = tag_content.contains("aria-hidden=\"true\"")
                    || tag_content.contains("aria-hidden='true'");
                let has_data_t = tag_content.contains("data-t");

                // Self-closing tags
                if tag_content.ends_with('/') {
                    // Check placeholder on input elements
                    if tag_name == "input"
                        && tag_content.contains("placeholder")
                        && !tag_content.contains("data-t-placeholder")
                    {
                        diagnostics.push(
                            Diagnostic::warning(
                                DiagnosticCode::W203,
                                format!(
                                    "Input placeholder without data-t-placeholder: <{}>",
                                    tag_content.chars().take(60).collect::<String>()
                                ),
                            )
                            .with_hint(
                                "Add data-t-placeholder attribute for locale translation"
                                    .to_string(),
                            ),
                        );
                    }
                    pos += tag_end + 1;
                    continue;
                }

                if has_translate_no || has_aria_hidden {
                    // Skip this element's content
                    if let Some(close) = html[pos..].find(&format!("</{}>", tag_name)) {
                        pos += close + tag_name.len() + 3;
                    } else {
                        pos += tag_end + 1;
                    }
                    continue;
                }

                // For text-bearing elements, check text content
                if is_text_element(&tag_name) && !has_data_t {
                    let after_tag = pos + tag_end + 1;
                    if let Some(close_pos) = html[after_tag..].find('<') {
                        let text = html[after_tag..after_tag + close_pos].trim();
                        // Skip text that is entirely HTML entities (e.g., &check; &amp;)
                        let is_all_entities = !text.is_empty() && {
                            let mut remaining = text;
                            let mut all_ent = true;
                            while !remaining.is_empty() {
                                let remaining_trimmed = remaining.trim_start();
                                if remaining_trimmed.is_empty() {
                                    break;
                                }
                                if remaining_trimmed.starts_with('&') {
                                    if let Some(semi) = remaining_trimmed.find(';') {
                                        remaining = &remaining_trimmed[semi + 1..];
                                    } else {
                                        all_ent = false;
                                        break;
                                    }
                                } else {
                                    all_ent = false;
                                    break;
                                }
                            }
                            all_ent
                        };

                        if !text.is_empty()
                            && text.len() > 1
                            && !text.starts_with('{')
                            && !text.starts_with("$")
                            && !is_all_entities
                        {
                            let preview: String = text.chars().take(40).collect();
                            diagnostics.push(
                                Diagnostic::warning(
                                    DiagnosticCode::W203,
                                    format!(
                                        "Untranslated text in <{}>: \"{}\"{}",
                                        tag_name,
                                        preview,
                                        if text.len() > 40 { "..." } else { "" }
                                    ),
                                )
                                .with_hint(
                                    "Add data-t attribute for locale translation".to_string(),
                                ),
                            );
                        }
                    }
                }

                pos += tag_end + 1;
            } else {
                pos += 1;
            }
        } else {
            pos += 1;
        }
    }
}

fn is_text_element(tag: &str) -> bool {
    matches!(
        tag,
        "h1" | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "p"
            | "span"
            | "a"
            | "button"
            | "label"
            | "li"
            | "td"
            | "th"
            | "dt"
            | "dd"
            | "figcaption"
            | "legend"
            | "summary"
            | "option"
            | "title"
    )
}

// ============================================================
// W204: Antipattern onclick
// ============================================================

/// Inline event handler attribute names that are antipatterns.
const INLINE_EVENT_ATTRS: &[&str] = &[
    "onclick",
    "onsubmit",
    "onchange",
    "onmouseover",
    "onmouseout",
    "onmousedown",
    "onmouseup",
    "onkeydown",
    "onkeyup",
    "onkeypress",
    "onfocus",
    "onblur",
    "oninput",
    "onload",
    "onscroll",
    "onresize",
    "ondblclick",
    "oncontextmenu",
    "ontouchstart",
    "ontouchend",
    "ontouchmove",
];

/// Check for inline event handler attributes in HTML.
fn check_onclick_antipattern(html_content: Option<&str>, diagnostics: &mut Vec<Diagnostic>) {
    let html = match html_content {
        Some(h) => h,
        None => return,
    };

    for attr in INLINE_EVENT_ATTRS {
        let pattern = format!("{}=", attr);
        let mut search_pos = 0;
        while let Some(found) = html[search_pos..].find(&pattern) {
            let abs_pos = search_pos + found;
            diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::W204,
                    format!(
                        "Inline event handler '{}' detected (see docs/antipatterns.md)",
                        attr
                    ),
                )
                .with_span(SourceSpan::new(abs_pos, abs_pos + attr.len()).into())
                .with_hint(format!("Use Spacetime @on bindings instead of {}", attr)),
            );
            search_pos = abs_pos + pattern.len();
        }
    }
}

// ============================================================
// W205: Unresolved locale template
// ============================================================

/// Check for {locale} templates that are not resolved.
fn check_unresolved_locale(
    matches: &[FormMatch],
    scopes: &[ScopeBlock],
    html_content: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let has_locale = matches.iter().any(|fm| fm.macro_name == "locale");

    // Collect all string values from CSS declarations and HTML
    let mut locale_refs: Vec<(String, SourceSpan)> = Vec::new();

    // Check CSS declarations in scopes
    for scope in scopes {
        collect_locale_refs_from_scope(scope, &mut locale_refs);
    }

    // Check HTML content
    if let Some(html) = html_content {
        collect_locale_refs_from_html(html, &mut locale_refs);
    }

    // Check FormMatches for {locale} in string values, but exclude %emit js blocks
    for fm in matches {
        if fm.macro_name == "emit" {
            // Skip %emit blocks — {locale} may be used in JS runtime context
            continue;
        }
        for val in fm.captures.values() {
            if let crate::syntax::CapturedValue::String(s) = val
                && s.contains("{locale}")
            {
                locale_refs.push((s.clone(), fm.span));
            }
        }
    }

    for (value, span) in &locale_refs {
        if value.contains("{locale}") && !has_locale {
            diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::W205,
                    format!(
                        "Unresolved {{locale}} template in value: \"{}\"",
                        value.chars().take(60).collect::<String>()
                    ),
                )
                .with_span((*span).into())
                .with_hint(
                    "Declare @locale to enable locale resolution, or remove {locale} reference"
                        .to_string(),
                ),
            );
        }
    }
}

fn collect_locale_refs_from_scope(scope: &ScopeBlock, refs: &mut Vec<(String, SourceSpan)>) {
    for decl in &scope.css_declarations {
        if decl.value.contains("{locale}") {
            refs.push((decl.value.clone(), scope.span));
        }
    }
    for ns in &scope.nested_scopes {
        collect_locale_refs_from_nested(ns, refs);
    }
}

fn collect_locale_refs_from_nested(
    ns: &crate::parser::NestedScope,
    refs: &mut Vec<(String, SourceSpan)>,
) {
    for decl in &ns.css_declarations {
        if decl.value.contains("{locale}") {
            refs.push((decl.value.clone(), ns.span));
        }
    }
    for child in &ns.nested_scopes {
        collect_locale_refs_from_nested(child, refs);
    }
}

fn collect_locale_refs_from_html(html: &str, refs: &mut Vec<(String, SourceSpan)>) {
    // Check src=, href=, and other URL-bearing attributes for {locale}
    let url_attrs = ["src=", "href=", "action=", "data-src="];
    for attr in &url_attrs {
        let mut pos = 0;
        while let Some(found) = html[pos..].find(attr) {
            let abs = pos + found + attr.len();
            if abs < html.len() {
                let quote = html.as_bytes()[abs];
                if (quote == b'"' || quote == b'\'')
                    && let Some(end) = html[abs + 1..].find(quote as char)
                {
                    let val = &html[abs + 1..abs + 1 + end];
                    if val.contains("{locale}") {
                        refs.push((val.to_string(), SourceSpan::new(abs, abs + 1 + end)));
                    }
                }
            }
            pos = abs + 1;
        }
    }
}

// ============================================================
// Public Entry Point
// ============================================================

/// Main entry point. Analyze visual consistency and return diagnostics.
pub fn analyze_visual(
    scopes: &[ScopeBlock],
    matches: &[FormMatch],
    html_content: Option<&str>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    check_low_contrast(scopes, &mut diagnostics);
    check_invisible_elements(scopes, &mut diagnostics);
    check_untranslated_content(matches, html_content, &mut diagnostics);
    check_onclick_antipattern(html_content, &mut diagnostics);
    check_unresolved_locale(matches, scopes, html_content, &mut diagnostics);

    diagnostics
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::CssDeclaration;
    use std::collections::HashMap;

    fn make_scope(selector: &str, css: Vec<(&str, &str)>) -> ScopeBlock {
        ScopeBlock {
            kind: Default::default(),
            selector: selector.to_string(),
            behavior: Default::default(),
            css_declarations: css
                .into_iter()
                .map(|(p, v)| CssDeclaration {
                    property: p.to_string(),
                    value: v.to_string(),
                    is_injection: false,
                    span: SourceSpan::default(),
                })
                .collect(),
            form_refs: Vec::new(),
            nested_scopes: vec![],
            matches: vec![],
            span: SourceSpan::default(),
            source_file: None,
            exports: vec![],
            refs: vec![],
            states: vec![],
            html: String::new(),
        }
    }

    fn make_fm(macro_name: &str) -> FormMatch {
        FormMatch {
            macro_name: macro_name.to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    fn make_fm_with_string(macro_name: &str, key: &str, val: &str) -> FormMatch {
        let mut captures = HashMap::new();
        captures.insert(
            key.to_string(),
            crate::syntax::CapturedValue::String(val.to_string()),
        );
        FormMatch {
            macro_name: macro_name.to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    // ---- W201 tests ----

    #[test]
    fn test_w201_low_contrast_detected() {
        // White text on white background — ratio ~1:1
        let scopes = vec![make_scope(
            ".bad",
            vec![("color", "#ffffff"), ("background-color", "#fefefe")],
        )];
        let diags = analyze_visual(&scopes, &[], None);
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W201),
            "Expected W201 for low contrast, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w201_good_contrast_passes() {
        // Black text on white background — ratio 21:1
        let scopes = vec![make_scope(
            ".good",
            vec![("color", "#000000"), ("background-color", "#ffffff")],
        )];
        let diags = analyze_visual(&scopes, &[], None);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W201),
            "Expected no W201 for good contrast, got: {:?}",
            diags
        );
    }

    // ---- W202 tests ----

    #[test]
    fn test_w202_opacity_zero_without_reveal() {
        let scopes = vec![make_scope(".hidden", vec![("opacity", "0")])];
        let diags = analyze_visual(&scopes, &[], None);
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W202),
            "Expected W202 for opacity:0 without reveal, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w202_opacity_zero_with_scroll_passes() {
        let mut scope = make_scope(".reveal", vec![("opacity", "0")]);
        scope.matches.push(make_fm("scroll"));
        let scopes = vec![scope];
        let diags = analyze_visual(&scopes, &[], None);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W202),
            "Expected no W202 when scroll animation present, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w202_loop_animation_suppresses_warning() {
        let mut scope = make_scope(".pulse", vec![("opacity", "0")]);
        scope.matches.push(make_fm("loop"));
        let scopes = vec![scope];
        let diags = analyze_visual(&scopes, &[], None);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W202),
            "Expected no W202 when loop animation present, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w202_scroll_animation_suppresses_warning() {
        let mut scope = make_scope(".reveal", vec![("opacity", "0")]);
        scope.matches.push(make_fm("scroll"));
        let scopes = vec![scope];
        let diags = analyze_visual(&scopes, &[], None);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W202),
            "Expected no W202 when scroll animation present, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w202_nested_scroll_suppresses_warning() {
        let mut scope = make_scope(".parent", vec![("opacity", "0")]);
        // Add a nested scope with a scroll animation
        scope.nested_scopes.push(crate::parser::NestedScope {
            kind: Default::default(),
            selector: ".child".to_string(),
            composed_selector: ".child".to_string(),
            behavior: Default::default(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![],
            matches: vec![make_fm("scroll")],
            collection_ref: None,
            span: SourceSpan::default(),
        });
        let scopes = vec![scope];
        let diags = analyze_visual(&scopes, &[], None);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W202),
            "Expected no W202 when nested scope has scroll animation, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w202_grouped_selector_scroll_suppresses() {
        let mut scope = make_scope(".a, .b", vec![("opacity", "0")]);
        scope.matches.push(make_fm("scroll"));
        let scopes = vec![scope];
        let diags = analyze_visual(&scopes, &[], None);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W202),
            "Expected no W202 for grouped selector with scroll animation, got: {:?}",
            diags
        );
    }

    // ---- W203 tests ----

    #[test]
    fn test_w203_untranslated_text_detected() {
        let matches = vec![make_fm("locale")];
        let html = r#"<h1>Hello World</h1>"#;
        let diags = analyze_visual(&[], &matches, Some(html));
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W203),
            "Expected W203 for untranslated text, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w203_translated_text_passes() {
        let matches = vec![make_fm("locale")];
        let html = r#"<h1 data-t="greeting">Hello World</h1>"#;
        let diags = analyze_visual(&[], &matches, Some(html));
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W203),
            "Expected no W203 for translated text, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w203_html_entity_not_flagged() {
        let matches = vec![make_fm("locale")];
        let html = r#"<p>&check;</p>"#;
        let diags = analyze_visual(&[], &matches, Some(html));
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W203),
            "Expected no W203 for HTML entity text, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w203_mixed_entity_and_text_still_flags() {
        let matches = vec![make_fm("locale")];
        let html = r#"<p>Hello &check; World</p>"#;
        let diags = analyze_visual(&[], &matches, Some(html));
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W203),
            "Expected W203 for mixed entity+text, got: {:?}",
            diags
        );
    }

    // ---- W204 tests ----

    #[test]
    fn test_w204_onclick_detected() {
        let html = r#"<button onclick="doSomething()">Click</button>"#;
        let diags = analyze_visual(&[], &[], Some(html));
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W204),
            "Expected W204 for onclick, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w204_no_onclick_passes() {
        let html = r#"<button class="action">Click</button>"#;
        let diags = analyze_visual(&[], &[], Some(html));
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W204),
            "Expected no W204 without inline handlers, got: {:?}",
            diags
        );
    }

    // ---- W205 tests ----

    #[test]
    fn test_w205_unresolved_locale_detected() {
        // {locale} in a string value but no @locale declared
        let matches = vec![make_fm_with_string(
            "data-fetch",
            "url",
            "/api/{locale}/data",
        )];
        let diags = analyze_visual(&[], &matches, None);
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W205),
            "Expected W205 for unresolved locale, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_w205_locale_in_emit_js_passes() {
        // {locale} inside %emit block should be ignored
        let matches = vec![make_fm_with_string(
            "emit",
            "body",
            "const url = `/${locale}/api`",
        )];
        let diags = analyze_visual(&[], &matches, None);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W205),
            "Expected no W205 for locale in emit block, got: {:?}",
            diags
        );
    }
}
