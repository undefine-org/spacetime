//! Fuzz the full parse->compile path (PLAN-027 W4, compiler tier).
//! Invariant: a successfully-parsed file must compile without panicking.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(ast) = spacetime::parser::parse(s) {
            let _ = spacetime::compile(&ast, spacetime::compiler::CompileOptions::default());
        }
    }
});
