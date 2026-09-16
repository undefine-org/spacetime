# Generative Testing — `@property` & `@fuzz`

Don't enumerate cases by hand — declare a **property** that must hold for *all*
inputs of a type, and let the runner generate them, find a counterexample, and
**shrink** it to the minimal failing case.

```spacetime
@import "stdlib/testing/test"
@import "stdlib/testing/generative"
```

## `@property` — for-all holds

```spacetime
@property "abs is never negative" forall (n: int -1000..1000) {
  @assert (Math.abs(n) >= 0)
}

@property "reverse is its own inverse" forall (xs: array int) {
  @assert (reverse(reverse(xs)).join() === xs.join())
}
```

- The runner generates many `n` from the spec, runs the body for each, and a
  single failing case fails the property.
- On failure it **shrinks** — searches for the smallest/simplest input that
  still fails — and reports that minimal counterexample plus the **seed** so the
  case is reproducible.

### Input specs

| spec | generates |
|------|-----------|
| `int a..b` | integers in `[a, b]` |
| `number a..b` | floats in `[a, b]` |
| `bool` | `true` / `false` |
| `string ~ <corpus>` | strings from a corpus (e.g. `unicode`, `ascii`) |
| `array <T>` | arrays of a sub-spec |
| `<A> \| <B>` | union — one of the alternatives |

## `@fuzz` — survive adversarial junk

`@fuzz` throws hostile inputs at the body and **fails on a thrown error or a
per-case timeout**. Use it for parsers, sanitizers, and anything that takes
untrusted input.

```spacetime
@fuzz "parser survives junk" inputs (s: string ~ unicode) {
  @eval (parse(s))   // must not throw or hang
}
```

## Compiler-tier fuzzing

The Spacetime **compiler itself** is fuzzed from Rust — the parser must never
panic on arbitrary input:

```sh
cargo test --test parser_fuzz_test       # ~thousands of proptest cases, libfuzzer-free
cargo +nightly fuzz run parse_target     # the cargo-fuzz crate under fuzz/
```

`tests/parser_fuzz_test.rs` asserts `parse(arbitrary) ⇒ Ok | Err`, never a
panic. The `fuzz/` crate has `parse` and `compile` targets for deeper coverage.

## How it works

The engine lives in `public/runtime/generative.js` (deterministic Mulberry32
PRNG so a seed reproduces a run exactly): `__stGen` builds values from a spec,
`__stProperty` does generate → check → structural-shrink, `__stFuzz` does the
crash/timeout boundary. The `@property` / `@fuzz` macros in
`stdlib/testing/generative.st` capture the binding as a raw string and bind the
generated value at runtime.
