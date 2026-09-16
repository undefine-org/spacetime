//! Macro expansion utilities.
//!
//! Legacy expand_macro/ExpandedDirectives have been removed.
//! The pipeline (Evaluate → Resolve → Sort → Expand → Emit) handles all macro expansion.
//!
//! Remaining: `convert_properties_to_keyframes_js` (used by form_match.rs for `%animates`).

use crate::pipeline::types::{CompileError, CompileErrorKind};

/// Type alias preserved for downstream re-exports.
pub type MacroExpansionError = CompileError;
/// Type alias preserved for downstream re-exports.
pub type MacroExpansionErrorKind = CompileErrorKind;

/// Convert `%animates` property declarations into a JS object describing
/// keyframe scopes, properties, and static values.
///
/// Called from `form_match.rs` when processing `%animates` clauses.
///
/// # Input format
///
/// Each `(key, value)` pair is either:
/// - `("property", "from -> to")` — animated keyframe
/// - `("property", "static_value")` — static property
/// - `(".selector property", "from -> to")` — scoped to a child selector
///
/// # Output
///
/// Returns a JS object literal string, e.g.:
/// ```js
/// { properties: [{ property: 'opacity', keyframes: [{ at: 0, value: '0' }, { at: 1, value: '1' }] }] }
/// ```
pub(crate) fn convert_properties_to_keyframes_js(props: &[(String, String)]) -> String {
    use std::collections::HashMap;

    // Group properties by selector
    let mut scopes: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut root_props: Vec<(String, String)> = Vec::new();

    for (key, value) in props {
        // Key format: ".selector child property" or just "property"
        // Split from the RIGHT: property is always the last word, everything before is selector
        if let Some(last_space) = key.rfind(' ') {
            let selector = key[..last_space].to_string();
            let property = key[last_space + 1..].to_string();
            scopes
                .entry(selector)
                .or_default()
                .push((property, value.clone()));
        } else {
            root_props.push((key.clone(), value.clone()));
        }
    }

    let mut js_scopes: Vec<String> = Vec::new();

    for (selector, scope_props) in &scopes {
        let mut properties: Vec<String> = Vec::new();
        let mut static_properties: Vec<String> = Vec::new();
        let mut scope_easing: Option<String> = None;
        let mut scope_range: Option<(f64, f64)> = None;

        for (prop, val) in scope_props {
            if prop == "easing" {
                scope_easing = Some(val.trim_start_matches('&').to_string());
            } else if prop == "range" {
                let parts: Vec<&str> = val.split(" to ").collect();
                if parts.len() == 2
                    && let (Ok(start), Ok(end)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
                {
                    scope_range = Some((start, end));
                }
            } else if val.contains("->") {
                let keyframes = parse_keyframes_value(val);
                properties.push(format!(
                    "{{ property: '{}', keyframes: {} }}",
                    prop, keyframes
                ));
            } else {
                static_properties.push(format!(
                    "{{ property: '{}', value: '{}' }}",
                    prop,
                    val.replace('\'', "\\'")
                ));
            }
        }

        let mut scope_obj = format!("{{ selector: '{}'", selector);

        if !properties.is_empty() {
            scope_obj.push_str(&format!(", properties: [{}]", properties.join(", ")));
        }
        if !static_properties.is_empty() {
            scope_obj.push_str(&format!(
                ", staticProperties: [{}]",
                static_properties.join(", ")
            ));
        }
        if let Some(easing) = scope_easing {
            scope_obj.push_str(&format!(", easing: '{}'", easing));
        }
        if let Some((start, end)) = scope_range {
            scope_obj.push_str(&format!(", range: {{ start: {}, end: {} }}", start, end));
        }
        scope_obj.push_str(" }");

        js_scopes.push(scope_obj);
    }

    // Handle root-level properties (no selector prefix)
    let mut root_properties: Vec<String> = Vec::new();
    let mut root_static: Vec<String> = Vec::new();

    for (prop, val) in &root_props {
        if val.contains("->") {
            let keyframes = parse_keyframes_value(val);
            root_properties.push(format!(
                "{{ property: '{}', keyframes: {} }}",
                prop, keyframes
            ));
        } else {
            root_static.push(format!(
                "{{ property: '{}', value: '{}' }}",
                prop,
                val.replace('\'', "\\'")
            ));
        }
    }

    let mut result = String::from("{ ");
    let mut parts: Vec<String> = Vec::new();

    if !root_properties.is_empty() {
        parts.push(format!("properties: [{}]", root_properties.join(", ")));
    }
    if !root_static.is_empty() {
        parts.push(format!("staticProperties: [{}]", root_static.join(", ")));
    }
    if !js_scopes.is_empty() {
        parts.push(format!("scopes: [{}]", js_scopes.join(", ")));
    }

    result.push_str(&parts.join(", "));
    result.push_str(" }");
    result
}

/// Parse keyframe value like "0 -> 0.35" or "0 -> 0.5 -> 1" into JS array
pub(crate) fn parse_keyframes_value(val: &str) -> String {
    let stops = parse_stops(val);
    let count = stops.len();
    if count == 0 {
        return "[]".to_string();
    }

    // Resolve `at` positions. Endpoints default to 0 and 1; an unpositioned
    // interior stop is distributed EVENLY between its nearest positioned
    // neighbours (so `0 -> a -> 1 at 25% -> 1` puts `a` at 12.5%). This is
    // computed here, at compile time, so the runtime keeps its one job:
    // interpolate between `{at, value}` pairs it is handed.
    let mut ats: Vec<Option<f64>> = stops.iter().map(|s| s.at).collect();
    if count == 1 {
        ats[0] = Some(1.0);
    } else {
        if ats[0].is_none() {
            ats[0] = Some(0.0);
        }
        if ats[count - 1].is_none() {
            ats[count - 1] = Some(1.0);
        }
        // fill each run of Nones between two anchors by even division
        let mut i = 0;
        while i < count {
            if ats[i].is_some() {
                i += 1;
                continue;
            }
            let lo = i - 1; // always Some (index 0 anchored above)
            let mut j = i;
            while j < count && ats[j].is_none() {
                j += 1;
            }
            // j is the next anchored index (<= count-1, anchored above)
            let lo_at = ats[lo].unwrap();
            let hi_at = ats[j].unwrap();
            let gaps = (j - lo) as f64;
            for (k, slot) in (i..j).enumerate() {
                ats[slot] = Some(lo_at + (hi_at - lo_at) * ((k + 1) as f64) / gaps);
            }
            i = j;
        }
    }

    // Per-step easing: the author writes the curve on the stop the step
    // REACHES (`0 -> 1 --ease-out-expo`), but the runtime reads a segment's
    // easing from its EARLIER keyframe (`getEase(prev.easing || propEasing)`).
    // So a stop's declared easing is emitted on the PREVIOUS keyframe. The
    // final stop's easing (if any) governs nothing and is dropped.
    let keyframes: Vec<String> = (0..count)
        .map(|i| {
            let at = ats[i].unwrap_or(0.0);
            let value = stops[i].value.replace('\'', "\\'");
            // easing for segment i->i+1 comes from stop i+1's declared easing
            let seg_easing = stops.get(i + 1).and_then(|s| s.easing.as_ref());
            match seg_easing {
                Some(e) => format!(
                    "{{ at: {}, value: '{}', easing: '{}' }}",
                    at,
                    value,
                    e.trim_start_matches("--").replace('\'', "\\'")
                ),
                None => format!("{{ at: {}, value: '{}' }}", at, value),
            }
        })
        .collect();

    format!("[{}]", keyframes.join(", "))
}

/// The initial (progress=0) VALUE of a keyframe chain, with any film-surface
/// stop syntax stripped: `0.5 --ease-out-expo -> 1` -> `0.5`, `40px at 0% -> 0`
/// -> `40px`. Used by FOUC initial-state CSS so a per-step easing or an `at`
/// on the first stop never leaks into `transform: scale(0.5 --ease-out-expo)`.
/// Returns None for a static value (no `->`) or a var() first stop.
pub(crate) fn first_stop_value(val: &str) -> Option<String> {
    if !val.contains("->") {
        return None;
    }
    let stops = parse_stops(val);
    let first = stops.first()?;
    if first.value.starts_with("var(") {
        return None;
    }
    Some(first.value.clone())
}

/// One parsed keyframe stop: its value, an optional author position (`at 25%`
/// -> 0.25, `at 0.4` -> 0.4), and an optional per-step easing form ref
/// (`--ease-out-expo`). PLAN-150 W1, the film-surface keyframe grammar of
/// docs/language/film.st.md §2.
struct Stop {
    value: String,
    at: Option<f64>,
    easing: Option<String>,
}

/// Split a `a -> b -> c` value chain into stops, then peel `at <pos>` and a
/// trailing `--easing` off each. All scans are PAREN-DEPTH aware so nothing
/// inside `calc(…)`, `circle(50% at 50%)` or a `radial-gradient(… at …)` is
/// mistaken for the stop-level `at`/`--` tokens.
fn parse_stops(val: &str) -> Vec<Stop> {
    split_top_level(val, "->")
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .map(|raw| {
            let (body, easing) = peel_trailing_easing(raw.trim());
            let (value, at) = peel_at(body.trim());
            Stop {
                value: value.trim().to_string(),
                at,
                easing,
            }
        })
        .collect()
}

/// Split `s` on a top-level (paren-depth 0, outside strings) occurrence of the
/// two-byte separator `sep` (`->`).
fn split_top_level(s: &str, sep: &str) -> Vec<String> {
    let b = s.as_bytes();
    let sepb = sep.as_bytes();
    let mut out = Vec::new();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'"' && (i == 0 || b[i - 1] != b'\\') {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ if depth == 0 && b[i..].starts_with(sepb) => {
                out.push(s[start..i].to_string());
                i += sepb.len();
                start = i;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(s[start..].to_string());
    out
}

/// Peel a trailing top-level `--<ident>` easing ref off a stop, returning
/// (value-without-easing, Some(easing)). A stop that is ENTIRELY a `--ref`
/// (no value before it) is left intact — that is a form splice / whole-body
/// easing, not a per-step easing, and its own path owns it.
fn peel_trailing_easing(stop: &str) -> (&str, Option<String>) {
    // find the last top-level whitespace-delimited token; if it is `--…` and
    // something non-empty precedes it, it is the step easing.
    let b = stop.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut last_ws: Option<usize> = None;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'"' && (i == 0 || b[i - 1] != b'\\') {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ if depth == 0 && c.is_ascii_whitespace() => last_ws = Some(i),
            _ => {}
        }
        i += 1;
    }
    if let Some(ws) = last_ws {
        let tail = stop[ws..].trim();
        let head = stop[..ws].trim();
        if tail.starts_with("--") && !head.is_empty() {
            return (&stop[..ws], Some(tail.to_string()));
        }
    }
    (stop, None)
}

/// Peel a top-level `at <pos>` off a stop, returning (value, Some(fraction)).
/// `at 25%` -> 0.25, `at 0.4` -> 0.4. A malformed position is left in the
/// value (it will surface downstream) rather than silently dropped.
fn peel_at(stop: &str) -> (&str, Option<f64>) {
    let b = stop.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'"' && (i == 0 || b[i - 1] != b'\\') {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ if depth == 0
                && (c == b'a' || c == b'A')
                && b.get(i + 1).map(|x| x | 0x20) == Some(b't')
                && (i == 0 || b[i - 1].is_ascii_whitespace())
                && b.get(i + 2).is_some_and(|x| x.is_ascii_whitespace()) =>
            {
                let head = &stop[..i];
                let pos_txt = stop[i + 2..].trim();
                if let Some(frac) = parse_position(pos_txt) {
                    return (head, Some(frac));
                }
                return (stop, None);
            }
            _ => {}
        }
        i += 1;
    }
    (stop, None)
}

/// `25%` -> 0.25, `0.4` -> 0.4, `40` -> 0.4 (a bare number ≥ 1 is read as a
/// percent-of-100 only when it carries `%`; a bare `> 1` is out of range and
/// rejected). Returns None if unparseable.
fn parse_position(txt: &str) -> Option<f64> {
    let t = txt.trim();
    if let Some(p) = t.strip_suffix('%') {
        return p.trim().parse::<f64>().ok().map(|v| v / 100.0);
    }
    let v = t.parse::<f64>().ok()?;
    (0.0..=1.0).contains(&v).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_properties_to_keyframes_js() {
        let props = vec![
            ("opacity".to_string(), "0 -> 1".to_string()),
            (
                "transform".to_string(),
                "translateY(20px) -> translateY(0)".to_string(),
            ),
        ];
        let result = convert_properties_to_keyframes_js(&props);
        assert!(result.contains("property: 'opacity'"));
        assert!(result.contains("keyframes:"));
        assert!(result.contains("at: 0"));
        assert!(result.contains("at: 1"));
    }

    #[test]
    fn test_scoped_keyframes() {
        let props = vec![(".child opacity".to_string(), "0 -> 1".to_string())];
        let result = convert_properties_to_keyframes_js(&props);
        assert!(result.contains("selector: '.child'"));
        assert!(result.contains("property: 'opacity'"));
    }

    #[test]
    fn test_parse_keyframes_value() {
        assert_eq!(
            parse_keyframes_value("0 -> 1"),
            "[{ at: 0, value: '0' }, { at: 1, value: '1' }]"
        );
    }

    // ===== PLAN-150 W1: positioned stops + per-step easing =====

    #[test]
    fn test_positioned_stop_percent() {
        // `at 25%` fixes the middle stop; endpoints anchor 0 and 1.
        assert_eq!(
            parse_keyframes_value("0 -> 0.85 at 25% -> 0"),
            "[{ at: 0, value: '0' }, { at: 0.25, value: '0.85' }, { at: 1, value: '0' }]"
        );
    }

    #[test]
    fn test_positioned_stop_fraction() {
        assert_eq!(
            parse_keyframes_value("0 -> 1 at 0.3 -> 0"),
            "[{ at: 0, value: '0' }, { at: 0.3, value: '1' }, { at: 1, value: '0' }]"
        );
    }

    #[test]
    fn test_unpositioned_stops_distribute_between_anchors() {
        // one anchored stop at 60%; the two unpositioned interior stops split
        // the [0, 0.6] and [0.6, 1] gaps evenly.
        assert_eq!(
            parse_keyframes_value("0 -> a -> b at 60% -> c -> 1"),
            "[{ at: 0, value: '0' }, { at: 0.3, value: 'a' }, { at: 0.6, value: 'b' }, \
{ at: 0.8, value: 'c' }, { at: 1, value: '1' }]"
        );
    }

    #[test]
    fn test_per_step_easing_shifts_back_one_stop() {
        // author writes easing on the stop the step REACHES; the runtime reads
        // it from the earlier keyframe, so it lands on stop i-1.
        assert_eq!(
            parse_keyframes_value("40px -> 0 --ease-out-expo -> -6px --linear -> 0"),
            "[{ at: 0, value: '40px', easing: 'ease-out-expo' }, \
{ at: 0.3333333333333333, value: '0', easing: 'linear' }, \
{ at: 0.6666666666666666, value: '-6px' }, { at: 1, value: '0' }]"
        );
    }

    #[test]
    fn test_at_inside_calc_is_not_a_positioned_stop() {
        // the `at` inside `circle(50% at 50% 50%)` must NOT be peeled; the stop
        // stays whole and keeps its even offset.
        let out = parse_keyframes_value("circle(0% at 50% 50%) -> circle(75% at 50% 50%)");
        assert!(
            out.contains("value: 'circle(0% at 50% 50%)'")
                && out.contains("value: 'circle(75% at 50% 50%)'"),
            "paren-level `at` must survive: {out}"
        );
        assert!(out.contains("at: 0,") && out.contains("at: 1,"), "even offsets: {out}");
    }

    #[test]
    fn test_first_stop_value_strips_stop_syntax() {
        assert_eq!(first_stop_value("0.5 --ease-out-expo -> 1").as_deref(), Some("0.5"));
        assert_eq!(first_stop_value("40px at 0% -> 0").as_deref(), Some("40px"));
        assert_eq!(first_stop_value("static"), None);
        assert_eq!(first_stop_value("var(--x) -> 1"), None);
    }
}
