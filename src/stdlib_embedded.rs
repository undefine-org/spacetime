//! Embedded stdlib files for when filesystem access isn't available.
//!
//! This module provides embedded stdlib files that can be used when the
//! spacetime library is used as a dependency and the stdlib directory
//! isn't accessible from the current working directory.

/// Embedded stdlib file entry
pub struct EmbeddedFile {
    pub path: &'static str,
    pub content: &'static str,
}

/// All embedded stdlib files organized by category
pub mod capture_types {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[
        EmbeddedFile {
            path: "on-actions.st",
            content: include_str!("../stdlib/capture-types/on-actions.st"),
        },
        EmbeddedFile {
            path: "param_list.st",
            content: include_str!("../stdlib/capture-types/param_list.st"),
        },
        EmbeddedFile {
            path: "type-variants.st",
            content: include_str!("../stdlib/capture-types/type-variants.st"),
        },
        EmbeddedFile {
            path: "signal-receive.st",
            content: include_str!("../stdlib/capture-types/signal-receive.st"),
        },
        EmbeddedFile {
            path: "handle-block.st",
            content: include_str!("../stdlib/capture-types/handle-block.st"),
        },
        // The CSS scalar grammars (PLAN-122). `stdlib/scalars/types.st` was
        // embedded without them, so a distributed binary with no disk stdlib had
        // the scalar TABLE and none of the productions it names — every
        // `capture_type_accepts` lookup then took its unknown-type path and
        // accepted anything. The table and its grammars must travel together.
        EmbeddedFile {
            path: "css-values.st",
            content: include_str!("../stdlib/capture-types/css-values.st"),
        },
        EmbeddedFile {
            path: "spacetime-values.st",
            content: include_str!("../stdlib/capture-types/spacetime-values.st"),
        },
        // The value-node grammar. It is what `validate_declaration_value` asks
        // "is this deferred, or a wide keyword?" — the productions that replaced
        // three hand-written Rust branches. Unembedded, a distributed binary
        // would take the unknown-type path and accept anything, which is the
        // exact failure this parity test was written for.
        EmbeddedFile {
            path: "value-node.st",
            content: include_str!("../stdlib/capture-types/value-node.st"),
        },
        EmbeddedFile {
            path: "properties.st",
            content: include_str!("../stdlib/capture-types/properties.st"),
        },
        EmbeddedFile {
            path: "transition-body.st",
            content: include_str!("../stdlib/capture-types/transition-body.st"),
        },
        EmbeddedFile {
            path: "shot-body.st",
            content: include_str!("../stdlib/capture-types/shot-body.st"),
        },
        EmbeddedFile {
            path: "claims.st",
            content: include_str!("../stdlib/capture-types/claims.st"),
        },
        EmbeddedFile {
            path: "component-body.st",
            content: include_str!("../stdlib/capture-types/component-body.st"),
        },
        EmbeddedFile {
            path: "keyframes.st",
            content: include_str!("../stdlib/capture-types/keyframes.st"),
        },
    ];
}

pub mod runtime {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[EmbeddedFile {
        path: "registries.st",
        content: include_str!("../stdlib/runtime/registries.st"),
    }];
}

pub mod syntax {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[
        EmbeddedFile {
            path: "entity-scope.st",
            content: include_str!("../stdlib/syntax/entity-scope.st"),
        },
        EmbeddedFile {
            path: "element-ref.st",
            content: include_str!("../stdlib/syntax/element-ref.st"),
        },
        EmbeddedFile {
            path: "local-state.st",
            content: include_str!("../stdlib/syntax/local-state.st"),
        },
        EmbeddedFile {
            path: "version.st",
            content: include_str!("../stdlib/syntax/version.st"),
        },
    ];
}

