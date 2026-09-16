//! Timeline lint system for scroll animation analysis.
//!
//! Detects timing issues in scroll-driven animations:
//! - W101: Stacked visibility overlap (cross-element)
//! - W110: Missing out-animation for scroll waypoint
//! - W111: Same-target scroll range overlap
//! - W112: Invalid scroll range (start >= end)
//! - W113: Insufficient read time for content section
//! - W114: No hold time (content never fully visible)

use serde::{Deserialize, Serialize};

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::parser::{ScopeBlock, SourceSpan};
use crate::syntax::{CapturedValue, FormMatch};

// =============================================================================
// Configuration
// =============================================================================

/// Top-level configuration for the timeline linter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TimelineLintConfig {
    pub enabled: bool,
    pub rules: TimelineLintRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct TimelineLintRules {
    pub stacked_overlap: StackedOverlapRule,
    pub missing_out: MissingOutRule,
    pub same_target_overlap: SameTargetOverlapRule,
    pub invalid_range: InvalidRangeRule,
    pub readability: ReadabilityRule,
}

/// W101 — Stacked visibility overlap between absolute-positioned elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StackedOverlapRule {
    pub enabled: bool,
    /// Maximum allowed overlap as fraction of scroll range (0.0-1.0).
    pub max_overlap: f64,
}

/// W110 — Missing out-animation for a scroll waypoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MissingOutRule {
    pub enabled: bool,
    /// Skip warning for the last waypoint in a sequence.
    pub allow_last_element: bool,
}

/// W111 — Same target+property scroll range overlap.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SameTargetOverlapRule {
    pub enabled: bool,
    pub max_overlap: f64,
}

/// W112 — Invalid scroll range (start >= end).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InvalidRangeRule {
    pub enabled: bool,
}

/// W113/W114 — Readability: minimum visible/hold duration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadabilityRule {
    pub enabled: bool,
    /// Minimum visible scroll fraction for a content section.
    pub min_visible_duration: f64,
    /// Minimum hold duration (fully visible, not transitioning).
    pub min_hold_duration: f64,
}

impl Default for TimelineLintConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: TimelineLintRules::default(),
        }
    }
}

impl Default for StackedOverlapRule {
    fn default() -> Self {
        Self {
            enabled: true,
            max_overlap: 0.02,
        }
    }
}

impl Default for MissingOutRule {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_last_element: true,
        }
    }
}

impl Default for SameTargetOverlapRule {
    fn default() -> Self {
        Self {
            enabled: true,
            max_overlap: 0.0,
        }
    }
}

impl Default for InvalidRangeRule {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Default for ReadabilityRule {
    fn default() -> Self {
        Self {
            enabled: true,
            min_visible_duration: 0.08,
            min_hold_duration: 0.03,
        }
    }
}

// =============================================================================
// Data Structures
// =============================================================================

/// A scroll animation span collected from the AST.
#[derive(Debug, Clone)]
pub struct ScrollAnimationSpan {
    pub timeline_name: String,
    pub parent_selector: String,
    pub target_selector: Option<String>,
    pub property: String,
    pub scroll_start: f64,
    pub scroll_end: f64,
    pub from_value: Option<String>,
    pub to_value: Option<String>,
    pub span: SourceSpan,
}

/// Computed visibility window for an element.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct VisibilityWindow {
    /// Grouping key: "parent > target"
    target_key: String,
    parent_selector: String,
    /// When element first becomes visible (start of fade-in)
    visible_start: f64,
    /// When element becomes fully invisible (end of fade-out, or 1.0 if no out)
    visible_end: f64,
    /// When element is fully opaque (end of fade-in)
    fully_visible_start: f64,
    /// When element starts fading out (start of fade-out)
    fully_visible_end: f64,
    /// Hold duration = fully_visible_end - fully_visible_start
    hold_duration: f64,
    has_out: bool,
    in_span: SourceSpan,
}

// =============================================================================
// Linter Engine
// =============================================================================

