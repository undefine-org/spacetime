//! Fuzz the Spacetime parser (PLAN-027 W4, compiler tier).
//! Invariant: `parse` must never panic or hang on ANY input — it returns
//! `Result<StFile, ParseErrors>`, so a panic is a real bug.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = spacetime::parser::parse(s);
    }
});