pub mod primitives {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[
        EmbeddedFile {
            path: "css-property.st",
            content: include_str!("../stdlib/primitives/css-property.st"),
        },
        EmbeddedFile {
            path: "dom.st",
            content: include_str!("../stdlib/primitives/dom.st"),
        },
        EmbeddedFile {
            path: "fetch.st",
            content: include_str!("../stdlib/primitives/fetch.st"),
        },
        EmbeddedFile {
            path: "signal.st",
            content: include_str!("../stdlib/primitives/signal.st"),
        },
        EmbeddedFile {
            path: "host-register.st",
            content: include_str!("../stdlib/primitives/host-register.st"),
        },
        EmbeddedFile {
            path: "signal-handler.st",
            content: include_str!("../stdlib/primitives/signal-handler.st"),
        },
        EmbeddedFile {
            path: "signal-arms.st",
            content: include_str!("../stdlib/primitives/signal-arms.st"),
        },
        EmbeddedFile {
            path: "floating-toolbar.st",
            content: include_str!("../stdlib/primitives/floating-toolbar.st"),
        },
        EmbeddedFile {
            path: "gesture.st",
            content: include_str!("../stdlib/primitives/gesture.st"),
        },
        EmbeddedFile {
            path: "intersection.st",
            content: include_str!("../stdlib/primitives/intersection.st"),
        },
        EmbeddedFile {
            path: "media.st",
            content: include_str!("../stdlib/primitives/media.st"),
        },
        EmbeddedFile {
            path: "mutation.st",
            content: include_str!("../stdlib/primitives/mutation.st"),
        },
        EmbeddedFile {
            path: "pointer.st",
            content: include_str!("../stdlib/primitives/pointer.st"),
        },
        EmbeddedFile {
            path: "resize.st",
            content: include_str!("../stdlib/primitives/resize.st"),
        },
        EmbeddedFile {
            path: "scroll.st",
            content: include_str!("../stdlib/primitives/scroll.st"),
        },
        EmbeddedFile {
            path: "selection-unwrapper.st",
            content: include_str!("../stdlib/primitives/selection-unwrapper.st"),
        },
        EmbeddedFile {
            path: "selection-wrapper.st",
            content: include_str!("../stdlib/primitives/selection-wrapper.st"),
        },
        EmbeddedFile {
            path: "socket.st",
            content: include_str!("../stdlib/primitives/socket.st"),
        },
        EmbeddedFile {
            path: "template.st",
            content: include_str!("../stdlib/primitives/template.st"),
        },
        EmbeddedFile {
            path: "tick.st",
            content: include_str!("../stdlib/primitives/tick.st"),
        },
        EmbeddedFile {
            path: "now.st",
            content: include_str!("../stdlib/primitives/now.st"),
        },
        EmbeddedFile {
            path: "websocket.st",
            content: include_str!("../stdlib/primitives/websocket.st"),
        },
        // Animation primitives
        EmbeddedFile {
            path: "animation/animate.st",
            content: include_str!("../stdlib/primitives/animation/animate.st"),
        },
        EmbeddedFile {
            path: "animation/apply-animations.st",
            content: include_str!("../stdlib/primitives/animation/apply-animations.st"),
        },
        EmbeddedFile {
            path: "animation/color.st",
            content: include_str!("../stdlib/primitives/animation/color.st"),
        },
        EmbeddedFile {
            path: "animation/drivers.st",
            content: include_str!("../stdlib/primitives/animation/drivers.st"),
        },
        EmbeddedFile {
            path: "animation/easing.st",
            content: include_str!("../stdlib/primitives/animation/easing.st"),
        },
        EmbeddedFile {
            path: "animation/functions.st",
            content: include_str!("../stdlib/primitives/animation/functions.st"),
        },
        EmbeddedFile {
            path: "animation/index.st",
            content: include_str!("../stdlib/primitives/animation/index.st"),
        },
        EmbeddedFile {
            path: "animation/interpolate.st",
            content: include_str!("../stdlib/primitives/animation/interpolate.st"),
        },
        // Data primitives
        EmbeddedFile {
            path: "data/binding.st",
            content: include_str!("../stdlib/primitives/data/binding.st"),
        },
        EmbeddedFile {
            path: "data/cart-count.st",
            content: include_str!("../stdlib/primitives/data/cart-count.st"),
        },
        EmbeddedFile {
            path: "data/cart-items.st",
            content: include_str!("../stdlib/primitives/data/cart-items.st"),
        },
        EmbeddedFile {
            path: "data/cart-total.st",
            content: include_str!("../stdlib/primitives/data/cart-total.st"),
        },
        EmbeddedFile {
            path: "data/computed-source.st",
            content: include_str!("../stdlib/primitives/data/computed-source.st"),
        },
        EmbeddedFile {
            path: "data/each.st",
            content: include_str!("../stdlib/primitives/data/each.st"),
        },
        EmbeddedFile {
            path: "data/filters.st",
            content: include_str!("../stdlib/primitives/data/filters.st"),
        },
        EmbeddedFile {
            path: "data/fn-registry.st",
            content: include_str!("../stdlib/primitives/data/fn-registry.st"),
        },
        EmbeddedFile {
            path: "data/index.st",
            content: include_str!("../stdlib/primitives/data/index.st"),
        },
        EmbeddedFile {
            path: "data/local-storage.st",
            content: include_str!("../stdlib/primitives/data/local-storage.st"),
        },
        EmbeddedFile {
            path: "data/reactive-binding.st",
            content: include_str!("../stdlib/primitives/data/reactive-binding.st"),
        },
        EmbeddedFile {
            path: "data/source.st",
            content: include_str!("../stdlib/primitives/data/source.st"),
        },
        // Data: compute & text-cycle
        EmbeddedFile {
            path: "data/compute-fn.st",
            content: include_str!("../stdlib/primitives/data/compute-fn.st"),
        },
        EmbeddedFile {
            path: "data/text-cycle.st",
            content: include_str!("../stdlib/primitives/data/text-cycle.st"),
        },
        // NOTE: the hand-rolled WebGL primitives were removed (PLAN-050); 3D is now
        // the vendored three.js `stdlib/3d` module (filesystem-loaded, opt-in).
    ];
}

