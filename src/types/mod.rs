//! Value-level type machinery (FEAT-109).
//!
//! Today this is inference: what type IS a literal, with no annotation. The
//! module exists as its own tree rather than living under `syntax::events`
//! because inference is not a parsing concern — it consumes parse results and
//! answers a question about MEANING, and the four shadow classifiers it is meant
//! to replace (sync/protocol.rs, lsp/colors.rs, analysis/visual_lint.rs, and the
//! detector half of color/mod.rs) all live outside the parser too.

pub mod value_infer;
