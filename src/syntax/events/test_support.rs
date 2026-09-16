//! Shared test helpers for the events layer (test-only).
//!
//! The `@on <driver>` head was cut over from a bare event name (`@on click`)
//! to a sigil ELEMENT_REF driver (`@on &.click`). Hand-built unit tests that
//! previously matched `$event:ident` now need a capture that consumes the
//! element-ref token run. This mirrors the stdlib `driver_expr` %capture_type
//! (`( "&" $subject:ident? ) "." $member:ident`) closely enough for those tests,
//! and produces the same `Named{subject, member, driver_params}` shape the
//! pipeline's driver handling reads (src/pipeline/drivers.rs).

use std::collections::HashMap;

use crate::syntax::events::extractors::{CaptureExtractor, ExtractResult, ExtractorRegistry, TokenData};
use crate::syntax::form_match::CapturedValue;
use crate::syntax::cst::SyntaxKind;

/// Consumes `&[subject].member` (subject optional) and yields the production
/// `driver` capture shape: `Named{ subject, member, driver_params }`.
pub struct DriverExprExtractor;

impl CaptureExtractor for DriverExprExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let mut i = 0;
        while tokens.get(i).is_some_and(|t| t.kind.is_trivia()) {
            i += 1;
        }
        if !tokens.get(i).is_some_and(|t| t.kind == SyntaxKind::AMPERSAND) {
            return None;
        }
        i += 1;
        let mut subject = String::new();
        if tokens.get(i).is_some_and(|t| t.kind == SyntaxKind::IDENT) {
            subject = tokens[i].text(source).to_string();
            i += 1;
        }
        if !tokens.get(i).is_some_and(|t| t.kind == SyntaxKind::DOT) {
            return None;
        }
        i += 1;
        let member = tokens.get(i).filter(|t| t.kind == SyntaxKind::IDENT)?;
        let member_text = member.text(source).to_string();
        i += 1;

        let mut map = HashMap::new();
        map.insert("subject".to_string(), CapturedValue::Ident(subject));
        map.insert("member".to_string(), CapturedValue::Ident(member_text));
        map.insert("driver_params".to_string(), CapturedValue::Array(Vec::new()));
        Some((CapturedValue::Named(map), i))
    }
}

/// Register the `driver_expr` custom capture so a hand-built form can name
/// `CaptureType::Custom("driver_expr")`.
pub fn register_driver_expr(extractors: &mut ExtractorRegistry) {
    extractors.register_custom("driver_expr".to_string(), Box::new(DriverExprExtractor));
}

/// Pull the `member` of a `driver` capture (the production `&.click` → "click").
pub fn driver_member(fm: &crate::syntax::form_match::FormMatch, var_name: &str) -> Option<String> {
    match fm.captures.get(var_name) {
        Some(CapturedValue::Named(m)) => match m.get("member") {
            Some(CapturedValue::Ident(s)) => Some(s.clone()),
            _ => None,
        },
        _ => None,
    }
}
