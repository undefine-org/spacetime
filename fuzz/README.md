# Spacetime fuzz targets (PLAN-027 W4 — compiler tier)

Pure-Rust libfuzzer targets that assert the parser/compiler never panic or hang.

## Run

```sh
cargo install cargo-fuzz          # one-time (needs nightly toolchain)
cargo +nightly fuzz run parse     # fuzz parse()
cargo +nightly fuzz run compile   # fuzz parse() -> compile()
cargo +nightly fuzz run parse -- -max_total_time=60   # time-boxed
```

## Targets

- `parse`   — `spacetime::parser::parse(utf8)` must never panic.
- `compile` — a parsed AST must compile without panicking.

## Corpus

Seed the corpus from the tracked fixtures + test inputs:

```sh
mkdir -p corpus/parse
cp tests/fixtures/**/*.st corpus/parse/ 2>/dev/null || true
find tests examples stdlib -name '*.st' -exec cp {} corpus/parse/ \; 2>/dev/null || true
```

## Runtime tier

The *runtime* layer (generated JS behaviour) is fuzzed separately via the
`@fuzz` directive once the generative macro layer lands (FEAT-065 — blocked on
structured-capture field-access in `%emit`). The runtime generator/shrinker
(`window.__stGen` / `__stFuzz` / `__stProperty`) is already implemented in the
test runtime (`src/test_runner.rs`).