pub mod macros {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[
        EmbeddedFile {
            path: "bindings.st",
            content: include_str!("../stdlib/macros/bindings.st"),
        },
        EmbeddedFile {
            path: "data.st",
            content: include_str!("../stdlib/macros/data.st"),
        },
        EmbeddedFile {
            path: "drag.st",
            content: include_str!("../stdlib/macros/drag.st"),
        },
        EmbeddedFile {
            path: "each.st",
            content: include_str!("../stdlib/macros/each.st"),
        },
        EmbeddedFile {
            path: "editable.st",
            content: include_str!("../stdlib/macros/editable.st"),
        },
        EmbeddedFile {
            path: "fade-in.st",
            content: include_str!("../stdlib/macros/fade-in.st"),
        },
        EmbeddedFile {
            path: "form.st",
            content: include_str!("../stdlib/macros/form.st"),
        },
        EmbeddedFile {
            path: "drivers.st",
            content: include_str!("../stdlib/macros/drivers.st"),
        },
        EmbeddedFile {
            path: "on.st",
            content: include_str!("../stdlib/macros/on.st"),
        },
        EmbeddedFile {
            path: "loop.st",
            content: include_str!("../stdlib/macros/loop.st"),
        },
        EmbeddedFile {
            path: "on-event.st",
            content: include_str!("../stdlib/macros/on-event.st"),
        },
        EmbeddedFile {
            path: "now.st",
            content: include_str!("../stdlib/macros/now.st"),
        },
        EmbeddedFile {
            path: "presets.st",
            content: include_str!("../stdlib/macros/presets.st"),
        },
        EmbeddedFile {
            path: "responsive.st",
            content: include_str!("../stdlib/macros/responsive.st"),
        },
        EmbeddedFile {
            path: "template.st",
            content: include_str!("../stdlib/macros/template.st"),
        },
        EmbeddedFile {
            path: "timeline.st",
            content: include_str!("../stdlib/macros/timeline.st"),
        },
        EmbeddedFile {
            path: "type-data.st",
            content: include_str!("../stdlib/macros/type-data.st"),
        },
        EmbeddedFile {
            path: "host.st",
            content: include_str!("../stdlib/macros/host.st"),
        },
        EmbeddedFile {
            path: "handle.st",
            content: include_str!("../stdlib/macros/handle.st"),
        },
        EmbeddedFile {
            path: "websocket.st",
            content: include_str!("../stdlib/macros/websocket.st"),
        },
        // NOTE: the hand-rolled `@scene` macro family was removed (PLAN-050); 3D is
        // now the vendored three.js `stdlib/3d` module (filesystem-loaded, opt-in).
    ];
}

