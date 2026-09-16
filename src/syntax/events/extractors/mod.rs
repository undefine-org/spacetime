//! CaptureExtractor trait and ExtractorRegistry — Layer 1 of the MatchSink architecture.
//!
//! Each CaptureType (Ident, String, Number, Time, Length, etc.) has a corresponding
//! extractor that operates on `&[TokenData]` slices. Extractors are individually
//! unit-testable, composable, and extensible.

pub mod blocks;
pub mod complex;
pub mod custom;
pub mod pattern;
pub mod reference;
pub mod simple;

use std::collections::HashMap;

use crate::parser::meta_ast::CaptureType;
use crate::syntax::cst::SyntaxKind;
use crate::syntax::form_match::CapturedValue;

/// A collected token from sink events, stored in node contexts.
/// Represents a token by its kind and byte range in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenData {
    pub kind: SyntaxKind,
    /// Byte range in source: (start, end) exclusive.
    pub text_range: (usize, usize),
}

impl TokenData {
    /// Get the text of this token from the source string.
    pub fn text<'s>(&self, source: &'s str) -> &'s str {
        &source[self.text_range.0..self.text_range.1]
    }
}

/// Result of a capture extraction: the captured value and how many tokens were consumed.
pub type ExtractResult = Option<(CapturedValue, usize)>;

/// Each CaptureType implements extraction from a token slice.
///
/// Simple extractors (Ident, Number, etc.) match 1-2 tokens.
/// Complex extractors (Expr, Properties, Keyframes) use chumsky combinators.
pub trait CaptureExtractor: Send + Sync {
    /// Try to extract a value from the given token slice.
    /// Returns `Some((value, tokens_consumed))` on success, `None` on failure.
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult;

    /// Whether this extractor must match with NO whitespace before it.
    ///
    /// Almost nothing does — a grammar sequence normally tolerates whitespace
    /// between its elements, and `param_list`'s `$a, &b` relies on that. A
    /// dimension's UNIT is the exception: `40px` is one value and `40 px` is not
    /// a value at all, so its terminal opts in and a preceding space makes the
    /// whole sequence fail rather than silently accepting malformed CSS.
    fn requires_adjacency(&self) -> bool {
        false
    }
}

