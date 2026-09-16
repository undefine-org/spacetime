//! Editable rich-text schema (PLAN-031 / FEAT-097).
//!
//! An `@editable-mark` / `@editable-block` IS a `@template` whose first parameter
//! is the selection (`&sel`) and whose body is the HTML *projection* of that mark.
//! The editor needs a machine-readable description of every registered mark/block:
//! which tag it projects to, which typed attributes it carries (the free params),
//! and where the selection lands. That description is the **editable schema**.
//!
//! This module derives the schema from the parsed mark/block declaration — the
//! same `param_list` + `component_body` captures the template macro already
//! produces — and applies the **invertibility check** that decides whether a mark
//! can participate in paste-lift (the backward direction of the projection lens).
//!
//! The schema is a *projection of registered constructs*, never a hand-maintained
//! allowlist (AGENTS self-describing rule): add an `@editable-mark`, its schema
//! falls out automatically.

pub mod schema;
pub mod validate;

pub use schema::{
    AttrSchema, EditableKind, MarkSchema, SchemaError, SelPosition, collect_schemas_from_matches,
    extract_editable_schema, schemas_to_json,
};
pub use validate::{NodeShape, ValidationSchema, validate_richtext_ast};