pub mod mcp {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[
        EmbeddedFile {
            path: "MODULE.st",
            content: include_str!("../stdlib/__mcp__/MODULE.st"),
        },
        EmbeddedFile {
            path: "index.st",
            content: include_str!("../stdlib/__mcp__/index.st"),
        },
        EmbeddedFile {
            path: "env.st",
            content: include_str!("../stdlib/__mcp__/env.st"),
        },
        EmbeddedFile {
            path: "workbench/data.st",
            content: include_str!("../stdlib/__mcp__/workbench/data.st"),
        },
        EmbeddedFile {
            path: "workbench/cards.st",
            content: include_str!("../stdlib/__mcp__/workbench/cards.st"),
        },
        EmbeddedFile {
            path: "workbench/lists.st",
            content: include_str!("../stdlib/__mcp__/workbench/lists.st"),
        },
        EmbeddedFile {
            path: "workbench/actions.st",
            content: include_str!("../stdlib/__mcp__/workbench/actions.st"),
        },
        EmbeddedFile {
            path: "workbench/styles.st",
            content: include_str!("../stdlib/__mcp__/workbench/styles.st"),
        },
        EmbeddedFile {
            path: "primitives/host.st",
            content: include_str!("../stdlib/__mcp__/primitives/host.st"),
        },
        EmbeddedFile {
            path: "primitives/bundle.st",
            content: include_str!("../stdlib/__mcp__/primitives/bundle.st"),
        },
        // Interaction kit (PLAN-043): picker/confirm/form mount as functions.
        EmbeddedFile {
            path: "kit/index.st",
            content: include_str!("../stdlib/__mcp__/kit/index.st"),
        },
        EmbeddedFile {
            path: "kit/picker.st",
            content: include_str!("../stdlib/__mcp__/kit/picker.st"),
        },
        EmbeddedFile {
            path: "kit/confirm.st",
            content: include_str!("../stdlib/__mcp__/kit/confirm.st"),
        },
        EmbeddedFile {
            path: "kit/form.st",
            content: include_str!("../stdlib/__mcp__/kit/form.st"),
        },
        EmbeddedFile {
            path: "kit/review.st",
            content: include_str!("../stdlib/__mcp__/kit/review.st"),
        },
    ];
}

pub mod testing {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[
        // assertions/
        EmbeddedFile {
            path: "assertions/binding.st",
            content: include_str!("../stdlib/testing/assertions/binding.st"),
        },
        EmbeddedFile {
            path: "assertions/state.st",
            content: include_str!("../stdlib/testing/assertions/state.st"),
        },
        EmbeddedFile {
            path: "assertions/timeline.st",
            content: include_str!("../stdlib/testing/assertions/timeline.st"),
        },
        // top-level
        EmbeddedFile {
            path: "reporter.st",
            content: include_str!("../stdlib/testing/reporter.st"),
        },
        EmbeddedFile {
            path: "test.st",
            content: include_str!("../stdlib/testing/test.st"),
        },
    ];
}

pub mod admin {
    use super::EmbeddedFile;

    /// Spacetime CMS admin module. Currently only the `@cms` display-hint macro,
    /// which must be available when compiling user `.st` so authors can annotate
    /// their `@type`s for the local content admin. Produces no runtime output.
    pub const FILES: &[EmbeddedFile] = &[EmbeddedFile {
        path: "cms-hint.st",
        content: include_str!("../stdlib/__admin__/cms-hint.st"),
    }];
}

