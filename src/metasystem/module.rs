//! Module identity for the metasystem: collections, namespaces, and fully
//! qualified names (FEAT-118 / `docs/specs/module-system.md`).
//!
//! Design invariants (M0 — behavior-preserving):
//! - A definition with NO namespace is GLOBAL. Globality is the empty-namespace
//!   degenerate case of the one keying rule (principle P4), NOT a special path.
//! - The canonical registry key of a global def is its bare name, byte-for-byte
//!   what it was before FQN keying existed. ∴ M0 changes representation, not
//!   behavior: every current def is global, so every key is unchanged.
//! - One separator `/` for namespace path segments; one `:` between collection
//!   and namespace path. These are the ONLY structural characters — they unify
//!   filesystem (`stdlib/scene/camera.st`), FQN (`std:scene/camera`), reference
//!   (`@scene/camera`), and Spell CodePath (`scene/camera.st::§macro`).
//!
//! Namespacing is OPT-IN: a folder becomes a module only when a `@use` import
//! or a `MODULE.st %module` clause assigns it (later milestones). M0 ships the
//! types + keying plumbing with every def still global.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A named root under which modules are resolved.
///
/// Collections decouple a module's *import identity* from where its bytes live
/// (Odin's model): `std:` is the stdlib compiled into the binary, `local:` is
/// the project root, named collections are user-configured roots. `Relative`
/// is a file-relative import (`@use "./sibling"`) that has not yet been
/// anchored to a collection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Collection {
    /// Embedded/standard library (`std:`).
    Std,
    /// Project-local root (`local:`).
    Local,
    /// A user-configured named collection root (`<name>:`).
    Named(String),
    /// File-relative, not yet anchored (`./` or `../`). Carries the raw path.
    Relative(String),
}

impl Collection {
    /// The textual prefix used in an FQN, WITHOUT the trailing `:`.
    /// `Relative` has no collection prefix (it is path-shaped).
    fn prefix(&self) -> Option<&str> {
        match self {
            Collection::Std => Some("std"),
            Collection::Local => Some("local"),
            Collection::Named(name) => Some(name.as_str()),
            Collection::Relative(_) => None,
        }
    }

    /// Parse a collection token (the part before `:` in `coll:ns`).
    fn parse(token: &str) -> Self {
        match token {
            "std" => Collection::Std,
            "local" => Collection::Local,
            other => Collection::Named(other.to_string()),
        }
    }
}

/// A namespace path: the ordered `/`-separated segments under a collection.
///
/// `scene` → `["scene"]`; `scene/effects` → `["scene", "effects"]`. The EMPTY
/// path denotes the collection root (global within that collection).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Namespace {
    /// Collection root. `None` = the implicit global root (no collection), the
    /// state of every definition in M0.
    pub collection: Option<Collection>,
    /// `/`-separated path segments. Empty = collection root.
    pub path: Vec<String>,
}

impl Namespace {
    /// The global namespace: no collection, empty path. Every M0 def has this.
    pub fn global() -> Self {
        Namespace {
            collection: None,
            path: Vec::new(),
        }
    }

    /// True when this is the global (no-collection, empty-path) namespace.
    pub fn is_global(&self) -> bool {
        self.collection.is_none() && self.path.is_empty()
    }

    /// The stable string key of this namespace (its [`Fqn::key`] with an empty
    /// leaf): `coll:a/b` / `a/b` / `./rel`. Used as a map key in [`ImportScope`]
    /// and for diagnostics. The global namespace keys to the empty string.
    pub fn key(&self) -> String {
        if self.is_global() {
            return String::new();
        }
        let mut out = String::new();
        match &self.collection {
            // A relative collection's raw (`./badge`, `../shared/util`) already
            // encodes the full directory shape, which IS the path — the `path`
            // segments are the same components, so the raw alone is the key (no
            // re-appending, which would double it to `./badge/badge`).
            Some(Collection::Relative(raw)) => {
                out.push_str(raw.trim_end_matches('/'));
            }
            Some(coll) => {
                if let Some(prefix) = coll.prefix() {
                    out.push_str(prefix);
                    out.push(':');
                }
                out.push_str(&self.path.join("/"));
            }
            None => out.push_str(&self.path.join("/")),
        }
        out
    }

    /// Build a namespace under a collection from path segments.
    pub fn new(collection: Option<Collection>, path: Vec<String>) -> Self {
        Namespace { collection, path }
    }

