//! Registry for primitives, macros, and presets.
//!
//! Stores and retrieves %primitive, %macro, and %preset definitions.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::parser::SourceSpan;
use crate::parser::meta_ast::{
    CaptureTypeDefAst, CommentTypeDefAst, MacroDefAst, MacroScope, MetaDef, MetaPresetDefAst,
    MigrationDefAst, PrimitiveDefAst, RegistryTarget, RuntimeRegistryDef, ScalarTypeDefAst,
    VendorDefAst,
};
use crate::pipeline::{StdlibLoadError, StdlibLoadErrorKind};
use crate::syntax::CapturedValue;
use crate::syntax::form_scoring::{CaptureMapInput, score_macro_form};

/// Stores concrete expression transformation rules
/// Populated during macro expansion when a macro has a %transforms clause
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransformRegistry {
    /// Maps function name -> output template
    /// e.g., "formatPrice" -> "SpacetimeFunctions.formatPrice"
    pub fn_transforms: HashMap<String, TransformRule>,
}

/// A concrete transform rule for function call rewriting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformRule {
    /// The resolved output prefix (e.g., "SpacetimeFunctions.")
    pub output_prefix: String,
    /// The original output template for reference
    pub output_template: String,
}

impl TransformRegistry {
    pub fn new() -> Self {
        Self {
            fn_transforms: HashMap::new(),
        }
    }

    /// Register a transform for a function name
    pub fn register(&mut self, fn_name: &str, rule: TransformRule) {
        self.fn_transforms.insert(fn_name.to_string(), rule);
    }

    /// Get transform rule for a function name
    pub fn get(&self, fn_name: &str) -> Option<&TransformRule> {
        self.fn_transforms.get(fn_name)
    }
}

/// Error loading or accessing the registry
#[derive(Debug, Clone)]
pub struct MetaRegistryError {
    pub kind: MetaRegistryErrorKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum MetaRegistryErrorKind {
    /// Primitive already defined
    DuplicatePrimitive(String),
    /// Macro already defined
    DuplicateMacro(String),
    /// Migration already defined
    DuplicateMigration(String),
    /// Migration failed structural validation (date shape, rewrite/hint
    /// exclusivity, unknown hole, live-macro collision, cycle)
    InvalidMigration(String),
    /// Comment type already defined — the id an author writes in `//@<id>`
    /// must resolve to exactly ONE declaration, or the roster is ambiguous.
    DuplicateCommentType(String),
    /// Scalar type already defined — the id a consumer (type recognition,
    /// schema, zero, widget) looks up must resolve to exactly ONE declaration,
    /// or the table stops being a single source of truth.
    DuplicateScalarType(String),
    /// File could not be parsed
    ParseError(String),
    /// IO error reading file
    IoError(String),
}

impl std::fmt::Display for MetaRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            MetaRegistryErrorKind::DuplicatePrimitive(name) => {
                write!(f, "duplicate primitive definition: {}", name)
            }
            MetaRegistryErrorKind::DuplicateMacro(name) => {
                write!(f, "duplicate macro definition: {}", name)
            }
            MetaRegistryErrorKind::DuplicateMigration(name) => {
                write!(f, "duplicate migration definition: {}", name)
            }
            MetaRegistryErrorKind::InvalidMigration(msg) => {
                write!(f, "invalid migration: {}", msg)
            }
            MetaRegistryErrorKind::DuplicateCommentType(name) => {
                write!(
                    f,
                    "duplicate comment type: {name} — a `//@{name}` comment would be ambiguous. \
                     Comment types are declared once, in stdlib/comments/entries/ or in the \
                     project's _prelude.st."
                )
            }
            MetaRegistryErrorKind::DuplicateScalarType(name) => {
                write!(
                    f,
                    "duplicate scalar type: {name} — every consumer (schema, zero, widget) must \
                     agree on what `{name}` means, so it is declared once, in stdlib/scalars/."
                )
            }
            MetaRegistryErrorKind::ParseError(msg) => {
                write!(f, "parse error: {}", msg)
            }
            MetaRegistryErrorKind::IoError(msg) => {
                write!(f, "IO error: {}", msg)
            }
        }
    }
}

impl std::error::Error for MetaRegistryError {}

/// Result of loading stdlib files with collected errors
#[derive(Debug, Default)]
pub struct StdlibLoadResult {
    /// Number of files successfully loaded
    pub loaded_count: usize,
    /// Errors encountered during loading (collected, not fatal)
    pub errors: Vec<StdlibLoadError>,
}

/// The canonical compile-side stdlib directory list (ONE list, every loader
/// shares it: full load, incremental cache, grammar generation). Order
/// matters: `stdlib/macros` before `stdlib/migrations/entries` so the
/// PLAN-076 shadow guard (migration vs live-macro collision) sees every
/// live macro at migration-registration time.
///
/// `stdlib/migrations/entries` is the ONLY part of the migrations module
/// loaded into the registry — its `__dev__/` (pill widget) is
/// dev-server-compiled, never registry-loaded. `stdlib/comments/entries`
/// (PLAN-123 comment types) follows the same carve-out for the same reason.
pub const STDLIB_DIRS: &[&str] = &[
    "stdlib/capture-types",
    "stdlib/runtime",
    "stdlib/primitives",
    "stdlib/syntax",
    "stdlib/macros",
    "stdlib/testing",
    "stdlib/migrations/entries",
    "stdlib/comments/entries",
    // FEAT-168 / PLAN-122 W4: the scalar type table (the SINGLE SOURCE for what
    // a scalar means — schema, zero, widget). It sits after comments/entries,
    // grouped with the other kinds-as-data table, and has no ordering
    // dependency: its `%capture` references are strings resolved downstream,
    // and a redefined scalar is a duplicate ERROR, never a later-dir override.
    "stdlib/scalars",
    // PLAN-077 W1: the `enum` module (tagged-union @match grammar + derive/
    // dispatch/state machinery) is an ALWAYS-ON core construct, so it registers
    // via the same directory scan as the legacy flat dirs — its `match_arm`
    // capture type must resolve for registry-scanned forms (@match/@view in
    // stdlib/enum/dispatch.st, W5), which the opt-in import path cannot guarantee.
    "stdlib/enum",
];

