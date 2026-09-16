//! EDN ⟷ Spacetime — a second concrete syntax projected from the `%form` registry.
//!
//! See `docs/edn-spacetime/SPEC.md` (normative) and PLAN-148 (rationale +
//! measurements).
//!
//! # The thesis
//!
//! Spacetime's grammar is already DATA: every construct is a `%form` pattern
//! declared in stdlib `.st` files (415 registered forms). So this module is not
//! a hand-written per-construct translator — it is a projection of the registry,
//! written once and covering every form, including ones added later.
//!
//! ```text
//!       %form registry  (the single grammar authority)
//!               │
//!       ┌───────┴───────┐
//!    .st text        EDN text        ← two concrete syntaxes, peer status
//!       └───────┬───────┘
//!          Vec<FormMatch>            ← the waist; both directions meet here
//!               │
//!     pipeline (resolve→sort→expand→emit)   ← UNTOUCHED
//! ```
//!
//! # Zero-change guarantee
//!
//! This module is purely ADDITIVE. The `.st` lexer/parser/CST, the `%form`
//! registry, and the pipeline are untouched; a regular `.st` user's file never
//! traverses a line of code in here. EDN *reads* the registry and can never
//! define syntax, so EDN cannot express anything `.st` cannot.

pub mod codec;
pub mod doc;
pub mod ingress;
pub mod print;
pub mod read;

pub use codec::{CodecError, from_edn_value, to_edn_value};
pub use print::{PrintError, print_file, print_form, print_form_in, print_forms};
pub use doc::{EdnDocument, EdnScope, construct_scope, scope_body_text, to_document, write_document};
pub use ingress::{SourceLang, detect, normalize_source, to_st_file};
pub use read::{ReadError, read_forms};