struct TimelineLinter {
    config: TimelineLintConfig,
    spans: Vec<ScrollAnimationSpan>,
    diagnostics: Vec<Diagnostic>,
}

impl TimelineLinter {
    fn new(config: TimelineLintConfig) -> Self {
        Self {
            config,
            spans: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Collect scroll animation spans from all scopes.
    fn collect_from_scopes(&mut self, scopes: &[ScopeBlock]) {
        for scope in scopes {
            self.collect_from_scope(scope);
        }
    }

    fn collect_from_scope(&mut self, scope: &ScopeBlock) {
        for fm in &scope.matches {
            if fm.macro_name == "scroll" {
                self.collect_scroll_fm(&scope.selector, fm);
            }
        }
        // Recurse into nested scopes
        for nested in &scope.nested_scopes {
            let nested_selector = format!("{} {}", scope.selector, nested.selector);
            for fm in &nested.matches {
                if fm.macro_name == "scroll" {
                    self.collect_scroll_fm(&nested_selector, fm);
                }
            }
        }
    }

    /// Extract ScrollAnimationSpans from a single @scroll FormMatch.
    fn collect_scroll_fm(&mut self, parent_selector: &str, fm: &FormMatch) {
        let name = fm.get_ident("name").unwrap_or("unnamed").to_string();
        let start = fm.get_number("start").unwrap_or(0.0);
        let end = fm.get_number("end").unwrap_or(1.0);

        // Try to extract body content in various formats
        match fm.get("body") {
            Some(CapturedValue::Keyframes(keyframes)) => {
                // Flat keyframes: target is the parent scope itself
                for kf in keyframes {
                    let (from_val, to_val) = extract_from_to(&kf.values);
                    self.spans.push(ScrollAnimationSpan {
                        timeline_name: name.clone(),
                        parent_selector: parent_selector.to_string(),
                        target_selector: None,
                        property: kf.property.clone(),
                        scroll_start: start,
                        scroll_end: end,
                        from_value: from_val,
                        to_value: to_val,
                        span: fm.span,
                    });
                }
            }
            Some(CapturedValue::StyleProperties(props)) if !props.is_empty() => {
                // Scoped body: "selector property" → "value"
                for (key, value) in props {
                    if let Some(last_space) = key.rfind(' ') {
                        let selector = &key[..last_space];
                        let property = &key[last_space + 1..];
                        let (from_val, to_val) = parse_arrow_value(value);
                        self.spans.push(ScrollAnimationSpan {
                            timeline_name: name.clone(),
                            parent_selector: parent_selector.to_string(),
                            target_selector: Some(selector.to_string()),
                            property: property.to_string(),
                            scroll_start: start,
                            scroll_end: end,
                            from_value: from_val,
                            to_value: to_val,
                            span: fm.span,
                        });
                    } else {
                        // Root-level property
                        let (from_val, to_val) = parse_arrow_value(value);
                        self.spans.push(ScrollAnimationSpan {
                            timeline_name: name.clone(),
                            parent_selector: parent_selector.to_string(),
                            target_selector: None,
                            property: key.clone(),
                            scroll_start: start,
                            scroll_end: end,
                            from_value: from_val,
                            to_value: to_val,
                            span: fm.span,
                        });
                    }
                }
            }
            _ => {
                // Body format not recognized; record as a generic span
                self.spans.push(ScrollAnimationSpan {
                    timeline_name: name.clone(),
                    parent_selector: parent_selector.to_string(),
                    target_selector: None,
                    property: String::new(),
                    scroll_start: start,
                    scroll_end: end,
                    from_value: None,
                    to_value: None,
                    span: fm.span,
                });
            }
        }
    }

    /// Run all enabled lint rules.
    fn run_all_rules(&mut self) {
        self.check_invalid_ranges();
        self.check_same_target_overlap();
        self.check_missing_out();
        self.check_stacked_overlap();
        self.check_readability();
    }

    /// W112: Invalid scroll range (start >= end).
    fn check_invalid_ranges(&mut self) {
        if !self.config.rules.invalid_range.enabled {
            return;
        }
        // Deduplicate: only warn once per timeline name
        let mut seen = std::collections::HashSet::new();
        for span in &self.spans {
            if span.scroll_start >= span.scroll_end && seen.insert(span.timeline_name.clone()) {
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W112,
                        format!(
                            "Scroll animation '{}' has invalid range: start ({}) >= end ({})",
                            span.timeline_name, span.scroll_start, span.scroll_end
                        ),
                    )
                    .with_span(span.span.into())
                    .with_hint("Scroll start must be less than end".to_string()),
                );
            }
        }
    }

