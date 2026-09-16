use spacetime::metasystem::MetaRegistry;
use std::path::Path;

/// Smoke-check that the PLAN-077 W6 state surface registers: `%macro state` +
/// `state-match` from `stdlib/enum/state.st` and `@socket` from
/// `stdlib/macros/websocket.st` (the retired state-machine.st's replacements).
fn main() {
    let mut registry = MetaRegistry::new();

    println!("Loading stdlib/enum + stdlib/macros...");
    for dir in ["stdlib/enum", "stdlib/macros"] {
        if let Err(e) = registry.load_stdlib_from_dir(Path::new(dir)) {
            eprintln!("Error loading {}: {:?}", dir, e);
        }
    }

    println!("\nLooking for the W6 state surface:");
    println!("  state: {}", registry.has_macro("state"));
    println!("  state-match: {}", registry.has_macro("state-match"));
    println!("  socket: {}", registry.has_macro("socket"));
}
