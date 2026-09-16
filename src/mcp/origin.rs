//! Provenance for loaded Spacetime fragments (FEAT-128).
//!
//! Every loaded fragment is tagged with WHERE it came from, so the unified live
//! environment can show "this came from stdlib", "this from env-3", "this was
//! sent by the agent". One field on two existing rails — no new mechanism:
//! - macros derive their origin from `MacroDefAst.source_file` (see
//!   `build_dispatch_json` in `compiler.rs`).
//! - [`crate::mcp::live::FunctionRecord`] carries an `Origin` set at `st_fn_put`.
//!
//! Fork 3-B (PLAN-041): FLAT registry + origin as METADATA (not namespaced). The
//! origin is descriptive provenance; it never changes a fragment's lookup key.

use serde_json::{Value, json};

/// Where a loaded Spacetime fragment came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Shipped with the toolchain (a path under `stdlib/`, or an embedded
    /// stdlib def).
    Stdlib,
    /// Bound to a discovered/opened environment (an import path). Carries the
    /// `env_id` so the UI can filter by environment.
    Env(String),
    /// Inline source sent by the agent through `st_fn_put` (no env binding).
    Agent,
    /// Reserved: a curated kit fragment (not yet produced; kept so the JSON
    /// contract and the chip palette are stable ahead of the kit work).
    #[cfg_attr(not(test), allow(dead_code))]
    Kit,
}

impl Origin {
    /// Stable machine token used as the CSS chip key (`data-origin="..."`) and
    /// the filter value. One token per variant; the `Env` id rides in `id`.
    pub fn kind(&self) -> &'static str {
        match self {
            Origin::Stdlib => "stdlib",
            Origin::Env(_) => "env",
            Origin::Agent => "agent",
            Origin::Kit => "kit",
        }
    }

    /// Human label shown on the chip. `Env` folds its id in: `Env(env-3)`.
    pub fn label(&self) -> String {
        match self {
            Origin::Stdlib => "Stdlib".to_string(),
            Origin::Env(id) => format!("Env({id})"),
            Origin::Agent => "Agent".to_string(),
            Origin::Kit => "Kit".to_string(),
        }
    }

    /// The environment id for [`Origin::Env`], else `None`.
    pub fn id(&self) -> Option<&str> {
        match self {
            Origin::Env(id) => Some(id.as_str()),
            _ => None,
        }
    }

    /// Serialize to the object shape the `.st` workbench templates consume:
    /// `{ kind, id, label }`. `id` is `null` for non-env origins. The chip keys
    /// its color off `kind`, shows `label`, and can filter by `id`.
    pub fn to_json(&self) -> Value {
        json!({
            "kind": self.kind(),
            "id": self.id(),
            "label": self.label(),
        })
    }

    /// Derive a macro's origin from its `source_file` (the same signal the
    /// `$macros` gallery already uses for `kind`): a path under `stdlib/` (or an
    /// embedded def with no path) is [`Origin::Stdlib`]; anything else is an
    /// authored project file → [`Origin::Env`] with an empty id (the macro rail
    /// has no env binding — only the function rail does).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn from_source_file(source_file: Option<&str>) -> Origin {
        match source_file {
            Some(sf) if !is_stdlib_path(sf) => Origin::Env(String::new()),
            _ => Origin::Stdlib,
        }
    }
}

/// True when a source path belongs to the SHIPPED toolchain stdlib. Both the
/// embedded loader (`stdlib/{category}/{file}`) and the filesystem loader (given
/// relative dirs like `stdlib/primitives`) stamp a LEADING `stdlib/` segment, so
/// we match the prefix — NOT a `/stdlib/` substring, which would misclassify an
/// authored project file under a dir named `stdlib/` (e.g.
/// `projects/foo/stdlib/card.st`) as shipped (FEAT-128 reviewer finding).
#[cfg_attr(not(test), allow(dead_code))]
fn is_stdlib_path(path: &str) -> bool {
    path.starts_with("stdlib/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_tokens_are_stable() {
        assert_eq!(Origin::Stdlib.kind(), "stdlib");
        assert_eq!(Origin::Env("env-3".into()).kind(), "env");
        assert_eq!(Origin::Agent.kind(), "agent");
        assert_eq!(Origin::Kit.kind(), "kit");
    }

    #[test]
    fn env_label_folds_in_id() {
        assert_eq!(Origin::Env("env-3".into()).label(), "Env(env-3)");
        assert_eq!(Origin::Env("env-3".into()).id(), Some("env-3"));
        assert_eq!(Origin::Stdlib.id(), None);
    }

    #[test]
    fn json_shape_is_kind_id_label() {
        let v = Origin::Env("env-3".into()).to_json();
        assert_eq!(v["kind"], "env");
        assert_eq!(v["id"], "env-3");
        assert_eq!(v["label"], "Env(env-3)");

        let s = Origin::Stdlib.to_json();
        assert_eq!(s["kind"], "stdlib");
        assert!(s["id"].is_null());
        assert_eq!(s["label"], "Stdlib");
    }

    #[test]
    fn macro_origin_derives_from_source_file() {
        assert_eq!(
            Origin::from_source_file(Some("stdlib/text/balance.st")),
            Origin::Stdlib
        );
        // An embedded def (no path) is stdlib.
        assert_eq!(Origin::from_source_file(None), Origin::Stdlib);
        // An authored project file is env-origin (empty id on the macro rail).
        assert_eq!(
            Origin::from_source_file(Some("projects/foo/index.st")),
            Origin::Env(String::new())
        );
        // A project dir merely NAMED `stdlib/` is NOT the shipped toolchain stdlib
        // — only a LEADING `stdlib/` counts (FEAT-128 reviewer finding).
        assert_eq!(
            Origin::from_source_file(Some("projects/foo/stdlib/card.st")),
            Origin::Env(String::new())
        );
        assert_eq!(
            Origin::from_source_file(Some("/home/u/proj/stdlib/render.st")),
            Origin::Env(String::new())
        );
    }
}
