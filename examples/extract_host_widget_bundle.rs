//! Regenerates `commercial/host/frontend/host-widget/dist/` -- the
//! sentinel-intact compiled `__host__` widget bundle that
//! `spacetime-host-cli::widget_bundle` embeds via `include_str!` (see that
//! module's own doc comment: this is the exact throwaway script it
//! describes; it was deleted once and is now recreated as a KEPT example
//! so the regeneration step is `cargo run --example
//! extract_host_widget_bundle` rather than re-writing this file from a
//! doc comment every time the widget's source changes).
//!
//! Run from `verse`'s repo root. Commit the regenerated
//! `frontend/host-widget/dist/{spacetime.js,spacetime.css,shell.html}`
//! in the `commercial/host` repo.

use spacetime::Compiler;
use std::path::Path;

fn main() {
    let entry = Path::new("stdlib/__host__/index.st");
    let root = Path::new(".");
    let compiled = Compiler::from_file(entry, root)
        .expect("failed to load stdlib/__host__/index.st")
        .compile();

    let out_dir = Path::new("commercial/host/frontend/host-widget/dist");
    std::fs::create_dir_all(out_dir).expect("failed to create dist dir");
    std::fs::write(out_dir.join("spacetime.js"), &compiled.js).expect("write spacetime.js");
    std::fs::write(out_dir.join("spacetime.css"), &compiled.css).expect("write spacetime.css");
    std::fs::write(out_dir.join("shell.html"), &compiled.html).expect("write shell.html");

    println!(
        "wrote {} (js {} bytes, css {} bytes, html {} bytes)",
        out_dir.display(),
        compiled.js.len(),
        compiled.css.len(),
        compiled.html.len()
    );
}
