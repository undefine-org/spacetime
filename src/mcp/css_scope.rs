//! CSS scope-wrapping + lint for region-mounted bundles (PLAN-041).
//!
//! Each bundle mounted into the unified MCP workbench is CSS-confined to its
//! region by wrapping the bundle's extracted CSS in a native CSS `@scope` block:
//!
//! ```text
//! @scope ([data-st-region="{region_id}"]) {
//!     /* ...bundle css, verbatim... */
//! }
//! ```
//!
//! [`wrap_bundle_css`] performs that wrapping — a single outer `@scope` block
//! is added and the bundle css (with its own braces) is inserted verbatim.
//! [`lint_bundle_css`] flags the constructs that `@scope` *cannot* confine and
//! which therefore leak across regions or touch page chrome: top-level
//! `@keyframes` (global even inside `@scope`) and any rule whose selector
//! targets `:root` / `html` / `body`.
//!
//! This is a pure utility. FEAT-126's `registerBundle` calls these at
//! registration time; no Spacetime primitive lives here.

use std::fmt;

/// Kind of [`CssLintWarning`] emitted by [`lint_bundle_css`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssLintKind {
    /// `@keyframes` is global even when nested inside `@scope`, so a bundle's
    /// keyframes escape its region. Define them in the unscoped workbench chrome.
    Keyframes,
    /// A rule targets `:root`, `html`, or `body` — page chrome that either leaks
    /// custom properties across regions or restyles the host page.
    ChromeSelector,
}

impl fmt::Display for CssLintKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CssLintKind::Keyframes => f.write_str("global-keyframes"),
            CssLintKind::ChromeSelector => f.write_str("chrome-selector"),
        }
    }
}

/// A structured warning emitted by [`lint_bundle_css`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssLintWarning {
    /// Machine-readable warning category.
    pub kind: CssLintKind,
    /// Human-readable explanation.
    pub message: String,
    /// Trimmed source snippet of the offending construct.
    pub snippet: String,
}

/// Wrap a bundle's CSS in a native CSS `@scope` block bound to its region.
///
/// The bundle css is inserted verbatim — its own braces are left untouched — and
/// exactly one outer `@scope ([data-st-region="{region_id}"]) { … }` block is
/// added. Empty css yields a harmless empty scope block. `region_id` is escaped
/// for safe interpolation into a CSS attribute-selector string.
pub fn wrap_bundle_css(css: &str, region_id: &str) -> String {
    let selector_value = escape_attr_value(region_id);
    if css.trim().is_empty() {
        return format!("@scope ([data-st-region=\"{selector_value}\"]) {{}}");
    }
    format!("@scope ([data-st-region=\"{selector_value}\"]) {{\n{css}\n}}")
}

/// Lint bundle CSS for constructs that `@scope` cannot confine.
///
/// Emits [`CssLintWarning`]s for:
/// * top-level `@keyframes` (global even inside `@scope`, so the animation
///   escapes its region);
/// * any rule — at any nesting depth, e.g. inside `@media` — whose selector
///   targets `:root`, `html`, or `body` (leaks custom properties across regions
///   or touches page chrome).
///
/// The scanner is string- and comment-aware, so braces inside `"…"` / `'…'`
/// literals or `/* … */` comments do not corrupt nesting tracking.
pub fn lint_bundle_css(css: &str) -> Vec<CssLintWarning> {
    let mut warnings = Vec::new();
    for (_depth, prelude, _body) in collect_rules(css) {
        let lower = prelude.to_ascii_lowercase();
        if lower.starts_with('@') && lower.contains("keyframes") {
            warnings.push(CssLintWarning {
                kind: CssLintKind::Keyframes,
                message: "@keyframes is global even inside @scope; move keyframes into \
                          the workbench chrome so they do not escape the region"
                    .to_string(),
                snippet: prelude.clone(),
            });
            continue;
        }
        if targets_chrome(&prelude) {
            warnings.push(CssLintWarning {
                kind: CssLintKind::ChromeSelector,
                message: "rule targets :root/html/body and would touch page chrome or \
                          leak custom properties across regions; bundles must not style \
                          chrome"
                    .to_string(),
                snippet: prelude.clone(),
            });
        }
    }
    warnings
}