impl StdlibLoadResult {
    /// Check if any errors were collected
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Registry holding primitive, macro, and preset definitions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetaRegistry {
    primitives: HashMap<String, PrimitiveDefAst>,
    macros: HashMap<String, MacroDefAst>,
    /// Presets keyed by "category:name" (e.g., "easing:linear")
    presets: HashMap<String, MetaPresetDefAst>,
    /// Transform rules populated during macro expansion
    pub transform_registry: TransformRegistry,
    /// Runtime registry definitions (functions, filters, etc.)
    pub runtime_registries: HashMap<String, RuntimeRegistryDef>,
    /// Custom capture types defined via %capture_type
    capture_types: HashMap<String, CaptureTypeDefAst>,
    /// Vendored dependency definitions declared via %vendor
    vendors: HashMap<String, VendorDefAst>,
    /// Current compilation target (default: "js")
    pub current_target: String,
    /// Maps binding names to macros that provide them (built from %binds outputs)
    /// e.g., "gl" -> ["scene"], "t" -> ["scene"]
    provider_index: HashMap<String, Vec<String>>,
    /// Per-namespace MODULE.st manifests (FEAT-118 M2). Keyed by the module
    /// namespace name (`scene`). RETAINS the `.st`-declared `%public` /
    /// `%reexport` / `%using` data so enforcement reads it back (self-hosted:
    /// the policy lives in `.st`, the registry just stores + serves it). Empty
    /// for folders without a `MODULE.st` manifest.
    module_manifests: HashMap<String, crate::parser::ast::ModuleManifest>,
    /// Syntax migrations (PLAN-076), keyed by wave date (`YYYY-MM-DD` —
    /// lexicographic order == chronological order, so iteration IS wave
    /// order). A wave = every migration sharing one date.
    #[serde(default)]
    migrations: BTreeMap<String, Vec<MigrationDefAst>>,
    /// Migration id -> wave date (fast `get_migration` without scanning waves)
    #[serde(default)]
    migration_ids: HashMap<String, String>,
    /// Registered migration macro name -> (migration id, rewrite rule id).
    /// Covers BOTH embedded retired macros (rule = None) and match-only rule
    /// defs (rule = Some). PLAN-079.
    #[serde(default)]
    macro_to_migration: HashMap<String, (String, Option<String>)>,
    /// Comment types (PLAN-123), keyed by type id. Sorted iteration order is
    /// what makes the roster STABLE across the pill, `check`, and MCP — three
    /// readers that must agree on what exists, so a BTreeMap, not a HashMap.
    ///
    /// Populated from `stdlib/comments/entries/` AND from a project's
    /// `_prelude.st` overlay. The registry does not distinguish the two:
    /// `source_file` on each def records provenance for diagnostics, but a
    /// project type is a first-class citizen of the roster.
    #[serde(default)]
    comment_types: BTreeMap<String, CommentTypeDefAst>,
    /// Scalar types (FEAT-168 / PLAN-122 W4), keyed by scalar id. This is the
    /// SINGLE SOURCE for what a scalar means: its JSON schema, zero value, and
    /// admin widget. Populated from `stdlib/scalars/`; the registry does not
    /// distinguish a stdlib default from a project override — `source_file` on
    /// each def records provenance for diagnostics.
    #[serde(default)]
    scalar_types: BTreeMap<String, ScalarTypeDefAst>,
}

impl MetaRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            primitives: HashMap::new(),
            macros: HashMap::new(),
            presets: HashMap::new(),
            transform_registry: TransformRegistry::new(),
            runtime_registries: HashMap::new(),
            capture_types: HashMap::new(),
            vendors: HashMap::new(),
            current_target: "js".to_string(),
            provider_index: HashMap::new(),
            module_manifests: HashMap::new(),
            migrations: BTreeMap::new(),
            comment_types: BTreeMap::new(),
            scalar_types: BTreeMap::new(),
            migration_ids: HashMap::new(),
            macro_to_migration: HashMap::new(),
        }
    }

    /// Register a primitive definition
    pub fn register_primitive(&mut self, def: PrimitiveDefAst) -> Result<(), MetaRegistryError> {
        if self.primitives.contains_key(&def.name) {
            return Err(MetaRegistryError {
                kind: MetaRegistryErrorKind::DuplicatePrimitive(def.name.clone()),
                span: def.span,
            });
        }
        self.primitives.insert(def.name.clone(), def);
        Ok(())
    }

    /// The canonical registry key for a macro definition (FEAT-118).
    ///
    /// A macro with no `module` is GLOBAL and its key is its bare name —
    /// byte-for-byte the pre-FEAT-118 key (principle P4: global = the
    /// empty-namespace degenerate case). A namespaced macro keys on its FQN
    /// (`coll:path/name`). Until `@use` / `MODULE.st %module` assigns a
    /// namespace (later milestones), every def is global, so this is a no-op
    /// rename of the existing keying.
    pub(crate) fn macro_key(def: &MacroDefAst) -> String {
        match &def.module {
            None => def.name.clone(),
            Some(ns) if ns.is_global() => def.name.clone(),
            Some(ns) => crate::metasystem::module::Fqn::new(ns.clone(), def.name.clone()).key(),
        }
    }

    /// Register a macro definition
    pub fn register_macro(&mut self, def: MacroDefAst) -> Result<(), MetaRegistryError> {
        let key = Self::macro_key(&def);
        if self.macros.contains_key(&key) {
            return Err(MetaRegistryError {
                kind: MetaRegistryErrorKind::DuplicateMacro(def.name.clone()),
                span: def.span,
            });
        }
        // PLAN-076 shadow guard (symmetric direction): a live macro must not
        // claim a form directive a registered migration still owns — retired
        // syntax stays retired. Re-introduction of a directive is a FUP that
        // needs registry-side coexistence machinery, not a silent override.
        // (PLAN-079: a migration owns the directives of its EMBEDDED macros.)
        if let Some(form) = &def.form {
            let directive = form.directive_name.trim_start_matches('@');
            for migrations in self.migrations.values() {
                for mig in migrations {
                    if mig.retired_directives().iter().any(|d| d == directive) {
                        return Err(MetaRegistryError {
                            kind: MetaRegistryErrorKind::InvalidMigration(format!(
                                "macro `{}` form @{directive} collides with migration `{}` — \
                                 a migration owns its directive until the wave is deleted",
                                def.name, mig.id
                            )),
                            span: def.span,
                        });
                    }
                }
            }
        }
        self.register_macro_inner(def)
    }

    /// Insert a macro with only the duplicate-key check. The migration-owned
    /// registration path uses this (the migration IS the directive owner —
    /// the live-vs-retired shadow guard doesn't apply to it; the reverse
    /// direction is enforced by `validate_migration` at load).
    fn register_macro_inner(&mut self, def: MacroDefAst) -> Result<(), MetaRegistryError> {
        let key = Self::macro_key(&def);
        if self.macros.contains_key(&key) {
            return Err(MetaRegistryError {
                kind: MetaRegistryErrorKind::DuplicateMacro(def.name.clone()),
                span: def.span,
            });
        }
        self.macros.insert(key, def);
        Ok(())
    }

    /// Register a syntax migration (PLAN-076).
    ///
    /// Runs the full structural validation (`validate_migration`) at load —
    /// a bad migration is a stdlib LOAD ERROR, never a deferred surprise.
    /// Migrations bucket by `%date`; iteration over the BTreeMap is wave
    /// order (lexicographic ISO date == chronological).
    pub fn register_migration(&mut self, def: MigrationDefAst) -> Result<(), MetaRegistryError> {
        if self.migration_ids.contains_key(&def.id) {
            return Err(MetaRegistryError {
                kind: MetaRegistryErrorKind::DuplicateMigration(def.id.clone()),
                span: def.span,
            });
        }
        if let Err(errors) = super::validate::validate_migration(&def, self) {
            let msg = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(MetaRegistryError {
                kind: MetaRegistryErrorKind::InvalidMigration(msg),
                span: def.span,
            });
        }
        self.migration_ids.insert(def.id.clone(), def.date.clone());
        // PLAN-079 capsule: the migration's macros ARE the registration —
        // every embedded macro (tagged retired) plus one match-only def per
        // rewrite rule. Replaces PLAN-076's synthetic-macro special-case.
        for mac in def.registration_macros() {
            let tag = mac.retired.clone().expect("registration macros are tagged");
            self.macro_to_migration
                .insert(Self::macro_key(&mac), (def.id.clone(), tag.rule.clone()));
            self.register_macro_inner(mac)?;
        }
        self.migrations
            .entry(def.date.clone())
            .or_default()
            .push(def);
        Ok(())
    }

    /// Resolve a matched macro name to its owning migration + rewrite rule
    /// (`None` rule = an embedded retired macro; `Some` = a match-only rule
    /// def). This is the pipeline's FormMatch → migration lookup
    /// (PLAN-079): `FormMatch.matched_macro` carries the exact def name.
    pub fn retired_by_macro(&self, macro_name: &str) -> Option<&(String, Option<String>)> {
        self.macro_to_migration.get(macro_name)
    }

    /// Get a migration by id. The id is what a retired-directive FormMatch
    /// carries in `matched_macro` (the exact def parse selected — its
    /// synthetic parse-side macro's name), so this is also the pipeline's
    /// FormMatch -> migration lookup.
    pub fn get_migration(&self, id: &str) -> Option<&MigrationDefAst> {
        let date = self.migration_ids.get(id)?;
        self.migrations.get(date)?.iter().find(|m| m.id == id)
    }

    /// The newest wave date the registry knows about, or `None` when no
    /// migration is registered at all.
    ///
    /// This is the DEFAULT `@version` for a file that declares none: an absent
    /// version means CURRENT, so new code is held to the current surface
    /// instead of sitting in a migration window that never closes. Derived
    /// from the registered capsules, so shipping a new wave moves the default
    /// with it — there is no date to keep in sync in Rust.
    pub fn newest_migration_wave(&self) -> Option<&str> {
        // `migrations` is a BTreeMap keyed by ISO date, so the last key is the
        // newest wave lexicographically == chronologically.
        self.migrations.keys().next_back().map(|s| s.as_str())
    }

    /// Iterate wave dates in chronological order.
    pub fn migration_wave_dates(&self) -> impl Iterator<Item = &str> {
        self.migrations.keys().map(|s| s.as_str())
    }

    /// All migrations in wave order (date ascending).
    pub fn all_migrations(&self) -> impl Iterator<Item = &MigrationDefAst> {
        self.migrations.values().flat_map(|v| v.iter())
    }

    /// Migrations participating for a project at the given `@version`
    /// (waves STRICTLY newer than the version; `None` = date-zero, every
    /// wave participates — the bootstrap case). Returns wave-ordered refs.
    pub fn migrations_pending_since(&self, version: Option<&str>) -> Vec<&MigrationDefAst> {
        self.migrations
            .iter()
            .filter(|(date, _)| version.is_none_or(|v| date.as_str() > v))
            .flat_map(|(_, migs)| migs.iter())
            .collect()
    }

    /// Register a comment type (PLAN-123).
    ///
    /// Unlike capture types and vendors — which stdlib may overwrite silently
    /// — a duplicate comment type across FILES is an ERROR. Overwriting would
    /// make `//@id` mean different things depending on load order, and the
    /// whole point of the roster is that a type id has ONE meaning everywhere
    /// (pill, check, MCP). A project wanting to change a stdlib type declares
    /// a new id.
    ///
    /// A same-`source_file` re-registration is treated as a RELOAD (the
    /// incremental cache re-reads a changed file after `unregister_file`), so
    /// it replaces rather than errors. NB this also means two declarations of
    /// one id WITHIN a single file silently keep the last — caught instead by
    /// the post-load roster check, which sees the final state and can report
    /// with the whole registry in view.
    pub fn register_comment_type(
        &mut self,
        def: CommentTypeDefAst,
    ) -> Result<(), MetaRegistryError> {
        if let Some(existing) = self.comment_types.get(&def.id) {
            // Re-registering the SAME file is a reload (incremental cache),
            // not a conflict.
            if existing.source_file != def.source_file {
                return Err(MetaRegistryError {
                    kind: MetaRegistryErrorKind::DuplicateCommentType(def.id.clone()),
                    span: def.span,
                });
            }
        }
        self.comment_types.insert(def.id.clone(), def);
        Ok(())
    }

    /// One comment type by id — the lookup behind validating a `//@<id>`
    /// header and rendering its chip.
    pub fn comment_type(&self, id: &str) -> Option<&CommentTypeDefAst> {
        self.comment_types.get(id)
    }

    /// THE roster: every declared comment type, id order. This is the single
    /// query the pill's picker, `check`'s section, and the MCP `types` tool
    /// all read — so "declare a type, it appears everywhere" is structural,
    /// not three implementations agreeing by luck.
    pub fn comment_types(&self) -> impl Iterator<Item = &CommentTypeDefAst> {
        self.comment_types.values()
    }

    /// Register a scalar type definition.
    ///
    /// A scalar's id must resolve to exactly ONE declaration — the whole
    /// point of the table is that every consumer agrees on what a scalar
    /// MEANS, so a redefined scalar is an error, never a silent overwrite.
    /// Re-registering the SAME file is a reload (incremental cache), not a
    /// conflict.
    pub fn register_scalar_type(
        &mut self,
        def: ScalarTypeDefAst,
    ) -> Result<(), MetaRegistryError> {
        if let Some(existing) = self.scalar_types.get(&def.id) {
            if existing.source_file != def.source_file {
                return Err(MetaRegistryError {
                    kind: MetaRegistryErrorKind::DuplicateScalarType(def.id.clone()),
                    span: def.span,
                });
            }
        }
        self.scalar_types.insert(def.id.clone(), def);
        Ok(())
    }

    /// One scalar type by id — the lookup behind type recognition, schema,
    /// zero, and widget consumers.
    pub fn get_scalar_type(&self, id: &str) -> Option<&ScalarTypeDefAst> {
        self.scalar_types.get(id)
    }

    /// THE scalar roster: every declared scalar type, id order. This is the
    /// single query every consumer reads — so "declare a scalar, it works
    /// everywhere" is structural, not N implementations agreeing by luck.
    pub fn scalar_types(&self) -> impl Iterator<Item = &ScalarTypeDefAst> {
        self.scalar_types.values()
    }

    /// Register a preset definition
    pub fn register_preset(&mut self, def: MetaPresetDefAst) -> Result<(), MetaRegistryError> {
        let key = format!("{}:{}", def.category, def.name);
        // Presets can be overwritten without error
        self.presets.insert(key, def);
        Ok(())
    }

    /// Register a runtime registry definition
    pub fn register_runtime_registry(
        &mut self,
        def: RuntimeRegistryDef,
    ) -> Result<(), MetaRegistryError> {
        let name = def.name.clone();
        self.runtime_registries.insert(name, def);
        Ok(())
    }

    /// Register a custom capture type definition
    pub fn register_capture_type(
        &mut self,
        def: CaptureTypeDefAst,
    ) -> Result<(), MetaRegistryError> {
        // Capture types can be overwritten without error (allows stdlib to be overridden)
        self.capture_types.insert(def.name.clone(), def);
        Ok(())
    }

    /// Register a vendored dependency definition
    pub fn register_vendor(&mut self, def: VendorDefAst) -> Result<(), MetaRegistryError> {
        // Vendors can be overwritten without error (allows stdlib to be overridden)
        self.vendors.insert(def.name.clone(), def);
        Ok(())
    }

    /// Register a MetaDef (primitive, macro, preset, runtime registry, or capture type)
    pub fn register(&mut self, def: MetaDef) -> Result<(), MetaRegistryError> {
        match def {
            MetaDef::Primitive(p) => self.register_primitive(p),
            MetaDef::Macro(m) => self.register_macro(m),
            MetaDef::Preset(p) => self.register_preset(p),
            MetaDef::RuntimeRegistry(r) => self.register_runtime_registry(r),
            MetaDef::CaptureType(ct) => self.register_capture_type(ct),
            MetaDef::Vendor(v) => self.register_vendor(v),
            MetaDef::Migration(m) => self.register_migration(m),
            MetaDef::CommentType(c) => self.register_comment_type(c),
            MetaDef::ScalarType(s) => self.register_scalar_type(s),
        }
    }

    /// Remove all definitions that were loaded from the given source file.
    ///
    /// Used by incremental stdlib cache to invalidate defs from a changed/deleted file
    /// before re-parsing it. Defs with `source_file: None` are never removed.
    pub fn unregister_file(&mut self, source_file: &str) {
        self.primitives
            .retain(|_, d| d.source_file.as_deref() != Some(source_file));
        self.macros
            .retain(|_, d| d.source_file.as_deref() != Some(source_file));
        self.capture_types
            .retain(|_, d| d.source_file.as_deref() != Some(source_file));
        self.presets
            .retain(|_, d| d.source_file.as_deref() != Some(source_file));
        self.runtime_registries
            .retain(|_, d| d.source_file.as_deref() != Some(source_file));
        self.vendors
            .retain(|_, d| d.source_file.as_deref() != Some(source_file));
        self.migrations
            .values_mut()
            .for_each(|v| v.retain(|d| d.source_file.as_deref() != Some(source_file)));
        self.comment_types
            .retain(|_, d| d.source_file.as_deref() != Some(source_file));
        self.scalar_types
            .retain(|_, d| d.source_file.as_deref() != Some(source_file));
        self.migrations.retain(|_, v| !v.is_empty());
        self.migration_ids.retain(|id, date| {
            self.migrations
                .get(date)
                .is_some_and(|v| v.iter().any(|d| &d.id == id))
        });
        // Retired-macro registrations die with their migration (the macros
        // themselves were already retained out of `self.macros` above by
        // source_file).
        self.macro_to_migration
            .retain(|_, (mig_id, _)| self.migration_ids.contains_key(mig_id));
    }

    /// Get a primitive by name
    pub fn get_primitive(&self, name: &str) -> Option<&PrimitiveDefAst> {
        self.primitives.get(name)
    }

    /// Get a macro by name
    pub fn get_macro(&self, name: &str) -> Option<&MacroDefAst> {
        self.macros.get(name)
    }

    /// Get a capture type by name
    pub fn get_capture_type(&self, name: &str) -> Option<&CaptureTypeDefAst> {
        self.capture_types.get(name)
    }

    /// Get a vendored dependency by name
    pub fn get_vendor(&self, name: &str) -> Option<&VendorDefAst> {
        self.vendors.get(name)
    }

    /// Iterate over all registered vendored dependencies
    pub fn vendors(&self) -> impl Iterator<Item = &VendorDefAst> {
        self.vendors.values()
    }

    /// Iterate over all registered macros
    pub fn iter_macros(&self) -> impl Iterator<Item = (&str, &MacroDefAst)> {
        self.macros.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Rebuild the provider index from all registered macros.
    /// This scans %binds outputs to discover which macros provide which bindings.
    /// Call this after loading stdlib macros.
    pub fn rebuild_provider_index(&mut self) {
        self.provider_index.clear();

        for (macro_name, macro_def) in &self.macros {
            for bind_decl in &macro_def.binds {
                for output in &bind_decl.outputs {
                    // Strip $ prefix if present
                    let binding_name = output.name.trim_start_matches('$').to_string();
                    self.provider_index
                        .entry(binding_name)
                        .or_default()
                        .push(macro_name.clone());
                }
            }
        }
    }

    /// Get macros that provide a binding (from the provider index).
    /// Returns empty vec if no providers found.
    pub fn get_providers(&self, binding: &str) -> Vec<&str> {
        let clean = binding.trim_start_matches('$');
        self.provider_index
            .get(clean)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Check if a primitive exists
    pub fn has_primitive(&self, name: &str) -> bool {
        self.primitives.contains_key(name)
    }

    /// Check if a macro exists (by macro name)
    pub fn has_macro(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /// Check if a macro exists that handles the given directive (via %form)
    pub fn has_macro_for_directive(&self, directive_name: &str) -> bool {
        self.get_macro_by_form_directive(directive_name).is_some()
    }

    /// Find a macro by its %form directive name (e.g., "on" or "@on" finds macro with %form { @on ... })
    /// This is used when resolving macro invocations where the form directive name
    /// differs from the internal macro definition name.
    /// Handles both prefixed ("@on") and unprefixed ("on") lookups.
    pub fn get_macro_by_form_directive(&self, directive_name: &str) -> Option<&MacroDefAst> {
        // Try exact match first
        if let Some(m) = self.macros.values().find(|m| {
            m.form
                .as_ref()
                .is_some_and(|f| f.directive_name == directive_name)
        }) {
            return Some(m);
        }

        // Try with @ prefix if not already prefixed
        if !directive_name.starts_with('@') {
            let prefixed = format!("@{}", directive_name);
            return self.macros.values().find(|m| {
                m.form
                    .as_ref()
                    .is_some_and(|f| f.directive_name == prefixed)
            });
        }

        None
    }

    /// Find ALL macros that match a form directive name
    /// Used for macro overloading (multiple patterns for the same directive)
    /// Handles both prefixed ("@on") and unprefixed ("on") lookups.
    pub fn get_all_macros_by_form_directive(&self, directive_name: &str) -> Vec<&MacroDefAst> {
        let prefixed = if directive_name.starts_with('@') {
            directive_name.to_string()
        } else {
            format!("@{}", directive_name)
        };

        self.macros
            .values()
            .filter(|m| {
                m.form.as_ref().is_some_and(|f| {
                    f.directive_name == directive_name || f.directive_name == prefixed
                })
            })
            .collect()
    }

    /// Resolve the macro for a `FormMatch` WITHOUT re-scoring (BUG-371).
    ///
    /// `preferred` is `FormMatch::matched_macro` — the exact `%macro` the parse layer
    /// selected. Parse sees the literal words that separate sibling forms (`@on … {…}`
    /// vs `@on … as $x : …`); the capture scorer does not, so re-deriving from the
    /// directive alone can tie, and the tie is broken by `HashMap` iteration order —
    /// i.e. differently on each process. Read-only consumers (inspect, docs, tooling)
    /// **MUST** come through here rather than `get_macro(directive)`, so what they
    /// report is what the compiler actually expanded.
    pub fn get_macro_for_match(
        &self,
        preferred: Option<&str>,
        directive_name: &str,
    ) -> Option<&MacroDefAst> {
        if let Some(m) = preferred.and_then(|n| self.get_macro(n)) {
            return Some(m);
        }
        self.get_macro(directive_name)
            .or_else(|| self.get_macro_by_form_directive(directive_name))
    }

    /// Find the best macro definition for a directive name, with scoring.
    ///
    /// 1. Try direct name lookup via `get_macro(name)`.
    /// 2. If not found, get all macros registered under the `@name` form directive.
    /// 3. Filter by scope compatibility.
    /// 4. Score each candidate against the captures and return the best match.
    pub fn find_macro_by_directive(
        &self,
        directive_name: &str,
        captures: &HashMap<String, CapturedValue>,
        selector: &Option<String>,
    ) -> Option<&MacroDefAst> {
        self.find_macro_for_match(None, directive_name, captures, selector)
    }

    /// Like `find_macro_by_directive`, but HONORS `preferred` — the exact `%macro`
    /// name that the parse layer already selected (specificity-ranked, literal-aware).
    /// When present and scope-compatible, it wins outright: parse disambiguated kinds
    /// (e.g. `@data inline` vs `@data derive`) via literal words the capture scorer
    /// cannot see, so re-guessing here would be lossy. Falls back to capture scoring
    /// when `preferred` is absent or unusable.
    pub fn find_macro_for_match(
        &self,
        preferred: Option<&str>,
        directive_name: &str,
        captures: &HashMap<String, CapturedValue>,
        selector: &Option<String>,
    ) -> Option<&MacroDefAst> {
        if let Some(name) = preferred
            && let Some(m) = self.get_macro(name)
            && macro_scope_matches(&m.scopes, selector)
        {
            return Some(m);
        }
        let input = CaptureMapInput { captures };

        // Fast path: a macro literally NAMED `directive_name` (prefix-based patterns,
        // direct invocations). Only honour it when its form actually SCORES against the
        // captures — otherwise a macro whose *name* collides with another macro's form
        // *directive* would wrongly win (e.g. scene `%macro light` named "light" vs
        // responsive `%macro light-mode` whose form is `@light { ... }`). When the named
        // macro is incompatible with the captures, fall through to capture-based scoring.
        // (BUG-037)
        if let Some(direct) = self.get_macro(directive_name)
            && macro_scope_matches(&direct.scopes, selector)
            && score_macro_form(direct, &input).is_some()
        {
            return Some(direct);
        }

        let matching = self.get_all_macros_by_form_directive(directive_name);
        if matching.is_empty() {
            // Back-compat last resort: the directly-named macro even if it did not score.
            return self.get_macro(directive_name);
        }
        matching
            .into_iter()
            .filter(|m| macro_scope_matches(&m.scopes, selector))
            .filter_map(|m| score_macro_form(m, &input).map(|s| (m, s)))
            .max_by_key(|(_, score)| *score)
            .map(|(m, _)| m)
            .or_else(|| self.get_macro(directive_name))
    }

    /// Detect an UNRESOLVED dispatch tie for a call (FEAT-088).
    ///
    /// Returns `Some((winner, rival, score))` when, after scope filtering, the two
    /// best-scoring candidate macros for `directive_name` share the SAME top score
    /// — i.e. the capture/literal scorer cannot tell them apart and
    /// `find_macro_for_match` would silently pick the first by iteration order.
    /// `None` when there is a clear winner (margin ≥ 1), fewer than two viable
    /// candidates, or the directive is handled by the direct-name fast path (which
    /// is unambiguous by construction). Names are returned winner-first in the same
    /// stable order `max_by_key` would choose.
    pub fn dispatch_ambiguity(
        &self,
        directive_name: &str,
        captures: &HashMap<String, CapturedValue>,
        selector: &Option<String>,
    ) -> Option<(String, String, u32)> {
        // The direct-name fast path resolves unambiguously — skip those (BUG-037).
        if let Some(direct) = self.get_macro(directive_name)
            && macro_scope_matches(&direct.scopes, selector)
            && score_macro_form(direct, &CaptureMapInput { captures }).is_some()
        {
            return None;
        }
        let input = CaptureMapInput { captures };
        let mut scored: Vec<(&str, u32)> = self
            .get_all_macros_by_form_directive(directive_name)
            .into_iter()
            .filter(|m| macro_scope_matches(&m.scopes, selector))
            .filter_map(|m| score_macro_form(m, &input).map(|s| (m.name.as_str(), s)))
            .collect();
        // Stable sort by score desc; ties keep registry iteration order so the
        // "winner" matches max_by_key's first-max choice.
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        match scored.as_slice() {
            [(win, ws), (rival, rs), ..] if ws == rs => {
                Some((win.to_string(), rival.to_string(), *ws))
            }
            _ => None,
        }
    }

    /// Get all primitive names
    pub fn primitive_names(&self) -> impl Iterator<Item = &str> {
        self.primitives.keys().map(|s| s.as_str())
    }

    /// Get all macro names
    pub fn macro_names(&self) -> impl Iterator<Item = &str> {
        self.macros.keys().map(|s| s.as_str())
    }

    /// Get all capture-type names (FEAT-118: enumerated by the Spell profile
    /// exporter to surface `%capture_type` declarations).
    pub fn capture_type_names(&self) -> impl Iterator<Item = &str> {
        self.capture_types.keys().map(|s| s.as_str())
    }

    /// Get number of primitives
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// Get number of macros
    pub fn macro_count(&self) -> usize {
        self.macros.len()
    }

    /// Get the %registers category for a macro (e.g., "type", "binding", "fn").
    ///
    /// Returns the category name from the macro's `%registers` clause, if any.
    pub fn registers_category(&self, macro_name: &str) -> Option<&str> {
        self.macros
            .get(macro_name)
            .and_then(|m| m.registers.as_ref())
            .map(|r| r.name.as_str())
    }

    /// Every macro that registers into `category`, with its declared record.
    ///
    /// This is the CONSUMPTION half of `%registers` (FUP-153). Until it existed,
    /// a `%registers` record was compiler-VISIBLE but compiler-INERT: the only
    /// read-back was [`registers_category`], a NAME lookup answering "which
    /// category does THIS macro register?" — never "what is registered IN this
    /// category?". So every consumer was hardcoded in Rust, and `@preset`, the
    /// closest analogue to a projection registry, resolved through a closed enum
    /// `PresetType { Easing, Scroll, Animation, Load }` that nothing enforced.
    ///
    /// With this, a category's variants are DATA: declaring a new one in `.st`
    /// makes it visible to its consumer with zero Rust changes — the
    /// kinds-as-data rule AGENTS.md states, and the same shape `scope_within`
    /// already uses for construct names.
    ///
    /// Returns `(macro_name, clause)` pairs, sorted by macro name so callers get
    /// a deterministic order regardless of load sequence.
    pub fn entries_of(
        &self,
        category: &str,
    ) -> Vec<(&str, &crate::parser::meta_ast::RegistersClause)> {
        let mut out: Vec<_> = self
            .macros
            .iter()
            .filter_map(|(name, m)| {
                m.registers
                    .as_ref()
                    .filter(|r| r.name == category)
                    .map(|r| (name.as_str(), r))
            })
            .collect();
        out.sort_by_key(|(name, _)| *name);
        out
    }

    /// The KEY a `%registers` clause registers under.
    ///
    /// `%registers driver(visible) { … }` — `driver` is the CATEGORY (the
    /// clause's `name`, what [`entries_of`] filters on) and `visible` is the
    /// KEY. Without this a caller can enumerate a category but not tell its
    /// entries apart, which is a list rather than a registry.
    ///
    /// A POSITIONAL arg carrying a `$capture` (`%registers preset($category,
    /// $name)`) has no compile-time key — it is filled per invocation — so this
    /// returns the literal only. Both shapes are real: static keys are how a
    /// policy table is written, captured keys are how `@preset`/`@template`
    /// register per-invocation records.
    pub fn entry_key<'a>(
        &self,
        clause: &'a crate::parser::meta_ast::RegistersClause,
    ) -> Option<&'a str> {
        use crate::parser::meta_ast::RegistersArg;
        clause.args.first().and_then(|arg| match arg {
            // A `$capture` positional is filled PER INVOCATION, so it is not a
            // compile-time key — the contract this doc-comment states. Enforced
            // here rather than relied upon upstream: the clause parser retains
            // captured args (they are real data a consumer may want), so the
            // literal-only rule has to live at the read boundary.
            RegistersArg::Positional(k) if k.starts_with('$') => None,
            RegistersArg::Positional(k) => Some(k.as_str()),
            RegistersArg::Named { .. } => None,
        })
    }

    /// The declared value of `field` in a macro's `%registers` record.
    ///
    /// NB this returns whatever was DECLARED, including a `$capture`
    /// ([`RegisterValue::Var`]) whose value is only known per invocation. An
    /// earlier doc-comment here claimed "literals only", which the type does not
    /// enforce; callers wanting a static policy value must match on the variant
    /// (`Ident`/`String`/`Bool`) and treat `Var` as "not statically known".
    pub fn entry_field<'a>(
        &self,
        clause: &'a crate::parser::meta_ast::RegistersClause,
        field: &str,
    ) -> Option<&'a crate::parser::meta_ast::RegisterValue> {
        use crate::parser::meta_ast::RegisterItem;
        clause.items.iter().find_map(|item| match item {
            RegisterItem::Field(f) if f.name == field => Some(&f.value),
            _ => None,
        })
    }

    /// The retained MODULE.st manifest for a namespace, if any (FEAT-118 M2).
    /// Self-hosted: this returns the `.st`-declared policy verbatim; callers
    /// read it rather than re-deriving rules in Rust.
    pub fn module_manifest(&self, namespace: &str) -> Option<&crate::parser::ast::ModuleManifest> {
        self.module_manifests.get(namespace)
    }

    /// Register a module manifest under `namespace` (its [`Namespace::key`]).
    /// The recursive stdlib loaders call this when a `MODULE.st` binds a folder;
    /// it is also the programmatic entry point for callers (and tests) that
    /// build a manifest directly. Last write wins.
    pub fn register_module_manifest(
        &mut self,
        namespace: impl Into<String>,
        manifest: crate::parser::ast::ModuleManifest,
    ) {
        self.module_manifests.insert(namespace.into(), manifest);
    }

    /// Whether `name` is PUBLIC in `namespace` (FEAT-118 M2 `%public`).
    ///
    /// Self-hosted visibility from the `.st` manifest: a module with a
    /// `%public (a, b)` list exposes ONLY those names; a module with NO
    /// `%public` clause is public-by-default (Odin floor) — everything visible;
    /// a namespace with no manifest at all is also fully public (global defs).
    ///
    /// PLAN-117 W5 — the boundary between the TWO visibility clauses, which are
    /// deliberately not unified:
    ///
    /// | clause | sigil | scope | governs |
    /// |---|---|---|---|
    /// | `%public (a, b)` | `%` | a FOLDER's `MODULE.st` | `@`/`%` vocabulary |
    /// | `@exports { $x }` | `@` | a FILE | `$` cells |
    ///
    /// They are different LAYERS, and the sigil says so: `%` marks the
    /// compile-time metasystem declaring which grammar a module hands out; `@`
    /// marks a directive declaring which runtime cells a file publishes. One
    /// sigil, one meaning — collapsing them would be the overload, not the
    /// cleanup. (An earlier plan revision proposed deleting `%public` as a
    /// parallel mechanism; it is not one, and it is enforced right here.)
    pub fn is_public(&self, namespace: &str, name: &str) -> bool {
        match self.module_manifests.get(namespace) {
            Some(m) if !m.public.is_empty() => m.public.iter().any(|n| n == name),
            _ => true,
        }
    }

    /// The `%reexport` declarations for a namespace (FEAT-118 M2 façades).
    pub fn module_reexports(&self, namespace: &str) -> &[crate::parser::ast::ReexportDecl] {
        self.module_manifests
            .get(namespace)
            .map(|m| m.reexports.as_slice())
            .unwrap_or(&[])
    }

    /// The `%using` active-extension hook for a namespace, if any (FEAT-118 M3).
    pub fn using_hook(&self, namespace: &str) -> Option<&crate::parser::ast::UsingHook> {
        self.module_manifests
            .get(namespace)
            .and_then(|m| m.using.as_ref())
    }

    /// Get the %scope declarations for a macro.
    pub fn macro_scopes(&self, macro_name: &str) -> &[MacroScope] {
        self.macros
            .get(macro_name)
            .map(|m| m.scopes.as_slice())
            .unwrap_or(&[])
    }

    /// The `%scope element(<tag>)` declarations for a macro (GH-12). Empty when
    /// the macro imposes no element restriction. Kinds-as-data, the same shape
    /// as `scope_within` — the pipeline reads these and refuses a bound selector
    /// whose implied element cannot be the required tag.
    pub fn macro_scope_elements(&self, macro_name: &str) -> &[String] {
        self.macros
            .get(macro_name)
            .map(|m| m.scope_element.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a macro registers a given category.
    ///
    /// Returns true if the macro has a `%registers` clause whose category name
    /// matches the given string. Used to replace hardcoded `is_type_def()`,
    /// `is_data_def()`, etc. checks.
    pub fn macro_registers(&self, macro_name: &str, category: &str) -> bool {
        self.registers_category(macro_name)
            .is_some_and(|c| c == category)
    }

    /// Get the %order value for a macro (None if unset).
    pub fn macro_order(&self, macro_name: &str) -> Option<u32> {
        self.macros.get(macro_name).and_then(|m| m.order)
    }

    /// Get a runtime registry by name
    pub fn get_runtime_registry(&self, name: &str) -> Option<&RuntimeRegistryDef> {
        self.runtime_registries.get(name)
    }

    /// Get the namespace for a registry on the current target
    pub fn get_registry_namespace(&self, name: &str) -> Option<&str> {
        let def = self.runtime_registries.get(name)?;
        let target = def
            .targets
            .iter()
            .find(|t| t.target_name == self.current_target)?;
        Some(&target.namespace)
    }

    /// Get the target definition for a registry
    pub fn get_registry_target(&self, name: &str) -> Option<&RegistryTarget> {
        let def = self.runtime_registries.get(name)?;
        def.targets
            .iter()
            .find(|t| t.target_name == self.current_target)
    }

    /// Load definitions from parsed meta_defs
    pub fn load_from_defs(&mut self, defs: Vec<MetaDef>) -> Result<(), MetaRegistryError> {
        for def in defs {
            self.register(def)?;
        }
        Ok(())
    }

    /// Like [`load_from_defs`], but stamps each def's `source_file` with the
    /// given (possibly virtual) path. The EMBEDDED stdlib loader
    /// (`load_embedded_stdlib`) previously dropped provenance because it routed
    /// through `load_from_defs` (no path), so embedded defs had `source_file:
    /// None` while filesystem-loaded defs carried their path. FEAT-118 needs
    /// provenance for namespace derivation (and `unregister_file` already
    /// relies on it for incremental reload), so embedded defs must carry their
    /// virtual path too. (AGENTS: fix the loading mechanism, don't special-case.)
    pub fn load_from_defs_with_source(
        &mut self,
        defs: Vec<MetaDef>,
        source_file: &str,
    ) -> Result<(), MetaRegistryError> {
        for mut def in defs {
            match &mut def {
                MetaDef::Macro(m) => m.source_file = Some(source_file.to_string()),
                MetaDef::Primitive(p) => p.source_file = Some(source_file.to_string()),
                MetaDef::CaptureType(ct) => ct.source_file = Some(source_file.to_string()),
                MetaDef::Preset(p) => p.source_file = Some(source_file.to_string()),
                MetaDef::RuntimeRegistry(r) => r.source_file = Some(source_file.to_string()),
                MetaDef::Vendor(v) => v.source_file = Some(source_file.to_string()),
                MetaDef::Migration(m) => m.source_file = Some(source_file.to_string()),
                MetaDef::CommentType(c) => c.source_file = Some(source_file.to_string()),
                MetaDef::ScalarType(s) => s.source_file = Some(source_file.to_string()),
            }
            self.register(def)?;
        }
        Ok(())
    }

    /// Load stdlib primitives and macros from a directory (recursive)
    pub fn load_stdlib_from_dir(&mut self, dir: &Path) -> Result<(), MetaRegistryError> {
        self.load_dir_recursive(dir)
    }

    /// Load stdlib primitives and macros, collecting all errors instead of failing early.
    ///
    /// Unlike `load_stdlib_from_dir`, this method continues loading even when files fail
    /// and returns all errors at once so they can be displayed together.
    pub fn load_stdlib_collecting_errors(&mut self, dir: &Path) -> StdlibLoadResult {
        let mut result = StdlibLoadResult::default();
        self.load_dir_recursive_collecting(&dir.to_path_buf(), &mut result);
        result
    }

    /// Internal recursive loader that collects errors
    fn load_dir_recursive_collecting(&mut self, dir: &PathBuf, result: &mut StdlibLoadResult) {
        let mut entries: Vec<_> = match std::fs::read_dir(dir) {
            Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
            Err(e) => {
                result.errors.push(StdlibLoadError {
                    file_path: dir.clone(),
                    span: None,
                    message: format!("Cannot read directory: {}", e),
                    kind: StdlibLoadErrorKind::FileNotFound,
                });
                return;
            }
        };
        // Sort for deterministic loading order across all platforms/filesystems.
        // Non-deterministic read_dir order causes HashMap seeding differences
        // that propagate into generated JS, making compiler output non-reproducible.
        entries.sort_by_key(|e| e.path());

        // FEAT-118 M2: if this directory carries a `MODULE.st`, its `%module`
        // clause names the namespace every sibling def registers under (folder =
        // module). The manifest is RETAINED (module_manifests) so its `%public`
        // / `%reexport` / `%using` policy can be read back for self-hosted
        // enforcement. `None` keeps defs global (the default everywhere today).
        let dir_namespace = match Self::module_manifest_for_dir(dir) {
            Some(manifest) => {
                let ns = Self::namespace_of_manifest(&manifest);
                if let Some(name) = manifest.name.clone() {
                    self.register_module_manifest(name, manifest);
                }
                ns
            }
            None => None,
        };

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                // Skip examples directories
                if path.file_name().is_some_and(|name| name == "examples") {
                    continue;
                }
                // Recurse into subdirectories
                self.load_dir_recursive_collecting(&path, result);
            } else if path.extension().is_some_and(|ext| ext == "st") {
                // Skip test files
                let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if file_name.ends_with(".test") {
                    continue;
                }
                // Load file, collecting errors
                match self.load_file_for_stdlib_in(&path, dir_namespace.as_ref()) {
                    Ok(()) => result.loaded_count += 1,
                    Err(e) => result.errors.push(e),
                }
            }
        }
    }

    /// Read a directory's `MODULE.st` manifest (if any), FEAT-118 M2.
    ///
    /// Returns the parsed `ModuleManifest` whenever a `MODULE.st` with a
    /// `%module <name>` clause is present — carrying the `.st`-declared `%public`
    /// / `%reexport` / `%using` policy so the registry can RETAIN it for
    /// self-hosted enforcement. `None` when there is no `MODULE.st` or no
    /// `%module` clause (defs stay global, preserving today's behaviour).
    fn module_manifest_for_dir(dir: &Path) -> Option<crate::parser::ast::ModuleManifest> {
        let module_st = dir.join("MODULE.st");
        if !module_st.is_file() {
            return None;
        }
        let content = std::fs::read_to_string(&module_st).ok()?;
        let parsed = crate::parser::parse(&content).ok()?;
        let manifest = parsed.module_manifest?;
        // A manifest only binds a namespace when it names one.
        manifest.name.as_ref()?;
        Some(manifest)
    }

    /// The namespace a manifest assigns (its `%module <name>` as a single-segment
    /// path). `None` for a nameless manifest.
    fn namespace_of_manifest(
        manifest: &crate::parser::ast::ModuleManifest,
    ) -> Option<crate::metasystem::module::Namespace> {
        manifest
            .name
            .as_ref()
            .map(|n| crate::metasystem::module::Namespace::new(None, vec![n.clone()]))
    }

    /// Load a single file, returning StdlibLoadError on failure.
    pub fn load_file_for_stdlib(&mut self, path: &Path) -> Result<(), StdlibLoadError> {
        self.load_file_for_stdlib_in(path, None)
    }

    /// Load a single file, stamping its macros with `namespace` when the file
    /// belongs to a `MODULE.st`-named folder (FEAT-118 M2). `None` = global,
    /// identical to the historical `load_file_for_stdlib`.
    pub fn load_file_for_stdlib_in(
        &mut self,
        path: &Path,
        namespace: Option<&crate::metasystem::module::Namespace>,
    ) -> Result<(), StdlibLoadError> {
        let content = std::fs::read_to_string(path).map_err(|e| StdlibLoadError {
            file_path: path.to_path_buf(),
            span: None,
            message: format!("Cannot read file: {}", e),
            kind: StdlibLoadErrorKind::FileNotFound,
        })?;

        let parsed = crate::parser::parse(&content).map_err(|e| {
            let first = e.first();
            StdlibLoadError {
                file_path: path.to_path_buf(),
                span: Some(SourceSpan::new(first.offset, first.offset + first.len)),
                message: e.to_string(),
                kind: StdlibLoadErrorKind::ParseError,
            }
        })?;

        let source_file = path.to_string_lossy().to_string();

        for mut def in parsed.meta_defs {
            match &mut def {
                MetaDef::Macro(m) => {
                    m.source_file = Some(source_file.clone());
                    // FEAT-118 M2: a MODULE.st-named folder binds its macros to
                    // its namespace (M0 Fqn keying then keys them ns/name).
                    if let Some(ns) = namespace {
                        m.module = Some(ns.clone());
                    }
                }
                MetaDef::Primitive(p) => {
                    p.source_file = Some(source_file.clone());
                }
                MetaDef::CaptureType(ct) => {
                    ct.source_file = Some(source_file.clone());
                }
                MetaDef::Preset(p) => {
                    p.source_file = Some(source_file.clone());
                }
                MetaDef::RuntimeRegistry(r) => {
                    r.source_file = Some(source_file.clone());
                }
                MetaDef::Vendor(v) => {
                    v.source_file = Some(source_file.clone());
                }
                MetaDef::Migration(m) => {
                    m.source_file = Some(source_file.clone());
                }
                MetaDef::CommentType(c) => {
                    c.source_file = Some(source_file.clone());
                }
                MetaDef::ScalarType(s) => {
                    s.source_file = Some(source_file.clone());
                }
            }
            self.register(def).map_err(|e| {
                let kind = match &e.kind {
                    MetaRegistryErrorKind::DuplicateMacro(name) => {
                        StdlibLoadErrorKind::DuplicateMacro(name.clone())
                    }
                    MetaRegistryErrorKind::DuplicatePrimitive(name) => {
                        StdlibLoadErrorKind::DuplicateMacro(name.clone())
                    }
                    _ => StdlibLoadErrorKind::ParseError,
                };
                StdlibLoadError {
                    file_path: path.to_path_buf(),
                    span: Some(e.span),
                    message: e.to_string(),
                    kind,
                }
            })?;
        }

        Ok(())
    }

    /// Load only .st files from the top level of a directory (non-recursive)
    pub fn load_dir_flat(&mut self, dir: &Path) -> Result<(), MetaRegistryError> {
        let entries = std::fs::read_dir(dir).map_err(|e| MetaRegistryError {
            kind: MetaRegistryErrorKind::IoError(e.to_string()),
            span: SourceSpan::default(),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| MetaRegistryError {
                kind: MetaRegistryErrorKind::IoError(e.to_string()),
                span: SourceSpan::default(),
            })?;

            let path = entry.path();
            // Only load files, skip subdirectories
            if path.is_file() && path.extension().is_some_and(|ext| ext == "st") {
                self.load_file(&path)?;
            }
        }

        Ok(())
    }

    fn load_dir_recursive(&mut self, dir: &Path) -> Result<(), MetaRegistryError> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| MetaRegistryError {
                kind: MetaRegistryErrorKind::IoError(e.to_string()),
                span: SourceSpan::default(),
            })?
            .filter_map(|e| e.ok())
            .collect();
        // Sort for deterministic loading order.
        entries.sort_by_key(|e| e.path());

        // FEAT-118 M2: folder = module — a MODULE.st `%module <name>` binds this
        // directory's macros to a namespace; the manifest is RETAINED for
        // self-hosted %public/%reexport/%using enforcement. `None` = global.
        let dir_namespace = match Self::module_manifest_for_dir(dir) {
            Some(manifest) => {
                let ns = Self::namespace_of_manifest(&manifest);
                if let Some(name) = manifest.name.clone() {
                    self.register_module_manifest(name, manifest);
                }
                ns
            }
            None => None,
        };

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                // Skip examples directories - they contain user-facing code, not metasystem defs
                if path.file_name().is_some_and(|name| name == "examples") {
                    continue;
                }
                let _ = self.load_dir_recursive(&path);
            } else if path.extension().is_some_and(|ext| ext == "st") {
                // Skip test files - they use macros, they don't define them
                let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if file_name.ends_with(".test") {
                    continue;
                }
                let _ = self.load_file_in(&path, dir_namespace.as_ref());
            }
        }

        Ok(())
    }

    fn load_file(&mut self, path: &Path) -> Result<(), MetaRegistryError> {
        self.load_file_in(path, None)
    }

    /// Like [`load_file`], stamping macros with `namespace` (FEAT-118 M2 folder
    /// binding). `None` = global, identical to the historical behaviour.
    fn load_file_in(
        &mut self,
        path: &Path,
        namespace: Option<&crate::metasystem::module::Namespace>,
    ) -> Result<(), MetaRegistryError> {
        let content = std::fs::read_to_string(path).map_err(|e| MetaRegistryError {
            kind: MetaRegistryErrorKind::IoError(format!("{}: {}", path.display(), e)),
            span: SourceSpan::default(),
        })?;

        let parsed = crate::parser::parse(&content).map_err(|e| MetaRegistryError {
            kind: MetaRegistryErrorKind::ParseError(format!("{}: {}", path.display(), e)),
            span: SourceSpan::default(),
        })?;

        // Store path for error traces
        let source_file = path.to_string_lossy().to_string();

        for mut def in parsed.meta_defs {
            // Tag each macro/primitive with its source file
            match &mut def {
                MetaDef::Macro(m) => {
                    m.source_file = Some(source_file.clone());
                    if let Some(ns) = namespace {
                        m.module = Some(ns.clone());
                    }
                }
                MetaDef::Primitive(p) => {
                    p.source_file = Some(source_file.clone());
                }
                _ => {}
            }
            self.register(def)?;
        }

        Ok(())
    }
}

