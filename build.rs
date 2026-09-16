//! Build script — lazy vendored-bundle embed trigger (PLAN-024 W1).
//!
//! On `cargo build`, ensure each stdlib module's vendored `out:` artifact is
//! fresh from its pinned git submodule, so the embedded stdlib bakes in a
//! current blob. This shares the SINGLE bundle routine in `src/vendor_core.rs`
//! (included below) with the `vendor build` CLI — one routine, two triggers.
//!
//! LAZY: rebundling is skipped when the artifact matches the submodule commit,
//! or when `bun` is absent but an artifact is already present. This keeps
//! `cargo build` working for contributors who are not touching the vendored dep.
//! A missing module dir (e.g. before a module lands) is a silent no-op.
//!
//! The spec table below is hand-maintained, mirroring the existing hand-listed
//! stdlib tables in `src/stdlib_embedded.rs` / `src/compiler.rs`.

include!("src/vendor_core.rs");

use std::path::PathBuf;
use vendor_core::{BundleSpec, Strategy, bundle};

/// One vendored bundle to keep fresh on `cargo build`.
struct VendorEntry {
    module_dir: &'static str,
    name: &'static str,
    source: &'static str,
    entry: &'static str,
    out: &'static str,
    strategy: Strategy,
}

/// Hand-listed vendored bundles. Mirrors the `%vendor` declarations in each
/// module's `MODULE.st`. Keep in sync when adding a vendored dependency.
const VENDORS: &[VendorEntry] = &[
    VendorEntry {
        module_dir: "stdlib/text",
        name: "pretext",
        source: "vendor/pretext",
        entry: "vendor/pretext-entry.ts",
        out: "vendor/pretext.bundle.js",
        strategy: Strategy::Bun,
    },
    VendorEntry {
        module_dir: "stdlib/3d",
        name: "three",
        source: "vendor/three",
        entry: "vendor/three-entry.ts",
        out: "vendor/three.bundle.js",
        strategy: Strategy::Bun,
    },
];

fn main() {
    for v in VENDORS {
        let module_dir = PathBuf::from(v.module_dir);

        // Module not present yet → silent no-op (the module lands in a later wave).
        if !module_dir.exists() {
            continue;
        }

        // Re-run when the submodule pointer or the artifact changes.
        println!("cargo:rerun-if-changed={}/{}", v.module_dir, v.source);
        println!("cargo:rerun-if-changed={}/{}", v.module_dir, v.out);

        let spec = BundleSpec {
            name: v.name.to_string(),
            module_dir,
            source: v.source.to_string(),
            entry: v.entry.to_string(),
            out: v.out.to_string(),
            strategy: v.strategy,
        };

        match bundle(&spec, false) {
            Ok(outcome) => {
                // Only surface a message when we actually rebundled; the lazy
                // skip is the silent common path.
                if !outcome.skipped {
                    println!(
                        "cargo:warning=vendor {}: rebuilt ({} bytes)",
                        v.name, outcome.bytes
                    );
                }
            }
            Err(e) => {
                // Hard error only when there is genuinely nothing to embed (no
                // artifact AND no way to build it). Emit a build error.
                panic!("vendor bundle `{}` failed: {}", v.name, e);
            }
        }
    }
}
