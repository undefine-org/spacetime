//! A value checked against the file it names (PLAN-156, `%facts`).
//!
//! `@gltf &robot (src: "models/robot.glb", clip: "Walkng")` is refused with
//! the clips `robot.glb` holds, because `clip`'s type row says its values
//! are the file's facts: `%facts src gltf-clips`. The file is the one truth;
//! nothing lists the clips by hand.

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::syntax::{CapturedValue, FormMatch};

/// E0971 for every value (nested matches included) that isn't among the
/// facts of the file its sibling param names.
pub fn fact_diagnostics(matches: &[FormMatch]) -> Vec<Diagnostic> {
    let registry = &*crate::syntax::STDLIB_REGISTRY;
    let facts = registry.scalar_facts();
    let mut out = Vec::new();
    if facts.is_empty() {
        return out;
    }
    let map = crate::syntax::property_types::property_type_map();
    let mut stack: Vec<&FormMatch> = matches.iter().collect();
    while let Some(m) = stack.pop() {
        for v in m.captures.values() {
            push_blocks(v, &mut stack);
        }
        let directive = m.matched_macro.as_deref().unwrap_or(&m.macro_name).trim_start_matches('@').to_string();
        for (name, value) in &m.captures {
            let Some(ty) = map
                .get(&(directive.clone(), name.clone()))
                .or_else(|| map.get(&(m.macro_name.trim_start_matches('@').to_string(), name.clone())))
            else {
                continue;
            };
            let Some((sibling, reader)) = facts.get(ty) else { continue };
            // only the author's words are judged, and only a word
            let Some(span) = m.capture_spans.get(name) else { continue };
            let Some(word) = text(value) else { continue };
            if word.is_empty() {
                continue;
            }
            let Some(file) = m.captures.get(sibling).and_then(text).and_then(|src| {
                crate::facts::resolve(m.source_file.as_deref(), &src)
            }) else {
                continue;
            };
            let Ok(known) = crate::facts::read(reader, &file) else { continue };
            if !known.iter().any(|k| k == &word) {
                let near: Vec<&String> = known
                    .iter()
                    .filter(|k| k.to_lowercase().starts_with(&word.to_lowercase().chars().take(3).collect::<String>()))
                    .collect();
                let mut d = Diagnostic::error(
                    DiagnosticCode::E0971,
                    format!(
                        "`{word}` is not in {}: it has {}",
                        file.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
                        known.join(", ")
                    ),
                )
                .with_span((*span).into());
                if !near.is_empty() {
                    d = d.with_hint(format!(
                        "did you mean {}?",
                        near.iter().map(|k| format!("`{k}`")).collect::<Vec<_>>().join(" or ")
                    ));
                }
                out.push(d);
            }
        }
    }
    out
}

fn push_blocks<'a>(v: &'a CapturedValue, stack: &mut Vec<&'a FormMatch>) {
    match v {
        CapturedValue::Block(ms) => stack.extend(ms.iter()),
        CapturedValue::Array(items) => items.iter().for_each(|i| push_blocks(i, stack)),
        CapturedValue::Named(f) => f.values().for_each(|i| push_blocks(i, stack)),
        _ => {}
    }
}

fn text(v: &CapturedValue) -> Option<String> {
    match v {
        CapturedValue::String(s) | CapturedValue::Ident(s) | CapturedValue::Expr(s) => {
            Some(s.trim().trim_matches('"').to_string())
        }
        _ => None,
    }
}
