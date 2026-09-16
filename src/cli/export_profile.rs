//! Export a Spell `LanguageProfile` bundle from the live metasystem registry
//! (FEAT-118 / `docs/specs/spell-spacetime-unification.md` §4).
//!
//! This is the Spacetime→Spell bridge: it emits a data profile describing the
//! `.st` language SO THAT the Spell harness can give `.st` files outline /
//! `::§kind` / structural-edit / graph intelligence WITHOUT a hand-written Rust
//! `LanguageProfile` (which would violate both repos' self-describing laws).
//!
//! Single source of truth: the same `MetaRegistry` the runtime parser uses. Edit
//! a `%form` in stdlib → re-export → Spell's `.st` intelligence updates. Zero
//! drift (the design's core promise).
//!
//! What it emits (the serde-able half of Spell's `LanguageProfile`):
//! - `id` / `extensions`
//! - `declarations`: every `%macro` / `%primitive` / `%capture_type` as a named
//!   declaration with its directive trigger + the `§kind` it maps to
//! - `kind_aliases`: `§function|§class|§decl|…` → the Spacetime construct kinds
//!   that satisfy them (the cross-language `::§kind` map)
//! - `references`: directive names that are references, not definitions
//! - `separators`: `/` (the namespace path axis — module-system.md §10)
//!
//! NB: the `ts_language` (grammar) + `node-types.json` are a separate emission
//! (the tree-sitter generator); this command owns the PROFILE half. They compose
//! into the bundle Spell loads.

use std::path::PathBuf;

use serde::Serialize;

use crate::metasystem::MetaRegistry;

/// A Spell `LanguageProfile` — the data half (no `ts_language`/procedures).
///
/// Field names mirror the Spell `pi-code-engine` `ProfileYaml`/`LanguageProfile`
/// serde shape so the JSON loads directly via `register_profile_from_json`.
#[derive(Debug, Clone, Serialize)]
pub struct SpellProfile {
    /// Language id (`"spacetime"`).
    pub id: String,
    /// File extensions (without the dot).
    pub extensions: Vec<String>,
    /// `§kind` → the construct kinds satisfying it (the `::§kind` resolution map).
    pub kind_aliases: KindAliases,
    /// Symbol-name path separator(s). `/` is the namespace axis.
    pub separators: Vec<String>,
    /// `§kind` → the GENERATED tree-sitter node kinds that represent it
    /// (`export-spell-profile`'s grammar half, FUP-054). Bridges the profile to
    /// the faithful grammar's named definition nodes so Spell resolves a `§kind`
    /// to concrete CST nodes with a `name` field. `§function`→macro_def/
    /// primitive_def, `§decl`→capture_type_def. (`§class`/`§import` reuse these
    /// node kinds, discriminated by the profile's `construct`/declarations.)
    pub ts_node_kinds: TsNodeKinds,
    /// Every addressable definition the language declares.
    pub declarations: Vec<Declaration>,
    /// Directive names that are references (uses), not definitions.
    pub references: Vec<String>,
    /// Provenance: how/when this profile was produced.
    pub generated_by: String,
}

/// The cross-language `§kind` aliases for `.st`. Each maps a universal Spell
/// `§kind` to the Spacetime construct kinds that satisfy it.
#[derive(Debug, Clone, Serialize)]
pub struct KindAliases {
    /// `§function` — a callable/composable construct: `%macro`, `%primitive`.
    pub function: Vec<String>,
    /// `§class` — a component-like construct (template/component macros).
    pub class: Vec<String>,
    /// `§decl` — a declaration of a type/grammar fragment: `%capture_type`.
    pub decl: Vec<String>,
    /// `§import` — module-loading directives: `@use`, `@import`.
    pub import: Vec<String>,
}