// ─── brace-aware rule scanner ────────────────────────────────────────────────

/// Every CSS rule block in `src` at any nesting depth, in source order, as
/// `(depth, trimmed_prelude, trimmed_body)`. `prelude` is the text up to `{`
/// (a selector list or at-rule prelude); `body` is the text between `{` and its
/// matching `}`. Strings and comments are skipped so their braces don't count.
fn collect_rules(src: &str) -> Vec<(usize, String, String)> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    walk(&chars, 0, chars.len(), 0, &mut out);
    out
}

/// If a CSS comment (`/* … */`) or string (`"…"` / `'…'`) starts at `pos`,
/// return the index just past it; otherwise `None`.
fn skip_obstacle(chars: &[char], pos: usize, end: usize) -> Option<usize> {
    if pos >= end {
        return None;
    }
    // Comment: /* … */ (runs to end if never closed).
    if chars[pos] == '/' && pos + 1 < end && chars[pos + 1] == '*' {
        let mut j = pos + 2;
        while j + 1 < end && !(chars[j] == '*' && chars[j + 1] == '/') {
            j += 1;
        }
        return Some(if j + 1 < end { j + 2 } else { end });
    }
    // String: " … " or ' … ' (honour `\` escapes; run to end if never closed).
    let quote = chars[pos];
    if quote == '"' || quote == '\'' {
        let mut j = pos + 1;
        while j < end {
            if chars[j] == '\\' && j + 1 < end {
                j += 2;
                continue;
            }
            if chars[j] == quote {
                return Some(j + 1);
            }
            j += 1;
        }
        return Some(end);
    }
    None
}

/// Recursive brace walk: collect every `{ … }` block between `start` and `end`
/// at `depth`, recursing into each block's body for nested rules.
fn walk(
    chars: &[char],
    start: usize,
    end: usize,
    depth: usize,
    out: &mut Vec<(usize, String, String)>,
) {
    let mut i = start;
    let mut prelude_start = start;
    while i < end {
        if let Some(next) = skip_obstacle(chars, i, end) {
            i = next;
            continue;
        }
        if chars[i] == '{' {
            let prelude: String = chars[prelude_start..i].iter().collect();
            let body_start = i + 1;
            // Find the matching close brace, skipping strings/comments.
            let mut j = body_start;
            let mut nesting = 1i32;
            while j < end && nesting > 0 {
                if let Some(next) = skip_obstacle(chars, j, end) {
                    j = next;
                    continue;
                }
                match chars[j] {
                    '{' => nesting += 1,
                    '}' => {
                        nesting -= 1;
                        if nesting == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            // `j` is the matching `}` (or `end` if unbalanced).
            let body: String = chars[body_start..j].iter().collect();
            out.push((depth, prelude.trim().to_string(), body.trim().to_string()));
            // Recurse to collect nested rules within this block.
            walk(chars, body_start, j, depth + 1, out);
            i = if j < end { j + 1 } else { end };
            prelude_start = i;
            continue;
        }
        i += 1;
    }
}

// ─── chrome-selector detection ───────────────────────────────────────────────

/// Does this rule prelude target page chrome (`:root`, `html`, `body`) in any
/// of its comma-separated selectors?
fn targets_chrome(prelude: &str) -> bool {
    split_selector_list(prelude)
        .iter()
        .any(|sel| sel_targets_chrome(sel))
}

/// True if a single (top-level-comma-free) selector targets `:root`, `html`,
/// or `body`.
fn sel_targets_chrome(sel: &str) -> bool {
    if sel.contains(":root") {
        return true;
    }
    // Split into compound selectors on whitespace / combinator chars, then test
    // each compound's leading type selector against html/body.
    for compound in sel.split(|c: char| c.is_whitespace() || matches!(c, '>' | '+' | '~')) {
        let type_sel = leading_type_selector(compound.trim_start());
        if type_sel.eq_ignore_ascii_case("html") || type_sel.eq_ignore_ascii_case("body") {
            return true;
        }
    }
    false
}

/// The leading type selector of a compound (`html`, `body`, `div`, …) or `""`
/// when the compound has no type selector (starts with `.`, `:`, `#`, `[`,
/// `*`, …).
fn leading_type_selector(compound: &str) -> String {
    let mut out = String::new();
    for (idx, c) in compound.char_indices() {
        if idx == 0 {
            if !c.is_ascii_alphabetic() {
                break;
            }
        } else if !(c.is_ascii_alphanumeric() || c == '-') {
            break;
        }
        out.push(c);
    }
    out
}

/// Split a CSS selector list on top-level commas, ignoring commas nested inside
/// `()` / `[]` (e.g. within `:is(:root, html)`).
fn split_selector_list(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                parts.push(cur.trim().to_string());
                cur = String::new();
            }
            _ => cur.push(c),
        }
    }
    parts.push(cur.trim().to_string());
    parts
}