    /// Derive a namespace from a `@use` MODULE REFERENCE string.
    ///
    /// Unlike [`Fqn::parse`] (where the last `/` segment is a LEAF name), a
    /// module ref is ALL namespace — every segment is part of the path:
    /// - `std:scene`            → collection Std, path `[scene]`
    /// - `std:scene/effects`    → collection Std, path `[scene, effects]`
    /// - `local:ui/cards`       → collection Local, path `[ui, cards]`
    /// - `./ui/cards` / `../x`  → Relative(dir), path from the tail segments
    /// - `stdlib/macros/scene`  → no collection, path `[stdlib, macros, scene]`
    ///
    /// A trailing `.st`, surrounding quotes, and empty segments are stripped so
    /// `"./scene.st"` and `scene` normalise consistently.
    pub fn from_module_ref(input: &str) -> Self {
        let raw = input.trim().trim_matches('"').trim_matches('\'');

        // Relative refs keep their directory shape as a Relative collection; the
        // path segments are the non-empty, non-`.`/`..` components.
        if raw.starts_with("./") || raw.starts_with("../") {
            // Strip a trailing `.st` from the raw so the namespace KEY reads
            // `./badge` not `./badge.st` (the extension is a file detail, not part
            // of the module identity). File resolution uses `ImportAst.path`
            // (the original ref), not this raw.
            let raw_norm = raw.trim_end_matches(".st");
            let segs: Vec<String> = raw_norm
                .split('/')
                .filter(|s| !s.is_empty() && *s != "." && *s != "..")
                .map(|s| s.trim_end_matches(".st").to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return Namespace::new(Some(Collection::Relative(raw_norm.to_string())), segs);
        }

        let (collection, rest) = match raw.split_once(':') {
            Some((coll, rest)) => (Some(Collection::parse(coll)), rest),
            None => (None, raw),
        };

        let path: Vec<String> = rest
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_end_matches(".st").to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Namespace::new(collection, path)
    }
}

/// A fully qualified name: a namespace plus a leaf identifier.
///
/// The leaf is the bare definition name (`camera`); the namespace locates it.
/// `Fqn` is the canonical registry key once rendered (see [`Fqn::key`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fqn {
    pub namespace: Namespace,
    pub name: String,
}

impl Fqn {
    /// A global FQN: bare leaf, no namespace. Its [`key`](Fqn::key) is the bare
    /// name — identical to the pre-FEAT-118 registry key.
    pub fn global(name: impl Into<String>) -> Self {
        Fqn {
            namespace: Namespace::global(),
            name: name.into(),
        }
    }

    /// Build an FQN from a namespace and leaf.
    pub fn new(namespace: Namespace, name: impl Into<String>) -> Self {
        Fqn {
            namespace,
            name: name.into(),
        }
    }

    /// The canonical registry key.
    ///
    /// - global (no collection, empty path) → `name`  (M0 invariant: unchanged)
    /// - collection + path                  → `coll:a/b/name`
    /// - path only (no collection)          → `a/b/name`
    /// - relative collection                → `./rel/name` (path carried raw)
    pub fn key(&self) -> String {
        let ns = &self.namespace;
        if ns.is_global() {
            return self.name.clone();
        }
        let mut out = String::new();
        match &ns.collection {
            Some(Collection::Relative(raw)) => {
                out.push_str(raw);
                if !raw.ends_with('/') {
                    out.push('/');
                }
            }
            Some(coll) => {
                if let Some(prefix) = coll.prefix() {
                    out.push_str(prefix);
                    out.push(':');
                }
            }
            None => {}
        }
        for seg in &ns.path {
            out.push_str(seg);
            out.push('/');
        }
        out.push_str(&self.name);
        out
    }

    /// Parse a reference/FQN string into structured form.
    ///
    /// Grammar: `(coll ":")? (seg "/")* leaf`. A leading `./` or `../` marks a
    /// relative collection (the whole dir portion is carried raw). No collection
    /// and no `/` yields a global FQN whose key round-trips to the input.
    pub fn parse(input: &str) -> Self {
        // Relative import: keep the directory portion as a raw Relative collection.
        if input.starts_with("./") || input.starts_with("../") {
            let (dir, leaf) = match input.rfind('/') {
                Some(idx) => (&input[..idx], &input[idx + 1..]),
                None => ("", input),
            };
            return Fqn::new(
                Namespace::new(Some(Collection::Relative(dir.to_string())), Vec::new()),
                leaf.to_string(),
            );
        }

        // Optional `coll:` prefix.
        let (collection, rest) = match input.split_once(':') {
            Some((coll, rest)) => (Some(Collection::parse(coll)), rest),
            None => (None, input),
        };

        // Split remaining on `/` into path segments + leaf.
        let mut segments: Vec<String> = rest.split('/').map(|s| s.to_string()).collect();
        let leaf = segments.pop().unwrap_or_default();

        if collection.is_none() && segments.is_empty() {
            return Fqn::global(leaf);
        }
        Fqn::new(Namespace::new(collection, segments), leaf)
    }
}

