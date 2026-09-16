//! Complex extractors — residual structured extractions (PLAN-023 W2).
//!
//! Properties, Fields, Params, and Keyframes were hand-rolled depth-tracking loops here;
//! they have been MIGRATED to stdlib `%capture_type` productions (capture-types/{properties,
//! params,keyframes}.st) compiled via the PEG engine and reified back into their typed
//! `CapturedValue` shapes by reifiers in `custom.rs`. The equivalence was proven by
//! differential tests before deletion and is now pinned by golden tests
//! (`*_via_stdlib_matches_rust_extractor` in `custom.rs`).
//!
//! What remains are States and Transitions — thin Expr-fallback aliases (not hand-rolled
//! loops; their stdlib grammar is future work).

use super::{CaptureExtractor, ExtractResult, TokenData};

/// Extract states — Expr-fallback alias. States are defined in stdlib via `%capture_type`
/// patterns; this provides a fallback when no custom extractor is registered.
pub struct StatesExtractor;

impl CaptureExtractor for StatesExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        super::simple::ExprExtractor.extract(tokens, source)
    }
}

/// Extract transitions — Expr-fallback alias (see StatesExtractor).
pub struct TransitionsExtractor;

impl CaptureExtractor for TransitionsExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        super::simple::ExprExtractor.extract(tokens, source)
    }
}