/// Registry mapping CaptureType -> Box<dyn CaptureExtractor>.
pub struct ExtractorRegistry {
    /// Built-in extractors keyed by CaptureType.
    extractors: HashMap<CaptureType, Box<dyn CaptureExtractor>>,
    /// Custom extractors from %capture_type definitions, keyed by name.
    pub custom: HashMap<String, Box<dyn CaptureExtractor>>,
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtractorRegistry {
    /// Create a new registry with all built-in extractors registered.
    pub fn new() -> Self {
        let mut extractors: HashMap<CaptureType, Box<dyn CaptureExtractor>> = HashMap::new();

        // Simple extractors
        extractors.insert(CaptureType::Ident, Box::new(simple::IdentExtractor));
        extractors.insert(
            CaptureType::DashedIdent,
            Box::new(simple::DashedIdentExtractor),
        );
        extractors.insert(CaptureType::EventName, Box::new(simple::EventNameExtractor));
        extractors.insert(CaptureType::String, Box::new(simple::StringExtractor));
        extractors.insert(CaptureType::Number, Box::new(simple::NumberExtractor));
        extractors.insert(CaptureType::Bool, Box::new(simple::BoolExtractor));
        extractors.insert(CaptureType::Time, Box::new(simple::TimeExtractor));
        extractors.insert(CaptureType::Duration, Box::new(simple::TimeExtractor)); // alias
        extractors.insert(CaptureType::Length, Box::new(simple::LengthExtractor));
        extractors.insert(CaptureType::Event, Box::new(simple::EventExtractor));
        extractors.insert(CaptureType::Easing, Box::new(simple::EasingExtractor));
        extractors.insert(
            CaptureType::Selector,
            Box::new(reference::SelectorExtractor),
        );
        extractors.insert(CaptureType::Typeref, Box::new(simple::TyperefExtractor));

        // Reference extractors
        extractors.insert(CaptureType::Binding, Box::new(reference::BindingExtractor));
        extractors.insert(CaptureType::Element, Box::new(reference::ElementExtractor));
        extractors.insert(CaptureType::Preset, Box::new(reference::PresetExtractor));

        // Expr: greedy token consumer
        extractors.insert(CaptureType::Expr, Box::new(simple::ExprExtractor));
        extractors.insert(CaptureType::Color, Box::new(simple::ColorExtractor));

        // Complex extractors. Properties/Fields/Params/Keyframes MIGRATED to stdlib
        // %capture_type (PLAN-023 W2) — resolved via Custom(name) + reifier. States/Transitions
        // remain Expr-fallback aliases (not hand-rolled loops; out of W2 scope).
        extractors.insert(CaptureType::States, Box::new(complex::StatesExtractor));
        extractors.insert(
            CaptureType::Transitions,
            Box::new(complex::TransitionsExtractor),
        );

        // Block extractors
        extractors.insert(CaptureType::JsBlock, Box::new(blocks::JsBlockExtractor));
        extractors.insert(CaptureType::HtmlBlock, Box::new(blocks::HtmlBlockExtractor));
        extractors.insert(
            CaptureType::MutationActions,
            Box::new(blocks::MutationActionsExtractor),
        );
        extractors.insert(CaptureType::Template, Box::new(blocks::TemplateExtractor));
        extractors.insert(
            CaptureType::ComponentBody,
            Box::new(blocks::ComponentBodyExtractor),
        );

        // Pattern extractors (param_list migrated to stdlib %capture_type; PLAN-023 W2)
        extractors.insert(
            CaptureType::TemplateInvocation,
            Box::new(pattern::TemplateInvocationExtractor),
        );

        Self {
            extractors,
            custom: HashMap::new(),
        }
    }

    /// Get the extractor for a given CaptureType.
    pub fn get(&self, capture_type: &CaptureType) -> Option<&dyn CaptureExtractor> {
        self.extractors.get(capture_type).map(|b| b.as_ref())
    }

    /// Register a custom extractor (from %capture_type).
    pub fn register_custom(&mut self, name: String, extractor: Box<dyn CaptureExtractor>) {
        self.custom.insert(name, extractor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_simple_extractors() {
        let reg = ExtractorRegistry::new();
        assert!(reg.get(&CaptureType::Ident).is_some());
        assert!(reg.get(&CaptureType::String).is_some());
        assert!(reg.get(&CaptureType::Number).is_some());
        assert!(reg.get(&CaptureType::Bool).is_some());
        assert!(reg.get(&CaptureType::Time).is_some());
        assert!(reg.get(&CaptureType::Duration).is_some());
        assert!(reg.get(&CaptureType::Length).is_some());
        assert!(reg.get(&CaptureType::Event).is_some());
        assert!(reg.get(&CaptureType::Easing).is_some());
        assert!(reg.get(&CaptureType::Expr).is_some());
        assert!(reg.get(&CaptureType::Typeref).is_some());
    }

    #[test]
    fn registry_has_reference_extractors() {
        let reg = ExtractorRegistry::new();
        assert!(reg.get(&CaptureType::Binding).is_some());
        assert!(reg.get(&CaptureType::Element).is_some());
        assert!(reg.get(&CaptureType::Preset).is_some());
        assert!(reg.get(&CaptureType::Selector).is_some());
    }

    #[test]
    fn registry_has_complex_extractors() {
        let reg = ExtractorRegistry::new();
        // Properties/Fields/Params/Keyframes migrated to stdlib %capture_type (PLAN-023 W2).
        assert!(reg.get(&CaptureType::States).is_some());
        assert!(reg.get(&CaptureType::Transitions).is_some());
    }

    #[test]
    fn registry_has_block_extractors() {
        let reg = ExtractorRegistry::new();
        assert!(reg.get(&CaptureType::JsBlock).is_some());
        assert!(reg.get(&CaptureType::HtmlBlock).is_some());
        assert!(reg.get(&CaptureType::MutationActions).is_some());
        assert!(reg.get(&CaptureType::Template).is_some());
        assert!(reg.get(&CaptureType::ComponentBody).is_some());
    }

    #[test]
    fn registry_has_pattern_extractors() {
        let reg = ExtractorRegistry::new();
        // param_list is now a stdlib %capture_type (PLAN-023 W2), not a builtin extractor.
        assert!(reg.get(&CaptureType::TemplateInvocation).is_some());
    }

    #[test]
    fn registry_covers_all_standard_types() {
        let reg = ExtractorRegistry::new();
        // All 29 standard CaptureType variants should have extractors
        let types = [
            CaptureType::Ident,
            CaptureType::String,
            CaptureType::Number,
            CaptureType::Bool,
            CaptureType::Time,
            CaptureType::Length,
            CaptureType::Duration,
            CaptureType::Easing,
            CaptureType::Typeref,
            CaptureType::Binding,
            CaptureType::Event,
            CaptureType::Expr,
            // Properties/Fields/Params/Keyframes migrated to stdlib %capture_type (PLAN-023
            // W2) — no longer builtin extractors.
            CaptureType::States,
            CaptureType::Transitions,
            CaptureType::Selector,
            CaptureType::Element,
            CaptureType::Preset,
            CaptureType::MutationActions,
            CaptureType::Template,
            CaptureType::HtmlBlock,
            CaptureType::JsBlock,
            CaptureType::TemplateInvocation,
            CaptureType::Color,
            CaptureType::ComponentBody,
        ];
        for ct in &types {
            assert!(reg.get(ct).is_some(), "Missing extractor for {:?}", ct);
        }
    }
}