impl fmt::Display for Fqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key())
    }
}

/// The per-file import environment built from a file's `@use`/`@import`
/// statements (FEAT-118 FUP-057). It is the single substrate the pipeline reads
/// to (a) resolve a qualified `@alias/name` reference to its real namespace,
/// (b) validate that a qualifier is actually bound and a name is actually
/// visible, and (c) scope a `%using` hook to the importing file.
///
/// It is DERIVED data — assembled from already-parsed [`ImportAst`] values
/// (themselves produced from `.st`-declared `%imports` clauses), so it adds no
/// new hardcoded recognition: it is the compiled table form of the `@use`
/// declarations, the same `.st`-declares / Rust-compiles-to-table pattern as
/// the `%registers` vocabulary.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportScope {
    /// Namespaces opened WITHOUT an alias (`@use "std:scene"`). Their public
    /// names are reachable bare (`@camera`). Order-preserving for stable
    /// diagnostics.
    pub open: Vec<Namespace>,
    /// `@use "std:scene" as s` — alias → the namespace it names. A qualified
    /// `@s/camera` reference resolves its leaf against THIS namespace.
    pub aliased: std::collections::BTreeMap<String, Namespace>,
    /// `only (a, b)` allow-lists, keyed by the namespace's [`Namespace::key`].
    /// A namespace absent here imposes no allow-list (open import).
    pub only: std::collections::BTreeMap<String, Vec<String>>,
    /// `hiding (a, b)` exclusion-lists, keyed by the namespace's key.
    pub hiding: std::collections::BTreeMap<String, Vec<String>>,
}

impl ImportScope {
    /// Assemble the scope from a file's parsed imports. Global (`@import`)
    /// entries — those with no namespace — contribute nothing to scoping (they
    /// flat-merge); only namespaced (`@use`) entries populate the environment.
    pub fn from_imports(imports: &[crate::parser::ast::ImportAst]) -> Self {
        let mut scope = ImportScope::default();
        for imp in imports {
            let Some(ns) = imp.namespace.as_ref() else {
                continue; // @import: flat-global, not part of the scope env.
            };
            let key = ns.key();
            if let Some(alias) = &imp.alias {
                scope.aliased.insert(alias.clone(), ns.clone());
            } else {
                scope.open.push(ns.clone());
            }
            if !imp.only.is_empty() {
                scope.only.insert(key.clone(), imp.only.clone());
            }
            if !imp.hiding.is_empty() {
                scope.hiding.insert(key, imp.hiding.clone());
            }
        }
        scope
    }

    /// True when the scope has no namespaced imports — the common case for a
    /// file using only `@import`. Lets the pipeline skip scope work entirely.
    pub fn is_empty(&self) -> bool {
        self.open.is_empty() && self.aliased.is_empty()
    }

    /// Resolve a qualifier (the segment(s) before the leaf in `@q/name`) to the
    /// namespace it denotes, if any. A qualifier is EITHER an alias bound by
    /// `as`, OR a literal namespace-path prefix of an opened/aliased namespace
    /// (so `@scene/camera` works when `std:scene` is open). Returns `None` when
    /// the qualifier names nothing in scope (→ unbound-alias diagnostic).
    pub fn resolve_qualifier(&self, qualifier: &[String]) -> Option<&Namespace> {
        if qualifier.is_empty() {
            return None;
        }
        // 1. Single-segment alias bound by `as`.
        if qualifier.len() == 1
            && let Some(ns) = self.aliased.get(&qualifier[0])
        {
            return Some(ns);
        }
        // 2. Literal namespace-path suffix match against any in-scope namespace
        //    (open or aliased). `@scene/camera` matches `std:scene` because the
        //    qualifier `[scene]` is the tail of that namespace's path.
        self.open
            .iter()
            .chain(self.aliased.values())
            .find(|ns| ns.path.ends_with(qualifier))
    }