/// Returns true if the macro is allowed in the given scope context.
/// - `selector`: `None` means file-level, `Some(_)` means inside a selector body
/// - If the macro has no `%scope` declarations, it's allowed everywhere (backward compat)
pub fn macro_scope_matches(scopes: &[MacroScope], selector: &Option<String>) -> bool {
    if scopes.is_empty() {
        return true; // no scope restriction
    }
    match selector {
        None => scopes.contains(&MacroScope::File),
        Some(_) => scopes.contains(&MacroScope::Selector),
    }
}

/// `%scope within(<construct>)` containment query (PLAN-039). A macro with a
/// `within` restriction matches ONLY when the ENCLOSING SCOPE STACK contains a
/// region whose construct-reference name is one of the restriction names.
/// `stack` is the ordered list of construct-ref names of the scopes enclosing the
/// match position (innermost last), e.g. `["selector", "template", "each"]`.
///
/// Kinds-as-DATA: this is a set-membership query over registry-key strings — NO
/// Rust match-arm per scope kind. Empty `within` = no restriction (always matches).
/// This is the successor predicate to the `MacroScope` enum's File/Selector check.
// Wired into match-time scope filtering during the PLAN-039 component_body cutover (the
// keystone factory-input rewrite); landed early with its test (S1e) as the proven predicate.
#[allow(dead_code)]
pub fn macro_within_matches(within: &[String], stack: &[String]) -> bool {
    if within.is_empty() {
        return true; // no within-restriction
    }
    stack.iter().any(|frame| within.iter().any(|w| w == frame))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::meta_ast::*;

    fn make_test_primitive(name: &str) -> PrimitiveDefAst {
        PrimitiveDefAst {
            name: name.to_string(),
            params: vec![],
            body: PrimitiveBody::default(),
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        }
    }

    fn make_test_macro(name: &str) -> MacroDefAst {
        MacroDefAst {
            retired: None,
            name: name.to_string(),
            form: None,
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            requires: vec![],
            span: SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        }
    }
    

    #[test]
    fn test_register_primitive() {
        let mut registry = MetaRegistry::new();
        let prim = make_test_primitive("test");
        assert!(registry.register_primitive(prim).is_ok());
        assert!(registry.has_primitive("test"));
        assert!(!registry.has_primitive("nonexistent"));
    }

    #[test]
    fn test_register_macro() {
        let mut registry = MetaRegistry::new();
        let m = make_test_macro("test");
        assert!(registry.register_macro(m).is_ok());
        assert!(registry.has_macro("test"));
        assert!(!registry.has_macro("nonexistent"));
    }

    #[test]
    fn test_duplicate_primitive_error() {
        let mut registry = MetaRegistry::new();
        let prim1 = make_test_primitive("dup");
        let prim2 = make_test_primitive("dup");
        assert!(registry.register_primitive(prim1).is_ok());
        let err = registry.register_primitive(prim2).unwrap_err();
        assert!(matches!(
            err.kind,
            MetaRegistryErrorKind::DuplicatePrimitive(_)
        ));
    }

    #[test]
    fn test_duplicate_macro_error() {
        let mut registry = MetaRegistry::new();
        let m1 = make_test_macro("dup");
        let m2 = make_test_macro("dup");
        assert!(registry.register_macro(m1).is_ok());
        let err = registry.register_macro(m2).unwrap_err();
        assert!(matches!(err.kind, MetaRegistryErrorKind::DuplicateMacro(_)));
    }

    #[test]
    fn test_get_primitive() {
        let mut registry = MetaRegistry::new();
        let prim = make_test_primitive("scroll");
        registry.register_primitive(prim).unwrap();

        let retrieved = registry.get_primitive("scroll");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "scroll");
    }

    #[test]
    fn test_primitive_names() {
        let mut registry = MetaRegistry::new();
        registry
            .register_primitive(make_test_primitive("a"))
            .unwrap();
        registry
            .register_primitive(make_test_primitive("b"))
            .unwrap();
        registry
            .register_primitive(make_test_primitive("c"))
            .unwrap();

        let names: Vec<_> = registry.primitive_names().collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
    }

    #[test]
    fn test_get_macro_by_form_directive() {
        use crate::parser::meta_ast::FormClause;

        let mut registry = MetaRegistry::new();

        // Create a macro with internal name "on-event-dispatcher"
        // but form directive "@on" (matching what users write)
        let mut macro_def = make_test_macro("on-event-dispatcher");
        macro_def.form = Some(FormClause {
            directive_name: "on".to_string(),
            inline_elements: vec![],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: Vec::new(),
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        });

        registry.register_macro(macro_def).unwrap();

        // Lookup by internal name should work
        assert!(
            registry.get_macro("on-event-dispatcher").is_some(),
            "Should find macro by internal name"
        );

        // Lookup by directive name via get_macro should NOT work
        // (get_macro uses internal name)
        assert!(
            registry.get_macro("on").is_none(),
            "get_macro should not find by directive name"
        );

        // Lookup by form directive should work
        let found = registry.get_macro_by_form_directive("on");
        assert!(found.is_some(), "Should find macro by form directive");
        assert_eq!(
            found.unwrap().name,
            "on-event-dispatcher",
            "Found macro should have correct internal name"
        );

        // get_all_macros_by_form_directive should return it
        let all = registry.get_all_macros_by_form_directive("on");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "on-event-dispatcher");
    }

    #[test]
    fn test_get_macro_by_form_directive_multiple_overloads() {
        use crate::parser::meta_ast::FormClause;

        let mut registry = MetaRegistry::new();

        // Register multiple macros that handle the same @state directive
        // but with different patterns (overloading)
        let mut macro1 = make_test_macro("state-simple");
        macro1.form = Some(FormClause {
            directive_name: "state".to_string(),
            inline_elements: vec![],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: Vec::new(),
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        });

        let mut macro2 = make_test_macro("state-machine");
        macro2.form = Some(FormClause {
            directive_name: "state".to_string(),
            inline_elements: vec![],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: Vec::new(),
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        });

        registry.register_macro(macro1).unwrap();
        registry.register_macro(macro2).unwrap();

        // get_macro_by_form_directive returns the first match
        let found = registry.get_macro_by_form_directive("state");
        assert!(found.is_some());

        // get_all_macros_by_form_directive returns all matches
        let all = registry.get_all_macros_by_form_directive("state");
        assert_eq!(all.len(), 2, "Should find both @state macros");

        let names: Vec<_> = all.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"state-simple"));
        assert!(names.contains(&"state-machine"));
    }

    /// BUG-037: a macro literally NAMED `light` (3D scene light, form `@light $type(...)`)
    /// must NOT shadow a macro `light-mode` whose form directive is also `@light` but takes
    /// a brace body. `find_macro_by_directive` previously short-circuited on `get_macro(name)`
    /// and returned the scene macro regardless of captures. The fix gates that fast-path on
    /// form-scoring: the named macro only wins when its form actually scores against the
    /// captures. This test enshrines BOTH directions of the disambiguation.
    #[test]
    fn test_find_macro_by_directive_named_collision_bug037() {
        use crate::parser::meta_ast::{
            CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement, FormParam,
        };
        use std::collections::HashMap;

        let mut registry = MetaRegistry::new();

        // Scene light: internal name "light", form `@light $type:ident(...)` — REQUIRES `type`.
        let mut scene_light = make_test_macro("light");
        scene_light.form = Some(FormClause {
            directive_name: "light".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "type".to_string(),
                    capture_type: CaptureType::Ident,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: Vec::new(),
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        });

        // Responsive light-mode: internal name "light-mode", form `@light { $styles ... }`.
        let mut light_mode = make_test_macro("light-mode");
        light_mode.form = Some(FormClause {
            directive_name: "light".to_string(),
            inline_elements: vec![],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: Some("$styles".to_string()),
            body_params: vec![FormParam {
                name: "styles".to_string(),
                elements: vec![FormInlineElement::Capture(
                    FormCapture {
                        var_name: "styles".to_string(),
                        capture_type: CaptureType::Properties,
                        modifier: CaptureModifier::Required,
                        alias_capture: None,
                    },
                    None,
                )],
                default: None,
            }],
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        });

        registry.register_macro(scene_light).unwrap();
        registry.register_macro(light_mode).unwrap();

        // (1) `@light { ... }` — captures carry `styles`, NOT `type`. The scene macro's form
        //     requires `type` → it must NOT win; light-mode must be selected.
        let mut brace_caps: HashMap<String, CapturedValue> = HashMap::new();
        brace_caps.insert("styles".to_string(), CapturedValue::StyleProperties(vec![]));
        let found = registry.find_macro_by_directive("light", &brace_caps, &None);
        assert_eq!(
            found.map(|m| m.name.as_str()),
            Some("light-mode"),
            "BUG-037: `@light {{ }}` must resolve to light-mode, not the scene `light` macro"
        );

        // (2) `@light ambient` — captures carry `type`. The scene macro IS compatible and is
        //     the directly-named macro, so it must still win (no behavior change).
        let mut call_caps: HashMap<String, CapturedValue> = HashMap::new();
        call_caps.insert(
            "type".to_string(),
            CapturedValue::Ident("ambient".to_string()),
        );
        let found = registry.find_macro_by_directive("light", &call_caps, &None);
        assert_eq!(
            found.map(|m| m.name.as_str()),
            Some("light"),
            "scene `@light ambient` must still resolve to the scene `light` macro"
        );
    }

    #[test]
    fn test_transform_registry_add_and_get() {
        let mut registry = TransformRegistry::new();
        registry.register(
            "formatPrice",
            TransformRule {
                output_prefix: "SpacetimeFunctions.".to_string(),
                output_template: "SpacetimeFunctions.$name($args)".to_string(),
            },
        );

        assert!(registry.get("formatPrice").is_some());
        assert_eq!(
            registry.get("formatPrice").unwrap().output_prefix,
            "SpacetimeFunctions."
        );
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_runtime_registry_storage() {
        let mut registry = MetaRegistry::new();

        let def = RuntimeRegistryDef {
            name: "functions".to_string(),
            targets: vec![RegistryTarget {
                target_name: "js".to_string(),
                namespace: "ST.functions".to_string(),
                init: "ST.functions = ST.functions || {};".to_string(),
                operations: vec![],
                span: SourceSpan::default(),
            }],
            span: SourceSpan::default(),
            source_file: None,
        };

        registry.register_runtime_registry(def).unwrap();

        assert!(registry.get_runtime_registry("functions").is_some());
        assert_eq!(
            registry.get_registry_namespace("functions"),
            Some("ST.functions")
        );
    }

    #[test]
    fn test_capture_type_registration() {
        let mut registry = MetaRegistry::new();

        let def = CaptureTypeDefAst {
            name: "test_type".to_string(),
            pattern: CapturePatternAst::Capture {
                var_name: "x".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
            },
            span: SourceSpan::default(),
            source_file: None,
        };

        registry.register_capture_type(def).unwrap();

        assert!(registry.get_capture_type("test_type").is_some());
        assert!(registry.get_capture_type("nonexistent").is_none());
    }

    #[test]
    fn test_stdlib_loads_param_list_capture_type() {
        // Test that the stdlib capture-types directory loads param_list
        let capture_types_dir = std::path::Path::new("stdlib/capture-types");
        if !capture_types_dir.exists() {
            return; // Skip if directory doesn't exist (e.g., in CI without stdlib)
        }

        let mut registry = MetaRegistry::new();
        let result = registry.load_stdlib_from_dir(capture_types_dir);
        assert!(
            result.is_ok(),
            "Failed to load capture-types: {:?}",
            result.err()
        );

        // Verify param_list is registered
        let param_list = registry.get_capture_type("param_list");
        assert!(
            param_list.is_some(),
            "param_list capture type should be registered after loading stdlib/capture-types"
        );

        // Verify the pattern structure is correct (should be a group with zero-or-more)
        let param_list_def = param_list.unwrap();
        assert_eq!(param_list_def.name, "param_list");
        match &param_list_def.pattern {
            CapturePatternAst::Group { modifier, .. } => {
                assert_eq!(*modifier, Some(CaptureModifier::ZeroOrMore));
            }
            _ => panic!(
                "Expected Group pattern for param_list, got {:?}",
                param_list_def.pattern
            ),
        }
    }

    #[test]
    fn test_provider_index_from_binds() {
        use crate::parser::meta_ast::BindDecl;
        use crate::parser::meta_ast::BindOutput;

        let mut registry = MetaRegistry::new();

        // Create a macro with %binds that output $gl, $width, $height
        let mut macro_def = make_test_macro("scene");
        macro_def.binds = vec![BindDecl {
            primitive: "glContext".to_string(),
            args: vec![],
            outputs: vec![
                BindOutput {
                    name: "$gl".to_string(),
                    alias: None,
                },
                BindOutput {
                    name: "$width".to_string(),
                    alias: None,
                },
                BindOutput {
                    name: "$height".to_string(),
                    alias: None,
                },
            ],
            span: SourceSpan::default(),
        }];

        registry.register_macro(macro_def).unwrap();
        registry.rebuild_provider_index();

        // Should find scene as provider for gl, width, height
        assert_eq!(registry.get_providers("gl"), vec!["scene"]);
        assert_eq!(registry.get_providers("$gl"), vec!["scene"]); // handles $ prefix
        assert_eq!(registry.get_providers("width"), vec!["scene"]);
        assert_eq!(registry.get_providers("height"), vec!["scene"]);
        assert!(registry.get_providers("nonexistent").is_empty());
    }

    #[test]
    fn test_provider_index_multiple_macros() {
        use crate::parser::meta_ast::BindDecl;
        use crate::parser::meta_ast::BindOutput;

        let mut registry = MetaRegistry::new();

        // Create two macros that provide different bindings
        let mut scene_macro = make_test_macro("scene");
        scene_macro.binds = vec![BindDecl {
            primitive: "glContext".to_string(),
            args: vec![],
            outputs: vec![
                BindOutput {
                    name: "$gl".to_string(),
                    alias: None,
                },
                BindOutput {
                    name: "$t".to_string(),
                    alias: None,
                },
            ],
            span: SourceSpan::default(),
        }];

        let mut timer_macro = make_test_macro("timer");
        timer_macro.binds = vec![BindDecl {
            primitive: "ticker".to_string(),
            args: vec![],
            outputs: vec![
                BindOutput {
                    name: "$t".to_string(),
                    alias: None,
                }, // Also provides $t
                BindOutput {
                    name: "$elapsed".to_string(),
                    alias: None,
                },
            ],
            span: SourceSpan::default(),
        }];

        registry.register_macro(scene_macro).unwrap();
        registry.register_macro(timer_macro).unwrap();
        registry.rebuild_provider_index();

        // gl only from scene
        assert_eq!(registry.get_providers("gl"), vec!["scene"]);
        // elapsed only from timer
        assert_eq!(registry.get_providers("elapsed"), vec!["timer"]);
        // t from both (order may vary, so check contains)
        let t_providers = registry.get_providers("t");
        assert_eq!(t_providers.len(), 2);
        assert!(t_providers.contains(&"scene"));
        assert!(t_providers.contains(&"timer"));
    }

    #[test]
    fn test_iter_macros() {
        let mut registry = MetaRegistry::new();
        registry.register_macro(make_test_macro("foo")).unwrap();
        registry.register_macro(make_test_macro("bar")).unwrap();
        registry.register_macro(make_test_macro("baz")).unwrap();

        let names: Vec<_> = registry.iter_macros().map(|(name, _)| name).collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"bar"));
        assert!(names.contains(&"baz"));
    }

    #[test]
    fn test_provider_index_with_stdlib() {
        // Test that loading stdlib macros properly populates the provider index
        let macros_dir = std::path::Path::new("stdlib/macros");
        if !macros_dir.exists() {
            return; // Skip if stdlib not available
        }

        let mut registry = MetaRegistry::new();
        let _ = registry.load_stdlib_from_dir(macros_dir);
        registry.rebuild_provider_index();

        // The responsive media macros provide $matches via their %binds. (This used
        // to assert the now-deleted `@scene` macro provided $gl; the hand-rolled
        // WebGL engine was retired in PLAN-050 — 3D is the opt-in stdlib/3d module.)
        let matches_providers = registry.get_providers("matches");
        assert!(
            matches_providers.iter().any(|p| *p == "media-query"),
            "Expected 'media-query' to provide $matches, got: {:?}",
            matches_providers
        );
    }

    #[test]
    fn test_registers_category() {
        use crate::parser::meta_ast::RegistersClause;

        let mut registry = MetaRegistry::new();

        let mut type_macro = make_test_macro("type");
        type_macro.registers = Some(RegistersClause {
            name: "type".to_string(),
            args: vec![],
            items: vec![],
            span: SourceSpan::default(),
        });
        registry.register_macro(type_macro).unwrap();

        let mut data_macro = make_test_macro("data");
        data_macro.registers = Some(RegistersClause {
            name: "binding".to_string(),
            args: vec![],
            items: vec![],
            span: SourceSpan::default(),
        });
        registry.register_macro(data_macro).unwrap();

        let plain_macro = make_test_macro("plain");
        registry.register_macro(plain_macro).unwrap();

        assert_eq!(registry.registers_category("type"), Some("type"));
        assert_eq!(registry.registers_category("data"), Some("binding"));
        assert_eq!(registry.registers_category("plain"), None);
        assert_eq!(registry.registers_category("nonexistent"), None);
    }

    #[test]
    fn test_macro_registers() {
        use crate::parser::meta_ast::RegistersClause;

        let mut registry = MetaRegistry::new();

        let mut type_macro = make_test_macro("type");
        type_macro.registers = Some(RegistersClause {
            name: "type".to_string(),
            args: vec![],
            items: vec![],
            span: SourceSpan::default(),
        });
        registry.register_macro(type_macro).unwrap();

        assert!(registry.macro_registers("type", "type"));
        assert!(!registry.macro_registers("type", "binding"));
        assert!(!registry.macro_registers("nonexistent", "type"));
    }

    #[test]
    fn test_macro_order() {
        let mut registry = MetaRegistry::new();

        let mut ordered_macro = make_test_macro("type");
        ordered_macro.order = Some(50);
        registry.register_macro(ordered_macro).unwrap();

        let unordered_macro = make_test_macro("data");
        registry.register_macro(unordered_macro).unwrap();

        assert_eq!(registry.macro_order("type"), Some(50));
        assert_eq!(registry.macro_order("data"), None);
        assert_eq!(registry.macro_order("nonexistent"), None);
    }

    #[test]
    fn test_macro_scopes() {
        use crate::parser::meta_ast::MacroScope;

        let mut registry = MetaRegistry::new();

        let mut scoped_macro = make_test_macro("type");
        scoped_macro.scopes = vec![MacroScope::File];
        registry.register_macro(scoped_macro).unwrap();

        let mut multi_scope = make_test_macro("data");
        multi_scope.scopes = vec![MacroScope::File, MacroScope::Selector];
        registry.register_macro(multi_scope).unwrap();

        let no_scope = make_test_macro("plain");
        registry.register_macro(no_scope).unwrap();

        assert_eq!(registry.macro_scopes("type"), &[MacroScope::File]);
        assert_eq!(
            registry.macro_scopes("data"),
            &[MacroScope::File, MacroScope::Selector]
        );
        assert_eq!(registry.macro_scopes("plain"), &[] as &[MacroScope]);
        assert_eq!(registry.macro_scopes("nonexistent"), &[] as &[MacroScope]);
    }

    #[test]
    fn test_registers_category_with_stdlib() {
        // Test that loading stdlib macros properly populates %registers
        let macros_dir = std::path::Path::new("stdlib/macros");
        if !macros_dir.exists() {
            return; // Skip if stdlib not available
        }

        let mut registry = MetaRegistry::new();
        // Load capture types first (needed by some macros)
        let capture_types_dir = std::path::Path::new("stdlib/capture-types");
        if capture_types_dir.exists() {
            let _ = registry.load_stdlib_from_dir(capture_types_dir);
        }
        let _ = registry.load_stdlib_from_dir(macros_dir);

        // @type should register as "type"
        assert_eq!(
            registry.registers_category("type"),
            Some("type"),
            "Expected @type macro to have registers category 'type'"
        );

        // @template should register as "template"
        assert_eq!(
            registry.registers_category("template"),
            Some("template"),
            "Expected @template macro to have registers category 'template'"
        );
    }

    #[test]
    fn test_macro_within_matches_containment() {
        // No restriction → always matches.
        assert!(macro_within_matches(&[], &[]));
        assert!(macro_within_matches(&[], &["selector".to_string()]));
        // within(template): matches inside a template region, refused outside.
        let within = vec!["template".to_string()];
        assert!(
            macro_within_matches(&within, &["selector".to_string(), "template".to_string()]),
            "within(template) must match when the scope stack contains a template region"
        );
        assert!(
            !macro_within_matches(&within, &["selector".to_string()]),
            "within(template) must be REFUSED outside any template region"
        );
        assert!(
            !macro_within_matches(&within, &[]),
            "within(template) must be refused at file root"
        );
        // within(each|template): matches if EITHER kind is on the stack.
        let within2 = vec!["each".to_string(), "template".to_string()];
        assert!(macro_within_matches(&within2, &["each".to_string()]));
        assert!(macro_within_matches(&within2, &["template".to_string()]));
        assert!(!macro_within_matches(&within2, &["selector".to_string()]));
    }

    #[test]
    fn test_macro_scope_matches_empty_scopes_always_matches() {
        // No scope restriction → allowed everywhere (backward compat)
        assert!(macro_scope_matches(&[], &None));
        assert!(macro_scope_matches(&[], &Some(".foo".to_string())));
    }

    #[test]
    fn test_macro_scope_matches_file_scope() {
        let scopes = vec![MacroScope::File];
        assert!(
            macro_scope_matches(&scopes, &None),
            "File scope should match file-level (None)"
        );
        assert!(
            !macro_scope_matches(&scopes, &Some(".foo".to_string())),
            "File scope should not match inside selector"
        );
    }

    #[test]
    fn dispatch_ambiguity_detects_tie_and_clears_on_disambiguation() {
        // FEAT-088: two macros sharing directive `widget`, both `@widget $x:ident`,
        // score identically for a call with an ident `x` → unresolved tie.
        use crate::parser::meta_ast::{
            CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement,
        };
        use std::collections::HashMap;

        // `extra_required` adds a second REQUIRED capture the call won't supply, so
        // that form hard-rejects. (find_macro_for_match's scoring fallback uses
        // CaptureMapInput, where literals are not positionally verified — discrimination
        // in this path is by capture presence/types, so we break the tie with a capture.)
        fn widget_form(extra_required: bool) -> FormClause {
            let mut inline = vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "x".to_string(),
                    capture_type: CaptureType::Ident,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )];
            if extra_required {
                inline.push(FormInlineElement::Capture(
                    FormCapture {
                        var_name: "y".to_string(),
                        capture_type: CaptureType::Ident,
                        modifier: CaptureModifier::Required,
                        alias_capture: None,
                    },
                    None,
                ));
            }
            FormClause {
                directive_name: "widget".to_string(),
                inline_elements: inline,
                params: vec![],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: Vec::new(),
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            }
        }

        // (1) Ambiguous: both forms identical → tie.
        let mut reg = MetaRegistry::new();
        let mut a = make_test_macro("widget-a");
        a.form = Some(widget_form(false));
        let mut b = make_test_macro("widget-b");
        b.form = Some(widget_form(false));
        reg.register_macro(a).unwrap();
        reg.register_macro(b).unwrap();

        let mut caps: HashMap<String, CapturedValue> = HashMap::new();
        caps.insert("x".to_string(), CapturedValue::Ident("hello".to_string()));
        let amb = reg.dispatch_ambiguity("widget", &caps, &None);
        assert!(amb.is_some(), "identical forms must tie");
        let (w, r, _) = amb.unwrap();
        assert!(
            (w == "widget-a" && r == "widget-b") || (w == "widget-b" && r == "widget-a"),
            "both competing macros named, got {w}/{r}"
        );

        // (2) Mutation proof: give widget-b a second REQUIRED capture the call does
        //     NOT supply → widget-b hard-rejects (None), widget-a wins clean → no tie.
        let mut reg2 = MetaRegistry::new();
        let mut a2 = make_test_macro("widget-a");
        a2.form = Some(widget_form(false));
        let mut b2 = make_test_macro("widget-b");
        b2.form = Some(widget_form(true));
        reg2.register_macro(a2).unwrap();
        reg2.register_macro(b2).unwrap();
        assert!(
            reg2.dispatch_ambiguity("widget", &caps, &None).is_none(),
            "a discriminating required capture must resolve the tie"
        );
    }

    #[test]
    fn test_macro_scope_matches_selector_scope() {
        let scopes = vec![MacroScope::Selector];
        assert!(
            !macro_scope_matches(&scopes, &None),
            "Selector scope should not match file-level"
        );
        assert!(
            macro_scope_matches(&scopes, &Some(".foo".to_string())),
            "Selector scope should match inside selector"
        );
    }

    #[test]
    fn test_macro_scope_matches_both_scopes() {
        let scopes = vec![MacroScope::File, MacroScope::Selector];
        assert!(
            macro_scope_matches(&scopes, &None),
            "File+Selector should match file-level"
        );
        assert!(
            macro_scope_matches(&scopes, &Some(".foo".to_string())),
            "File+Selector should match inside selector"
        );
    }

    #[test]
    fn test_macro_scope_matches_selector_only_excludes_file() {
        // The two enforced scopes are mutually exclusive contexts: a Selector-only
        // restriction must not match file-level, but matches a selector context.
        let scopes = vec![MacroScope::Selector];
        assert!(
            !macro_scope_matches(&scopes, &None),
            "Selector-only scope should not match file-level"
        );
        assert!(
            macro_scope_matches(&scopes, &Some(".foo".to_string())),
            "Selector scope should match a selector context"
        );
    }

    #[test]
    fn test_find_macro_by_directive_direct_name() {
        let mut registry = MetaRegistry::new();
        registry
            .register_macro(make_test_macro("my-macro"))
            .unwrap();

        let captures = HashMap::new();
        let found = registry.find_macro_by_directive("my-macro", &captures, &None);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "my-macro");
    }

    #[test]
    fn test_find_macro_by_directive_form_fallback() {
        let mut registry = MetaRegistry::new();

        let mut macro_def = make_test_macro("test-dispatcher");
        macro_def.form = Some(FormClause {
            directive_name: "@test".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "event".to_string(),
                    capture_type: CaptureType::Ident,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: vec![],
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        });
        registry.register_macro(macro_def).unwrap();

        let captures: HashMap<String, CapturedValue> = [(
            "event".to_string(),
            CapturedValue::Ident("visible".to_string()),
        )]
        .into_iter()
        .collect();

        // Direct name still works
        let found = registry.find_macro_by_directive("test-dispatcher", &captures, &None);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-dispatcher");

        // Directive-derived name finds via form fallback
        let found = registry.find_macro_by_directive("test", &captures, &None);
        assert!(
            found.is_some(),
            "Must find macro via form-directive fallback"
        );
        assert_eq!(found.unwrap().name, "test-dispatcher");
    }

    #[test]
    fn test_find_macro_by_directive_nonexistent() {
        let mut registry = MetaRegistry::new();
        registry
            .register_macro(make_test_macro("something"))
            .unwrap();

        let captures = HashMap::new();
        let found = registry.find_macro_by_directive("nonexistent", &captures, &None);
        assert!(found.is_none(), "Nonexistent directive must return None");
    }

    #[test]
    fn module_st_binds_folder_namespace() {
        // FEAT-118 M2: a folder whose MODULE.st declares `%module scene` binds
        // its sibling macros to the `scene` namespace — they key as scene/<name>
        // (M0 Fqn keying), not the bare name.
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let scene = dir.path().join("scene");
        std::fs::create_dir(&scene).unwrap();
        write!(
            std::fs::File::create(scene.join("MODULE.st")).unwrap(),
            "%module scene\n"
        )
        .unwrap();
        write!(
            std::fs::File::create(scene.join("camera.st")).unwrap(),
            "%macro camera {{\n  %form {{ @camera $fov:string }}\n}}\n"
        )
        .unwrap();

        let mut reg = MetaRegistry::new();
        let _ = reg.load_stdlib_from_dir(dir.path());

        // The macro is keyed under the folder namespace, not bare.
        assert!(
            reg.get_macro("scene/camera").is_some(),
            "a MODULE.st `%module scene` folder keys its macros as scene/<name>; \
             keys present: {:?}",
            reg.macro_names().collect::<Vec<_>>()
        );
        assert!(
            reg.get_macro("camera").is_none(),
            "the bare key must NOT exist — the macro is namespaced"
        );
    }

    #[test]
    fn no_module_st_stays_global() {
        // Control: a folder with NO MODULE.st keeps its macros global (bare key).
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        write!(
            std::fs::File::create(dir.path().join("beacon.st")).unwrap(),
            "%macro beacon {{\n  %form {{ @beacon $p:string }}\n}}\n"
        )
        .unwrap();

        let mut reg = MetaRegistry::new();
        let _ = reg.load_stdlib_from_dir(dir.path());
        assert!(
            reg.get_macro("beacon").is_some(),
            "no MODULE.st => global bare key"
        );
    }

    #[test]
    fn module_st_manifest_retained_and_public_enforced() {
        // FEAT-118 M2 (self-hosted): a MODULE.st `%public` list is RETAINED by
        // the registry and read back via is_public — the .st policy drives
        // visibility, no hardcoded Rust rule.
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let scene = dir.path().join("scene");
        std::fs::create_dir(&scene).unwrap();
        write!(
            std::fs::File::create(scene.join("MODULE.st")).unwrap(),
            "%module scene\n%public (camera, light)\n"
        )
        .unwrap();
        write!(
            std::fs::File::create(scene.join("defs.st")).unwrap(),
            "%macro camera {{\n  %form {{ @camera $f:string }}\n}}\n%macro fog {{\n  %form {{ @fog $d:string }}\n}}\n"
        )
        .unwrap();

        let mut reg = MetaRegistry::new();
        let _ = reg.load_stdlib_from_dir(dir.path());

        // The manifest is retained.
        let manifest = reg.module_manifest("scene").expect("manifest retained");
        assert_eq!(
            manifest.public,
            vec!["camera".to_string(), "light".to_string()]
        );

        // is_public reads the .st %public list: listed = public, unlisted = private.
        assert!(reg.is_public("scene", "camera"), "camera is in %public");
        assert!(reg.is_public("scene", "light"), "light is in %public");
        assert!(
            !reg.is_public("scene", "fog"),
            "fog is NOT in %public => private"
        );

        // A namespace with no manifest is public-by-default.
        assert!(
            reg.is_public("nonexistent", "anything"),
            "no manifest => all public"
        );
    }

    #[test]
    fn module_st_no_public_clause_is_all_public() {
        // A MODULE.st with %module but NO %public exposes everything (Odin floor).
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let ui = dir.path().join("ui");
        std::fs::create_dir(&ui).unwrap();
        write!(
            std::fs::File::create(ui.join("MODULE.st")).unwrap(),
            "%module ui\n"
        )
        .unwrap();
        write!(
            std::fs::File::create(ui.join("card.st")).unwrap(),
            "%macro card {{\n  %form {{ @card }}\n}}\n"
        )
        .unwrap();

        let mut reg = MetaRegistry::new();
        let _ = reg.load_stdlib_from_dir(dir.path());
        assert!(
            reg.is_public("ui", "card"),
            "no %public clause => public-by-default"
        );
        assert!(
            reg.is_public("ui", "anything"),
            "public-by-default exposes all"
        );
    }
}