/// Escape `s` for safe interpolation into a CSS attribute-selector value
/// (`[attr="…"]`): neutralise `"`, `\`, and line-break characters.
fn escape_attr_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\n' | '\r' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── wrap_bundle_css ──────────────────────────────────────────────────────

    #[test]
    fn wrap_produces_valid_scope_block() {
        let css = ".button { color: red; padding: 4px 8px; }";
        let out = wrap_bundle_css(css, "main");

        // Right region selector header.
        assert!(
            out.starts_with("@scope ([data-st-region=\"main\"]) {"),
            "missing/incorrect @scope header: {out}"
        );
        // The bundle css survives verbatim inside the wrapper.
        assert!(
            out.contains(css),
            "bundle css not preserved verbatim: {out}"
        );
        // Exactly one outer brace pair is added; inner braces untouched.
        assert_eq!(
            out.matches('{').count(),
            css.matches('{').count() + 1,
            "open-brace count off: {out}"
        );
        assert_eq!(
            out.matches('}').count(),
            css.matches('}').count() + 1,
            "close-brace count off: {out}"
        );
        assert!(out.ends_with('}'), "wrapper not closed: {out}");
    }

    #[test]
    fn wrap_handles_nested_braces() {
        let css = "@media (max-width: 600px) { .x { color: red; } .y { color: blue; } }";
        let out = wrap_bundle_css(css, "nav");
        assert!(out.contains("@scope ([data-st-region=\"nav\"]) {"));
        assert!(out.contains(css));
        assert_eq!(out.matches('{').count(), css.matches('{').count() + 1);
        assert_eq!(out.matches('}').count(), css.matches('}').count() + 1);
    }

    #[test]
    fn wrap_empty_css_is_harmless() {
        let out = wrap_bundle_css("   \n\t  ", "stage");
        assert!(out.contains("@scope ([data-st-region=\"stage\"])"));
        // A single balanced empty block — no stray inner content.
        assert_eq!(out.matches('{').count(), 1);
        assert_eq!(out.matches('}').count(), 1);
    }

    #[test]
    fn wrap_two_bundles_confine_distinct_scopes() {
        let a = wrap_bundle_css(".button { color: red; }", "alpha");
        let b = wrap_bundle_css(".button { color: blue; }", "beta");

        // Each bundle carries its own region scope header.
        assert!(a.contains("@scope ([data-st-region=\"alpha\"]) {"));
        assert!(b.contains("@scope ([data-st-region=\"beta\"]) {"));
        assert_ne!(a, b, "two bundles must wrap to distinct scopes");

        // The SAME `.button` selector lands under DIFFERENT @scope contexts, so
        // CSS @scope confines each to its own region subtree — no collision.
        assert!(a.contains(".button { color: red; }"));
        assert!(b.contains(".button { color: blue; }"));

        // Each output is exactly one @scope wrapper.
        assert_eq!(a.matches("@scope").count(), 1);
        assert_eq!(b.matches("@scope").count(), 1);
    }

    #[test]
    fn wrap_escapes_region_id_special_chars() {
        let out = wrap_bundle_css(".x { color: red; }", "a\"b");
        // The embedded `"` is escaped so the attribute selector stays well-formed.
        assert!(
            out.contains(r#"[data-st-region="a\"b"]"#),
            "region id not escaped: {out}"
        );
    }

    // ── lint_bundle_css ──────────────────────────────────────────────────────

    #[test]
    fn lint_clean_css_is_silent() {
        let css = ".card { color: red; } .card:hover { color: blue; }";
        assert!(
            lint_bundle_css(css).is_empty(),
            "clean bundle css should not warn"
        );
    }

    #[test]
    fn lint_flags_keyframes_and_root_vars() {
        let css = "\
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
:root { --brand: #e0915a; }
body { margin: 0; }
";
        let warnings = lint_bundle_css(css);
        let kinds: Vec<CssLintKind> = warnings.iter().map(|w| w.kind).collect();

        // One keyframes warning whose snippet is the @keyframes at-rule.
        let kf = warnings
            .iter()
            .find(|w| w.kind == CssLintKind::Keyframes)
            .expect("expected a keyframes warning");
        assert!(
            kf.snippet.to_ascii_lowercase().contains("@keyframes"),
            "keyframes snippet wrong: {}",
            kf.snippet
        );

        // Chrome warnings for :root (custom props) and body.
        let chrome_snippets: Vec<&str> = warnings
            .iter()
            .filter(|w| w.kind == CssLintKind::ChromeSelector)
            .map(|w| w.snippet.as_str())
            .collect();
        assert!(
            chrome_snippets.iter().any(|s| s.contains(":root")),
            "expected :root chrome warning, got {chrome_snippets:?}"
        );
        assert!(
            chrome_snippets.iter().any(|s| s.contains("body")),
            "expected body chrome warning, got {chrome_snippets:?}"
        );

        // No spurious warnings for the inner `from`/`to` keyframe stops.
        assert!(
            !warnings
                .iter()
                .any(|w| w.snippet == "from" || w.snippet == "to"),
            "inner keyframe stops must not be flagged: {warnings:?}"
        );
        // Sanity: we saw both kinds.
        assert!(kinds.contains(&CssLintKind::Keyframes));
        assert!(kinds.contains(&CssLintKind::ChromeSelector));
    }

    #[test]
    fn lint_flags_keyframes_even_nested_in_media() {
        // @keyframes is global regardless of where it syntactically appears.
        let css = "@media (min-width: 1px) { @keyframes pulse { 0% { opacity: 1; } 100% { opacity: 0; } } }";
        let warnings = lint_bundle_css(css);
        assert!(
            warnings.iter().any(|w| w.kind == CssLintKind::Keyframes),
            "nested @keyframes must still warn: {warnings:?}"
        );
    }

    #[test]
    fn lint_flags_html_and_combinator_selectors() {
        // `html, body` is one rule; `div > html` is another with html in a chain.
        let css = "html, body { font-family: serif; } div > html { display: none; }";
        let warnings = lint_bundle_css(css);
        let chrome = warnings
            .iter()
            .filter(|w| w.kind == CssLintKind::ChromeSelector)
            .count();
        assert_eq!(chrome, 2, "expected 2 chrome rules flagged: {warnings:?}");
    }

    #[test]
    fn lint_ignores_class_names_resembling_chrome() {
        // `.html` / `.body` / `.root` are classes, NOT chrome type selectors.
        let css = ".html { color: red; } .body { color: blue; } .root { color: green; }";
        let warnings = lint_bundle_css(css);
        assert!(
            warnings.is_empty(),
            "classes named like chrome must not warn: {warnings:?}"
        );
    }

    #[test]
    fn lint_ignores_braces_in_strings_and_comments() {
        let css = r#"
.card { content: "}"; /* a } stray brace in a comment */ color: red; }
.button { content: '{'; }
"#;
        let warnings = lint_bundle_css(css);
        assert!(
            warnings.is_empty(),
            "braces in strings/comments must not fool the scanner: {warnings:?}"
        );
    }
}