/// PLAN-077 W1: the `enum` module's capture types. The filesystem registry scan
/// covers the whole `stdlib/enum/` dir (STDLIB_DIRS); embedded mode lists files
/// explicitly. Only meta-def-carrying files are embedded (MODULE.st/index.st
/// carry no defs). `match_arm` lives here now (it was never embedded as
/// match-block.st — embedded @match grammar was already absent; W1 closes that
/// gap rather than preserving it).
pub mod enum_module {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[
        EmbeddedFile {
            path: "capture-types/variant-pattern.st",
            content: include_str!("../stdlib/enum/capture-types/variant-pattern.st"),
        },
        EmbeddedFile {
            path: "capture-types/match-arms.st",
            content: include_str!("../stdlib/enum/capture-types/match-arms.st"),
        },
        EmbeddedFile {
            path: "capture-types/cond-arms.st",
            content: include_str!("../stdlib/enum/capture-types/cond-arms.st"),
        },
        EmbeddedFile {
            path: "capture-types/inline-union.st",
            content: include_str!("../stdlib/enum/capture-types/inline-union.st"),
        },
        // PLAN-077 W2: `%macro data-derive-match` + `%primitive derive-match`.
        EmbeddedFile {
            path: "derive.st",
            content: include_str!("../stdlib/enum/derive.st"),
        },
        // PLAN-077 W5: `%macro match`/`%macro view` + `%primitive dispatch-mount`
        // (the merged view-mount/match-render).
        EmbeddedFile {
            path: "dispatch.st",
            content: include_str!("../stdlib/enum/dispatch.st"),
        },
        // PLAN-077 W6: `%macro state`/`state-match` (the optional-bindings
        // `state_when` capture covers both the bare and destructure forms —
        // the old state-machine.st's separate state-match-simple was NOT
        // carried over) + `%primitive state-reflect` (the generalized
        // data-st-state writer).
        EmbeddedFile {
            path: "state.st",
            content: include_str!("../stdlib/enum/state.st"),
        },
    ];
}

/// The `comments` module's registry-loaded entries (PLAN-123). Same carve-out
/// as `migrations`: only `entries/` is registry data — the pill in `__dev__/`
/// is dev-server-compiled and must never register.
///
/// Embedding matters for parity: a binary running without the stdlib on disk
/// must serve the SAME comment-type roster, or `//@todo` would resolve in a
/// source checkout and be "unknown type" from an installed binary.
pub mod comments {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[EmbeddedFile {
        path: "entries/defaults.st",
        content: include_str!("../stdlib/comments/entries/defaults.st"),
    }];
}

/// The `migrations` module's registry-loaded entries (PLAN-076). The module's
/// `__dev__/` (pill widget) is dev-server-compiled from disk, never embedded;
/// `MODULE.st` carries no defs.
pub mod migrations {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[
        EmbeddedFile {
            path: "entries/2026-06-09-reactive-surface.st",
            content: include_str!("../stdlib/migrations/entries/2026-06-09-reactive-surface.st"),
        },
        EmbeddedFile {
            path: "entries/2026-07-26-on-cutover.st",
            content: include_str!("../stdlib/migrations/entries/2026-07-26-on-cutover.st"),
        },
    ];
}

/// The scalar type table (FEAT-168 / PLAN-122 W4), embedded so an installed
/// binary (no source checkout) serves the same single source of truth that
/// `stdlib/scalars/types.st` provides from disk — exactly like the comments
/// roster above. `all_embedded_files` keys it as `scalars`, so the virtual
/// path `stdlib/scalars/types.st` matches the on-disk `source_file` and the
/// duplicate/reload guards treat both loads identically.
pub mod scalars {
    use super::EmbeddedFile;

    pub const FILES: &[EmbeddedFile] = &[EmbeddedFile {
        path: "types.st",
        content: include_str!("../stdlib/scalars/types.st"),
    }];
}