/// The faithful-grammar tree-sitter node kinds backing each `§kind`
/// (FUP-054). These are the names emitted by the generated grammar
/// (`tree-sitter-spacetime/`), so Spell's tree-sitter layer can target real
/// CST nodes (each carries a `name` field).
#[derive(Debug, Clone, Serialize)]
pub struct TsNodeKinds {
    /// `§function` → `%macro` / `%primitive` definition nodes.
    pub function: Vec<String>,
    /// `§class` → component-bearing macro nodes (same node, `construct`-tagged).
    pub class: Vec<String>,
    /// `§decl` → `%capture_type` definition nodes.
    pub decl: Vec<String>,
}

/// One addressable definition in `.st`.
#[derive(Debug, Clone, Serialize)]
pub struct Declaration {
    /// The defined name (e.g. `camera`, `morph-to-color`).
    pub name: String,
    /// The Spacetime meta-kind that defined it: `macro` | `primitive` |
    /// `capture_type`.
    pub construct: String,
    /// The `§kind` this declaration resolves to in Spell.
    pub kind: String,
    /// The `@directive` it introduces, when it carries a `%form` (else `None`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directive: Option<String>,
    /// Leading doc-comment, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

/// Build a [`SpellProfile`] from a loaded registry. Pure data transform — the
/// single source of truth is `registry`.
pub fn build_spell_profile(registry: &MetaRegistry) -> SpellProfile {
    let mut declarations: Vec<Declaration> = Vec::new();
    let mut references: Vec<String> = Vec::new();

    // %macro → §function, except module-loaders (§import) and component-like
    // (§class). A macro carrying an %imports clause is an import directive.
    let mut macro_names: Vec<&str> = registry.macro_names().collect();
    macro_names.sort();
    for name in macro_names {
        let Some(mac) = registry.get_macro(name) else {
            continue;
        };
        let directive = mac.form.as_ref().map(|f| f.directive_name.clone());
        let kind = if mac.imports.is_some() {
            "§import"
        } else if is_component_like(name, mac) {
            "§class"
        } else {
            "§function"
        };
        declarations.push(Declaration {
            name: mac.name.clone(),
            construct: "macro".to_string(),
            kind: kind.to_string(),
            directive: directive.clone(),
            doc: mac.doc.clone(),
        });
        if let Some(d) = directive {
            references.push(d);
        }
    }

    // %primitive → §function (the JS/CSS-emitting leaves).
    let mut prim_names: Vec<&str> = registry.primitive_names().collect();
    prim_names.sort();
    for name in prim_names {
        let Some(prim) = registry.get_primitive(name) else {
            continue;
        };
        declarations.push(Declaration {
            name: prim.name.clone(),
            construct: "primitive".to_string(),
            kind: "§function".to_string(),
            directive: None,
            doc: prim.doc.clone(),
        });
    }

    // %capture_type → §decl (grammar-fragment declarations).
    let mut ct_names: Vec<&str> = registry.capture_type_names().collect();
    ct_names.sort();
    for name in ct_names {
        let Some(ct) = registry.get_capture_type(name) else {
            continue;
        };
        declarations.push(Declaration {
            name: ct.name.clone(),
            construct: "capture_type".to_string(),
            kind: "§decl".to_string(),
            directive: None,
            // CaptureTypeDefAst carries no doc-comment field today.
            doc: None,
        });
    }

    references.sort();
    references.dedup();

    SpellProfile {
        id: "spacetime".to_string(),
        extensions: vec!["st".to_string()],
        kind_aliases: KindAliases {
            function: vec!["macro".to_string(), "primitive".to_string()],
            class: vec!["template".to_string()],
            decl: vec!["capture_type".to_string()],
            import: vec!["use".to_string(), "import".to_string()],
        },
        separators: vec!["/".to_string()],
        ts_node_kinds: TsNodeKinds {
            function: vec!["macro_def".to_string(), "primitive_def".to_string()],
            class: vec!["macro_def".to_string()],
            decl: vec!["capture_type_def".to_string()],
        },
        declarations,
        references,
        generated_by: format!(
            "spacetime export-spell-profile (v{})",
            env!("CARGO_PKG_VERSION")
        ),
    }
}

/// A macro is component-like (`§class`) when it carries a `component_body` body
/// capture — the registry-derived "body-bearing" notion (mirrors the FEAT-116
/// predicate). Template/component constructs map to Spell `§class`.
fn is_component_like(_name: &str, mac: &crate::parser::meta_ast::MacroDefAst) -> bool {
    mac.form
        .as_ref()
        .and_then(|f| f.body_capture.as_deref())
        .is_some_and(|b| b.contains(":component_body"))
}

/// Entry point for the `export-spell-profile` CLI command.
pub fn run_export_spell_profile(
    output: Option<PathBuf>,
    registry: &MetaRegistry,
) -> Result<(), String> {
    let profile = build_spell_profile(registry);
    let json = serde_json::to_string_pretty(&profile)
        .map_err(|e| format!("failed to serialize Spell profile: {e}"))?;

    match output {
        Some(path) => {
            std::fs::write(&path, &json)
                .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
            eprintln!(
                "Wrote Spell profile: {} ({} declarations, {} references)",
                path.display(),
                profile.declarations.len(),
                profile.references.len()
            );
        }
        None => println!("{json}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_from(src: &str) -> MetaRegistry {
        let parsed = crate::parser::parse_for_bootstrap(src).expect("parse");
        let mut reg = MetaRegistry::new();
        reg.load_from_defs(parsed.meta_defs).expect("load");
        reg
    }

    #[test]
    fn macro_maps_to_function() {
        let reg = registry_from("%macro spotlight {\n  %form { @spotlight $x:string }\n}");
        let p = build_spell_profile(&reg);
        let d = p
            .declarations
            .iter()
            .find(|d| d.name == "spotlight")
            .expect("decl");
        assert_eq!(d.kind, "§function");
        assert_eq!(d.construct, "macro");
        assert_eq!(d.directive.as_deref(), Some("@spotlight"));
        assert!(p.references.contains(&"@spotlight".to_string()));
    }

    #[test]
    fn import_macro_maps_to_import() {
        let reg = registry_from(
            "%macro use {\n  %form { @use $path:string }\n  %imports { module: $path }\n}",
        );
        let p = build_spell_profile(&reg);
        let d = p
            .declarations
            .iter()
            .find(|d| d.name == "use")
            .expect("decl");
        assert_eq!(
            d.kind, "§import",
            "an %imports-bearing macro is §import, not §function"
        );
    }

    #[test]
    fn capture_type_maps_to_decl() {
        let reg = registry_from("%capture_type color_value {\n  $r:string\n}");
        let p = build_spell_profile(&reg);
        let d = p
            .declarations
            .iter()
            .find(|d| d.name == "color_value")
            .expect("capture-type decl present");
        assert_eq!(d.kind, "§decl");
        assert_eq!(d.construct, "capture_type");
    }

    #[test]
    fn profile_carries_separator_and_extension() {
        let reg = registry_from("%macro x {\n  %form { @x }\n}");
        let p = build_spell_profile(&reg);
        assert_eq!(p.extensions, vec!["st".to_string()]);
        assert_eq!(p.separators, vec!["/".to_string()], "the / namespace axis");
        assert_eq!(p.id, "spacetime");
    }

    #[test]
    fn ts_node_kinds_bridge_to_faithful_grammar() {
        // FUP-054: the profile names the GENERATED grammar's def nodes so Spell
        // resolves a §kind to real CST nodes (each with a `name` field).
        let reg = registry_from("%macro x {\n  %form { @x }\n}");
        let p = build_spell_profile(&reg);
        assert!(p.ts_node_kinds.function.contains(&"macro_def".to_string()));
        assert!(
            p.ts_node_kinds
                .function
                .contains(&"primitive_def".to_string())
        );
        assert_eq!(p.ts_node_kinds.decl, vec!["capture_type_def".to_string()]);
    }
}