    /// True when `name` from namespace `ns` is visible under this scope's
    /// `only`/`hiding` lists. `only` (if present) is an allow-list; `hiding` is
    /// a deny-list. Absent lists impose no restriction.
    pub fn is_visible(&self, ns: &Namespace, name: &str) -> bool {
        let key = ns.key();
        if let Some(allow) = self.only.get(&key)
            && !allow.iter().any(|n| n == name)
        {
            return false;
        }
        if let Some(deny) = self.hiding.get(&key)
            && deny.iter().any(|n| n == name)
        {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_key_is_bare_name() {
        // The M0 invariant: a global def's key is byte-for-byte its bare name.
        assert_eq!(Fqn::global("camera").key(), "camera");
        assert_eq!(Fqn::global("morph-to-color").key(), "morph-to-color");
    }

    #[test]
    fn global_roundtrip() {
        let f = Fqn::parse("camera");
        assert!(f.namespace.is_global());
        assert_eq!(f.name, "camera");
        assert_eq!(f.key(), "camera");
    }

    #[test]
    fn collection_and_path() {
        let f = Fqn::parse("std:scene/camera");
        assert_eq!(f.namespace.collection, Some(Collection::Std));
        assert_eq!(f.namespace.path, vec!["scene".to_string()]);
        assert_eq!(f.name, "camera");
        assert_eq!(f.key(), "std:scene/camera");
    }

    #[test]
    fn nested_path() {
        let f = Fqn::parse("std:scene/effects/glitch");
        assert_eq!(
            f.namespace.path,
            vec!["scene".to_string(), "effects".to_string()]
        );
        assert_eq!(f.name, "glitch");
        assert_eq!(f.key(), "std:scene/effects/glitch");
    }

    #[test]
    fn path_without_collection() {
        let f = Fqn::parse("scene/camera");
        assert_eq!(f.namespace.collection, None);
        assert_eq!(f.namespace.path, vec!["scene".to_string()]);
        assert_eq!(f.name, "camera");
        assert_eq!(f.key(), "scene/camera");
    }

    #[test]
    fn local_collection() {
        let f = Fqn::parse("local:ui/card");
        assert_eq!(f.namespace.collection, Some(Collection::Local));
        assert_eq!(f.key(), "local:ui/card");
    }

    #[test]
    fn named_collection() {
        let f = Fqn::parse("acme:widgets/button");
        assert_eq!(
            f.namespace.collection,
            Some(Collection::Named("acme".to_string()))
        );
        assert_eq!(f.key(), "acme:widgets/button");
    }

    #[test]
    fn relative_import() {
        let f = Fqn::parse("./sibling");
        assert_eq!(f.name, "sibling");
        assert!(matches!(
            f.namespace.collection,
            Some(Collection::Relative(_))
        ));

        let nested = Fqn::parse("../shared/util");
        assert_eq!(nested.name, "util");
        match &nested.namespace.collection {
            Some(Collection::Relative(dir)) => assert_eq!(dir, "../shared"),
            other => panic!("expected relative, got {other:?}"),
        }
    }

    #[test]
    fn hyphenated_leaf_preserved() {
        // `.st` names are hyphenated; the `/` split must not touch `-`.
        let f = Fqn::parse("std:scene/morph-to-color");
        assert_eq!(f.name, "morph-to-color");
        assert_eq!(f.key(), "std:scene/morph-to-color");
    }

    #[test]
    fn global_is_default() {
        assert!(Namespace::default().is_global());
        assert!(Namespace::global().is_global());
    }

    #[test]
    fn module_ref_collection_and_path() {
        let ns = Namespace::from_module_ref("std:scene");
        assert_eq!(ns.collection, Some(Collection::Std));
        assert_eq!(ns.path, vec!["scene".to_string()]);
    }

    #[test]
    fn module_ref_is_all_namespace_no_leaf() {
        // Unlike Fqn::parse, the LAST segment of a module ref is namespace, not a leaf.
        let ns = Namespace::from_module_ref("std:scene/effects");
        assert_eq!(ns.path, vec!["scene".to_string(), "effects".to_string()]);
        // And a macro `glitch` from that module keys as scene/effects/glitch.
        assert_eq!(Fqn::new(ns, "glitch").key(), "std:scene/effects/glitch");
    }

    #[test]
    fn module_ref_no_collection() {
        let ns = Namespace::from_module_ref("stdlib/macros/scene");
        assert_eq!(ns.collection, None);
        assert_eq!(
            ns.path,
            vec![
                "stdlib".to_string(),
                "macros".to_string(),
                "scene".to_string()
            ]
        );
    }

    #[test]
    fn module_ref_relative_strips_dotsegs_and_st() {
        let ns = Namespace::from_module_ref("./ui/cards.st");
        assert!(matches!(ns.collection, Some(Collection::Relative(_))));
        assert_eq!(ns.path, vec!["ui".to_string(), "cards".to_string()]);

        let parent = Namespace::from_module_ref("../shared/util");
        assert_eq!(parent.path, vec!["shared".to_string(), "util".to_string()]);
    }

    #[test]
    fn module_ref_quotes_stripped() {
        let ns = Namespace::from_module_ref("\"std:scene\"");
        assert_eq!(ns.collection, Some(Collection::Std));
        assert_eq!(ns.path, vec!["scene".to_string()]);
    }

    // ---- ImportScope (FUP-057) ----

    use crate::parser::ast::ImportAst;

    fn use_import(path: &str, alias: Option<&str>, only: &[&str], hiding: &[&str]) -> ImportAst {
        ImportAst {
            path: path.to_string(),
            namespace: Some(Namespace::from_module_ref(path)),
            alias: alias.map(|s| s.to_string()),
            only: only.iter().map(|s| s.to_string()).collect(),
            hiding: hiding.iter().map(|s| s.to_string()).collect(),
            span: Default::default(),
        }
    }

    #[test]
    fn namespace_key_round_trips() {
        assert_eq!(Namespace::global().key(), "");
        assert_eq!(Namespace::from_module_ref("std:scene").key(), "std:scene");
        assert_eq!(
            Namespace::from_module_ref("std:scene/effects").key(),
            "std:scene/effects"
        );
        assert_eq!(
            Namespace::from_module_ref("local:ui/cards").key(),
            "local:ui/cards"
        );
    }

    #[test]
    fn import_scope_open_and_aliased() {
        let imports = vec![
            use_import("std:scene", None, &[], &[]),
            use_import("std:effects", Some("fx"), &[], &[]),
        ];
        let scope = ImportScope::from_imports(&imports);
        assert_eq!(scope.open.len(), 1);
        assert!(scope.aliased.contains_key("fx"));
        assert!(!scope.is_empty());
    }

    #[test]
    fn import_scope_global_import_is_ignored() {
        // @import (namespace: None) contributes nothing to scoping.
        let imports = vec![ImportAst {
            path: "./shared.st".to_string(),
            namespace: None,
            alias: None,
            only: vec![],
            hiding: vec![],
            span: Default::default(),
        }];
        let scope = ImportScope::from_imports(&imports);
        assert!(scope.is_empty());
    }

    #[test]
    fn resolve_qualifier_alias_and_path_suffix() {
        let imports = vec![
            use_import("std:scene", None, &[], &[]),
            use_import("std:effects", Some("fx"), &[], &[]),
        ];
        let scope = ImportScope::from_imports(&imports);
        // Alias resolves.
        let by_alias = scope.resolve_qualifier(&["fx".to_string()]).unwrap();
        assert_eq!(by_alias.key(), "std:effects");
        // Path-suffix of an open namespace resolves (`@scene/...`).
        let by_path = scope.resolve_qualifier(&["scene".to_string()]).unwrap();
        assert_eq!(by_path.key(), "std:scene");
        // Unknown qualifier → None (drives the unbound-alias diagnostic).
        assert!(scope.resolve_qualifier(&["nope".to_string()]).is_none());
    }

    #[test]
    fn is_visible_respects_only_and_hiding() {
        let imports = vec![
            use_import("std:scene", None, &["camera", "light"], &[]),
            use_import("std:effects", Some("fx"), &[], &["glitch"]),
        ];
        let scope = ImportScope::from_imports(&imports);
        let scene = Namespace::from_module_ref("std:scene");
        let effects = Namespace::from_module_ref("std:effects");
        // only: allow-list
        assert!(scope.is_visible(&scene, "camera"));
        assert!(!scope.is_visible(&scene, "fog"));
        // hiding: deny-list
        assert!(scope.is_visible(&effects, "bloom"));
        assert!(!scope.is_visible(&effects, "glitch"));
    }
}