    /// W111: Same-target scroll range overlap.
    fn check_same_target_overlap(&mut self) {
        if !self.config.rules.same_target_overlap.enabled {
            return;
        }
        let max_overlap = self.config.rules.same_target_overlap.max_overlap;

        // Group by (parent_selector, target_selector, property)
        let mut groups: std::collections::HashMap<String, Vec<&ScrollAnimationSpan>> =
            std::collections::HashMap::new();

        for span in &self.spans {
            if span.property.is_empty() || span.property == "easing" || span.property == "range" {
                continue;
            }
            let key = format!(
                "{}>>{}>>{}",
                span.parent_selector,
                span.target_selector.as_deref().unwrap_or("&"),
                span.property
            );
            groups.entry(key).or_default().push(span);
        }

        for spans in groups.values() {
            if spans.len() < 2 {
                continue;
            }
            let mut sorted: Vec<&&ScrollAnimationSpan> = spans.iter().collect();
            sorted.sort_by(|a, b| {
                a.scroll_start
                    .partial_cmp(&b.scroll_start)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for i in 0..sorted.len() - 1 {
                let a = sorted[i];
                let b = sorted[i + 1];
                let overlap = a.scroll_end - b.scroll_start;
                if overlap > max_overlap {
                    self.diagnostics.push(
                        Diagnostic::warning(
                            DiagnosticCode::W111,
                            format!(
                                "Scroll animations '{}' and '{}' overlap by {:.1}% on property '{}'",
                                a.timeline_name,
                                b.timeline_name,
                                overlap * 100.0,
                                a.property
                            ),
                        )
                        .with_span(b.span.into())
                        .with_hint(format!(
                            "Ensure '{}' ends ({}) before '{}' starts ({})",
                            a.timeline_name, a.scroll_end, b.timeline_name, b.scroll_start
                        )),
                    );
                }
            }
        }
    }

    /// W110: Missing out-animation for scroll waypoint.
    fn check_missing_out(&mut self) {
        if !self.config.rules.missing_out.enabled {
            return;
        }

        // Find opacity in-animations (opacity: 0 -> 1) and check for corresponding out
        let opacity_spans: Vec<&ScrollAnimationSpan> = self
            .spans
            .iter()
            .filter(|s| s.property == "opacity")
            .collect();

        // Group by (parent_selector, target_selector)
        let mut by_target: std::collections::HashMap<String, Vec<&ScrollAnimationSpan>> =
            std::collections::HashMap::new();

        for span in &opacity_spans {
            let key = format!(
                "{}>>{}",
                span.parent_selector,
                span.target_selector.as_deref().unwrap_or("&")
            );
            by_target.entry(key).or_default().push(span);
        }

        for spans in by_target.values() {
            // Classify as in (0→1) or out (1→0)
            let mut ins: Vec<&ScrollAnimationSpan> = Vec::new();
            let mut outs: Vec<&ScrollAnimationSpan> = Vec::new();

            for span in spans {
                if is_fade_in(span) {
                    ins.push(span);
                } else if is_fade_out(span) {
                    outs.push(span);
                }
            }

            // Sort by scroll_start
            ins.sort_by(|a, b| {
                a.scroll_start
                    .partial_cmp(&b.scroll_start)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // For each in-animation, check if there's a corresponding out
            for (i, in_span) in ins.iter().enumerate() {
                let is_last = i == ins.len() - 1;
                if self.config.rules.missing_out.allow_last_element && is_last {
                    continue;
                }
                let has_out = outs
                    .iter()
                    .any(|out| out.scroll_start >= in_span.scroll_end);
                if !has_out {
                    self.diagnostics.push(
                        Diagnostic::warning(
                            DiagnosticCode::W110,
                            format!(
                                "Scroll animation '{}' fades in but has no corresponding fade-out",
                                in_span.timeline_name
                            ),
                        )
                        .with_span(in_span.span.into())
                        .with_hint(
                            "Add an out-animation to fade the element out before the next waypoint"
                                .to_string(),
                        ),
                    );
                }
            }
        }
    }

    /// W101: Stacked visibility overlap between elements.
    fn check_stacked_overlap(&mut self) {
        if !self.config.rules.stacked_overlap.enabled {
            return;
        }
        let max_overlap = self.config.rules.stacked_overlap.max_overlap;

        // Build visibility windows from opacity animations
        let windows = self.build_visibility_windows();

        // Group by parent_selector (elements in the same container)
        let mut by_parent: std::collections::HashMap<String, Vec<&VisibilityWindow>> =
            std::collections::HashMap::new();
        for window in &windows {
            by_parent
                .entry(window.parent_selector.clone())
                .or_default()
                .push(window);
        }

        for windows in by_parent.values() {
            if windows.len() < 2 {
                continue;
            }
            let mut sorted: Vec<&&VisibilityWindow> = windows.iter().collect();
            sorted.sort_by(|a, b| {
                a.visible_start
                    .partial_cmp(&b.visible_start)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for i in 0..sorted.len() - 1 {
                let a = sorted[i];
                let b = sorted[i + 1];
                // Overlap = how much a extends past b's start
                let overlap = a.visible_end - b.visible_start;
                if overlap > max_overlap {
                    self.diagnostics.push(
                        Diagnostic::warning(
                            DiagnosticCode::W101,
                            format!(
                                "Stacked elements '{}' and '{}' have {:.1}% visibility overlap",
                                a.target_key,
                                b.target_key,
                                overlap * 100.0
                            ),
                        )
                        .with_span(b.in_span.into())
                        .with_hint(format!(
                            "Reduce overlap to <{:.0}% or tighten fade-out timing",
                            max_overlap * 100.0
                        ))
                        .with_note(format!(
                            "'{}' visible until {:.2}, '{}' starts at {:.2}",
                            a.target_key, a.visible_end, b.target_key, b.visible_start
                        )),
                    );
                }
            }
        }
    }

    /// W113 + W114: Readability checks.
    fn check_readability(&mut self) {
        if !self.config.rules.readability.enabled {
            return;
        }
        let min_visible = self.config.rules.readability.min_visible_duration;
        let min_hold = self.config.rules.readability.min_hold_duration;

        let windows = self.build_visibility_windows();
        for window in &windows {
            let total_visible = window.visible_end - window.visible_start;
            if total_visible < min_visible {
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W113,
                        format!(
                            "Content '{}' is only visible for {:.1}% of scroll range (minimum: {:.0}%)",
                            window.target_key,
                            total_visible * 100.0,
                            min_visible * 100.0
                        ),
                    )
                    .with_span(window.in_span.into())
                    .with_hint(
                        "Increase the scroll range between fade-in and fade-out for readability"
                            .to_string(),
                    ),
                );
            }

            if window.hold_duration < min_hold {
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W114,
                        format!(
                            "Content '{}' has only {:.1}% hold time at full opacity (minimum: {:.0}%)",
                            window.target_key,
                            window.hold_duration * 100.0,
                            min_hold * 100.0
                        ),
                    )
                    .with_span(window.in_span.into())
                    .with_hint(
                        "Add more scroll distance between fade-in end and fade-out start"
                            .to_string(),
                    ),
                );
            }
        }
    }

    /// Build visibility windows from opacity animation spans.
    fn build_visibility_windows(&self) -> Vec<VisibilityWindow> {
        let opacity_spans: Vec<&ScrollAnimationSpan> = self
            .spans
            .iter()
            .filter(|s| s.property == "opacity")
            .collect();

        // Group by (parent_selector, target_selector)
        let mut by_target: std::collections::HashMap<String, Vec<&ScrollAnimationSpan>> =
            std::collections::HashMap::new();
        for span in &opacity_spans {
            let key = format!(
                "{}>>{}",
                span.parent_selector,
                span.target_selector.as_deref().unwrap_or("&")
            );
            by_target.entry(key).or_default().push(span);
        }

        let mut windows = Vec::new();

        for spans in by_target.values() {
            let mut ins: Vec<&ScrollAnimationSpan> = Vec::new();
            let mut outs: Vec<&ScrollAnimationSpan> = Vec::new();

            for span in spans {
                if is_fade_in(span) {
                    ins.push(span);
                } else if is_fade_out(span) {
                    outs.push(span);
                }
            }

            ins.sort_by(|a, b| {
                a.scroll_start
                    .partial_cmp(&b.scroll_start)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            outs.sort_by(|a, b| {
                a.scroll_start
                    .partial_cmp(&b.scroll_start)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for in_span in &ins {
                // Find the nearest out-animation after this in-animation
                let matching_out = outs
                    .iter()
                    .find(|out| out.scroll_start >= in_span.scroll_end);

                let (visible_end, fully_visible_end, has_out) = match matching_out {
                    Some(out) => (out.scroll_end, out.scroll_start, true),
                    None => (1.0, 1.0, false),
                };

                let fully_visible_start = in_span.scroll_end;
                let hold_duration = fully_visible_end - fully_visible_start;

                // Build a display-friendly target key
                let display_key = in_span
                    .target_selector
                    .as_deref()
                    .unwrap_or(&in_span.parent_selector);

                windows.push(VisibilityWindow {
                    target_key: display_key.to_string(),
                    parent_selector: in_span.parent_selector.clone(),
                    visible_start: in_span.scroll_start,
                    visible_end,
                    fully_visible_start,
                    fully_visible_end,
                    hold_duration: hold_duration.max(0.0),
                    has_out,
                    in_span: in_span.span,
                });
            }
        }

        windows
    }

    fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn extract_from_to(values: &[String]) -> (Option<String>, Option<String>) {
    match values.len() {
        0 => (None, None),
        1 => (Some(values[0].clone()), None),
        _ => (
            Some(values[0].clone()),
            Some(values[values.len() - 1].clone()),
        ),
    }
}

fn parse_arrow_value(value: &str) -> (Option<String>, Option<String>) {
    let parts: Vec<&str> = value.split("->").map(|s| s.trim()).collect();
    match parts.len() {
        0 => (None, None),
        1 => (Some(parts[0].to_string()), None),
        _ => (
            Some(parts[0].to_string()),
            Some(parts[parts.len() - 1].to_string()),
        ),
    }
}

fn is_fade_in(span: &ScrollAnimationSpan) -> bool {
    match (&span.from_value, &span.to_value) {
        (Some(from), Some(to)) => {
            let from_num: f64 = from.trim().parse().unwrap_or(1.0);
            let to_num: f64 = to.trim().parse().unwrap_or(0.0);
            to_num > from_num
        }
        _ => false,
    }
}

fn is_fade_out(span: &ScrollAnimationSpan) -> bool {
    match (&span.from_value, &span.to_value) {
        (Some(from), Some(to)) => {
            let from_num: f64 = from.trim().parse().unwrap_or(0.0);
            let to_num: f64 = to.trim().parse().unwrap_or(1.0);
            to_num < from_num
        }
        _ => false,
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Main entry point. Analyze scroll timelines and return diagnostics.
pub fn analyze_timelines(scopes: &[ScopeBlock], config: &TimelineLintConfig) -> Vec<Diagnostic> {
    if !config.enabled {
        return vec![];
    }
    let mut linter = TimelineLinter::new(config.clone());
    linter.collect_from_scopes(scopes);
    linter.run_all_rules();
    linter.into_diagnostics()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_scroll_fm(
        name: &str,
        start: f64,
        end: f64,
        body: Vec<(&str, &str, &str)>, // (selector, property, value)
    ) -> FormMatch {
        let mut props = Vec::new();
        for (selector, property, value) in &body {
            if selector.is_empty() {
                props.push((property.to_string(), value.to_string()));
            } else {
                props.push((format!("{} {}", selector, property), value.to_string()));
            }
        }

        let mut captures = HashMap::new();
        captures.insert("name".to_string(), CapturedValue::Ident(name.to_string()));
        captures.insert("start".to_string(), CapturedValue::Number(start));
        captures.insert("end".to_string(), CapturedValue::Number(end));
        captures.insert("body".to_string(), CapturedValue::StyleProperties(props));

        FormMatch {
            macro_name: "scroll".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::new(0, 1),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    fn make_scope(selector: &str, matches: Vec<FormMatch>) -> ScopeBlock {
        ScopeBlock {
            kind: Default::default(),
            selector: selector.to_string(),
            behavior: Default::default(),
            css_declarations: Vec::new(),
            form_refs: Vec::new(),
            nested_scopes: Vec::new(),
            matches,
            span: SourceSpan::new(0, 1),
            source_file: None,
            exports: vec![],
            refs: vec![],
            states: vec![],
            html: String::new(),
        }
    }

    // ---- W112: Invalid range ----

    #[test]
    fn w112_detects_invalid_range() {
        let fm = make_scroll_fm("bad", 0.5, 0.3, vec![("", "opacity", "0 -> 1")]);
        let scopes = vec![make_scope(".test", vec![fm])];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W112),
            "Should detect invalid range: {:?}",
            diags
        );
    }

    #[test]
    fn w112_passes_valid_range() {
        let fm = make_scroll_fm("ok", 0.1, 0.5, vec![]);
        let scopes = vec![make_scope(".test", vec![fm])];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W112),
            "Should not warn on valid range"
        );
    }

    // ---- W111: Same-target overlap ----

    #[test]
    fn w111_detects_same_target_overlap() {
        let fm1 = make_scroll_fm(
            "fade1",
            0.0,
            0.5,
            vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
        );
        let fm2 = make_scroll_fm(
            "fade2",
            0.4,
            0.8,
            vec![("[data-wp=\"1\"]", "opacity", "1 -> 0")],
        );
        let scopes = vec![make_scope(".test", vec![fm1, fm2])];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W111),
            "Should detect same-target overlap"
        );
    }

    #[test]
    fn w111_passes_non_overlapping() {
        let fm1 = make_scroll_fm(
            "fade1",
            0.0,
            0.3,
            vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
        );
        let fm2 = make_scroll_fm(
            "fade2",
            0.3,
            0.6,
            vec![("[data-wp=\"1\"]", "opacity", "1 -> 0")],
        );
        let scopes = vec![make_scope(".test", vec![fm1, fm2])];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W111),
            "Should not warn when ranges don't overlap"
        );
    }

    // ---- W110: Missing out ----

    #[test]
    fn w110_detects_missing_out() {
        // Two in-animations but only one out (and allow_last_element=true, so only first missing)
        let fm1 = make_scroll_fm(
            "w1-in",
            0.0,
            0.2,
            vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
        );
        let fm2 = make_scroll_fm(
            "w2-in",
            0.5,
            0.7,
            vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
        );
        // No out-animations at all
        let scopes = vec![make_scope(".test", vec![fm1, fm2])];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W110),
            "Should detect missing out-animation for first waypoint"
        );
    }

    #[test]
    fn w110_allows_last_element_without_out() {
        let fm1 = make_scroll_fm(
            "w1-in",
            0.0,
            0.2,
            vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
        );
        let fm_out = make_scroll_fm(
            "w1-out",
            0.3,
            0.5,
            vec![("[data-wp=\"1\"]", "opacity", "1 -> 0")],
        );
        let fm2 = make_scroll_fm(
            "w2-in",
            0.6,
            0.8,
            vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
        );
        // w2-in has no out, but it's the last → allowed
        let scopes = vec![make_scope(".test", vec![fm1, fm_out, fm2])];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W110),
            "Should allow last element without out-animation"
        );
    }

    // ---- W101: Stacked visibility overlap ----

    #[test]
    fn w101_detects_stacked_overlap() {
        // WP1: in 0.0-0.2, out 0.3-0.5 → visible 0.0-0.5
        // WP2: in 0.35-0.55, out 0.7-0.9 → visible 0.35-0.9
        // Overlap: 0.5 - 0.35 = 0.15 (15%) > 2% threshold
        let scopes = vec![make_scope(
            ".pillar",
            vec![
                make_scroll_fm(
                    "w1-in",
                    0.0,
                    0.2,
                    vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
                ),
                make_scroll_fm(
                    "w1-out",
                    0.3,
                    0.5,
                    vec![("[data-wp=\"1\"]", "opacity", "1 -> 0")],
                ),
                make_scroll_fm(
                    "w2-in",
                    0.35,
                    0.55,
                    vec![("[data-wp=\"2\"]", "opacity", "0 -> 1")],
                ),
                make_scroll_fm(
                    "w2-out",
                    0.7,
                    0.9,
                    vec![("[data-wp=\"2\"]", "opacity", "1 -> 0")],
                ),
            ],
        )];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W101),
            "Should detect stacked visibility overlap: {:?}",
            diags
        );
    }

    #[test]
    fn w101_passes_tight_timing() {
        // WP1: in 0.05-0.15, out 0.17-0.22 → visible 0.05-0.22
        // WP2: in 0.23-0.33, out 0.37-0.42 → visible 0.23-0.42
        // Overlap: 0.22 - 0.23 = -0.01 (no overlap)
        let scopes = vec![make_scope(
            ".pillar",
            vec![
                make_scroll_fm(
                    "w1-in",
                    0.05,
                    0.15,
                    vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
                ),
                make_scroll_fm(
                    "w1-out",
                    0.17,
                    0.22,
                    vec![("[data-wp=\"1\"]", "opacity", "1 -> 0")],
                ),
                make_scroll_fm(
                    "w2-in",
                    0.23,
                    0.33,
                    vec![("[data-wp=\"2\"]", "opacity", "0 -> 1")],
                ),
                make_scroll_fm(
                    "w2-out",
                    0.37,
                    0.42,
                    vec![("[data-wp=\"2\"]", "opacity", "1 -> 0")],
                ),
            ],
        )];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W101),
            "Should not warn with tight timing: {:?}",
            diags
        );
    }

    // ---- W113: Insufficient read time ----

    #[test]
    fn w113_detects_insufficient_read_time() {
        // in 0.0-0.02, out 0.03-0.05 → visible 0.0-0.05 = 5% < 8% minimum
        let scopes = vec![make_scope(
            ".pillar",
            vec![
                make_scroll_fm(
                    "w1-in",
                    0.0,
                    0.02,
                    vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
                ),
                make_scroll_fm(
                    "w1-out",
                    0.03,
                    0.05,
                    vec![("[data-wp=\"1\"]", "opacity", "1 -> 0")],
                ),
            ],
        )];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W113),
            "Should detect insufficient read time: {:?}",
            diags
        );
    }

    // ---- W114: No hold time ----

    #[test]
    fn w114_detects_no_hold_time() {
        // in 0.0-0.10, out 0.10-0.20 → hold = 0.10 - 0.10 = 0% < 3% min
        let scopes = vec![make_scope(
            ".pillar",
            vec![
                make_scroll_fm(
                    "w1-in",
                    0.0,
                    0.10,
                    vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
                ),
                make_scroll_fm(
                    "w1-out",
                    0.10,
                    0.20,
                    vec![("[data-wp=\"1\"]", "opacity", "1 -> 0")],
                ),
            ],
        )];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W114),
            "Should detect no hold time: {:?}",
            diags
        );
    }

    #[test]
    fn w114_passes_with_hold_time() {
        // in 0.0-0.10, out 0.15-0.25 → hold = 0.15 - 0.10 = 5% >= 3%
        let scopes = vec![make_scope(
            ".pillar",
            vec![
                make_scroll_fm(
                    "w1-in",
                    0.0,
                    0.10,
                    vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
                ),
                make_scroll_fm(
                    "w1-out",
                    0.15,
                    0.25,
                    vec![("[data-wp=\"1\"]", "opacity", "1 -> 0")],
                ),
            ],
        )];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W114),
            "Should not warn with sufficient hold time: {:?}",
            diags
        );
    }

    // ---- Config: disable rules ----

    #[test]
    fn disabled_lint_emits_nothing() {
        let fm = make_scroll_fm("bad", 0.5, 0.3, vec![]);
        let scopes = vec![make_scope(".test", vec![fm])];
        let config = TimelineLintConfig {
            enabled: false,
            ..Default::default()
        };
        let diags = analyze_timelines(&scopes, &config);
        assert!(
            diags.is_empty(),
            "Disabled linter should emit no diagnostics"
        );
    }

    #[test]
    fn disable_individual_rule() {
        let fm = make_scroll_fm("bad", 0.5, 0.3, vec![]);
        let scopes = vec![make_scope(".test", vec![fm])];
        let mut config = TimelineLintConfig::default();
        config.rules.invalid_range.enabled = false;
        let diags = analyze_timelines(&scopes, &config);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W112),
            "Disabled W112 rule should not emit"
        );
    }

    #[test]
    fn configurable_overlap_threshold() {
        // Two same-target opacity spans overlapping by 5%
        let fm1 = make_scroll_fm(
            "fade1",
            0.0,
            0.55,
            vec![("[data-wp=\"1\"]", "opacity", "0 -> 1")],
        );
        let fm2 = make_scroll_fm(
            "fade2",
            0.50,
            1.0,
            vec![("[data-wp=\"1\"]", "opacity", "1 -> 0")],
        );
        let scopes = vec![make_scope(".test", vec![fm1, fm2])];

        // Default max_overlap=0.0 → should warn
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::W111),
            "Default threshold should catch 5% overlap"
        );

        // Raise threshold to 10% → should not warn
        let mut config = TimelineLintConfig::default();
        config.rules.same_target_overlap.max_overlap = 0.10;
        let diags = analyze_timelines(&scopes, &config);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W111),
            "Raised threshold should not warn on 5% overlap"
        );
    }

    // ---- Non-scroll FormMatches are ignored ----

    #[test]
    fn ignores_non_scroll_macros() {
        let mut fm = make_scroll_fm("test", 0.5, 0.3, vec![]);
        fm.macro_name = "on".to_string(); // not a scroll
        let scopes = vec![make_scope(".test", vec![fm])];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        assert!(diags.is_empty(), "Should ignore non-scroll FormMatches");
    }

    // ---- Flat keyframes body ----

    #[test]
    fn handles_flat_keyframes_body() {
        use crate::syntax::KeyframeDef;

        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("reveal".to_string()),
        );
        captures.insert("start".to_string(), CapturedValue::Number(0.1));
        captures.insert("end".to_string(), CapturedValue::Number(0.5));
        captures.insert(
            "body".to_string(),
            CapturedValue::Keyframes(vec![KeyframeDef {
                property: "opacity".to_string(),
                values: vec!["0".to_string(), "1".to_string()],
                selector: None,
            }]),
        );

        let fm = FormMatch {
            macro_name: "scroll".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::new(0, 1),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let scopes = vec![make_scope(".test", vec![fm])];
        let diags = analyze_timelines(&scopes, &TimelineLintConfig::default());
        // Should process without errors (no W112 since range is valid)
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::W112),
            "Flat keyframes body should work"
        );
    }
}