/// Get all embedded stdlib files for loading into a MetaRegistry
pub fn all_embedded_files() -> Vec<(&'static str, &'static [EmbeddedFile])> {
    vec![
        ("capture-types", capture_types::FILES),
        ("runtime", runtime::FILES),
        ("syntax", syntax::FILES),
        ("primitives", primitives::FILES),
        ("macros", macros::FILES),
        ("admin", admin::FILES),
        ("__mcp__", mcp::FILES),
        ("testing", testing::FILES),
        ("enum", enum_module::FILES),
        ("migrations", migrations::FILES),
        ("comments", comments::FILES),
        ("scalars", scalars::FILES),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_prelude_is_embedded() {
        let embedded = all_embedded_files();
        let (_, files) = embedded
            .iter()
            .find(|(category, _)| *category == "__mcp__")
            .expect("__mcp__ stdlib module should be embedded");

        let paths: Vec<&str> = files.iter().map(|file| file.path).collect();
        assert!(paths.contains(&"index.st"));
        assert!(paths.contains(&"env.st"));
        assert!(paths.contains(&"workbench/data.st"));
        assert!(paths.contains(&"workbench/cards.st"));
        assert!(paths.contains(&"workbench/lists.st"));
        assert!(paths.contains(&"workbench/actions.st"));
        assert!(paths.contains(&"workbench/styles.st"));
        assert!(paths.contains(&"primitives/host.st"));
        assert!(paths.contains(&"primitives/bundle.st"));
        // Interaction kit (PLAN-043).
        assert!(paths.contains(&"kit/index.st"));
        assert!(paths.contains(&"kit/picker.st"));
        assert!(paths.contains(&"kit/confirm.st"));
        assert!(paths.contains(&"kit/form.st"));
        assert!(paths.contains(&"kit/review.st"));
    }

    /// The scalar type table (FEAT-168 / PLAN-122 W4) is the SINGLE SOURCE for
    /// what a scalar means, so it must be baked into the binary — an installed
    /// build with no source checkout would otherwise serve NO scalar table, and
    /// every consumer (schema, zero, widget) would silently resolve nothing.
    #[test]
    fn scalar_table_is_embedded() {
        let embedded = all_embedded_files();
        let (_, files) = embedded
            .iter()
            .find(|(category, _)| *category == "scalars")
            .expect("scalars stdlib module should be embedded");

        let paths: Vec<&str> = files.iter().map(|f| f.path).collect();
        assert!(paths.contains(&"types.st"));

        // The embedded copy must PARSE into the 8 scalar rows — an embedded
        // table that does not load is a table that silently does nothing.
        let mut reg = crate::metasystem::MetaRegistry::new();
        let mut errors = Vec::new();
        crate::compiler::load_embedded_stdlib(&mut reg, &mut errors);
        assert!(
            errors.is_empty(),
            "embedded stdlib must load cleanly: {errors:?}"
        );
        let ids: Vec<&str> = reg.scalar_types().map(|t| t.id.as_str()).collect();
        for expected in [
            "color", "length", "duration", "url", "richtext", "string", "number", "boolean",
        ] {
            assert!(
                ids.contains(&expected),
                "embedded scalar `{expected}` missing: {ids:?}"
            );
        }
    }
}

#[cfg(test)]
mod embedded_parity {
    /// Every `stdlib/capture-types/*.st` on disk must also be EMBEDDED.
    ///
    /// The two lists are maintained by hand, and they drifted: `css-values.st`
    /// — the CSS scalar grammars — was missing while `stdlib/scalars/types.st`
    /// (the table that NAMES those grammars) was present. A distributed binary
    /// with no disk stdlib therefore had the table and none of the productions,
    /// so every `capture_type_accepts` lookup took its unknown-type path and
    /// accepted anything. The checkout build refused `#e8ee1`; the installed
    /// binary did not.
    ///
    /// That class has bitten before (BUG-227: falling back to embedded silently
    /// parsed a user's site against a stale `%form` set). A binary that behaves
    /// differently from the repo it was built from is the worst kind of bug —
    /// it cannot be reproduced where it is debugged.
    #[test]
    fn every_capture_type_file_on_disk_is_embedded() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("stdlib/capture-types");
        let mut missing = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("stdlib/capture-types exists").flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("st") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if !super::capture_types::FILES.iter().any(|f| f.path == name) {
                missing.push(name);
            }
        }
        assert!(
            missing.is_empty(),
            "these capture-type grammars exist on disk but are NOT embedded, so a \
             distributed binary would silently behave differently from this checkout: {missing:?}"
        );
    }
}
