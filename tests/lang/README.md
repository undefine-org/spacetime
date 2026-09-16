# `tests/lang/` — language-surface tests

One directory per **language feature area**. Each directory proves what the
*author-visible surface* of that feature does — the grammar it accepts, the
grammar it refuses, and the observable behaviour of what it compiles to.

```
tests/lang/<feature-area>/
    <aspect>.test.st     one file per ASPECT of the surface, not per bug
    <aspect>_test.rs     a Rust companion ONLY where .test.st cannot reach
    README.md            what this dir proves, and what it deliberately does not
```

## The rule, and why the rest of `tests/` looks different

`tests/` grew organically and the placement of a test currently tells you
nothing. `.test.st` files live at the root (12), in `bugs/` (17),
`integration/` (10), `component-model/` (5), and `e2e/` (1), with no rule
separating them. This directory establishes the rule going forward:

| directory | holds | keyed by |
|---|---|---|
| `tests/lang/` | language-surface behaviour | feature area |
| `tests/bugs/` | one file per `BUG-NNN` regression | bug id |
| `tests/component-model/` | template/component semantics | aspect |
| `tests/integration/` | compiler-pipeline integration (mostly Rust) | aspect |
| `tests/e2e/` | live-browser end-to-end | scenario |

Existing files are **not** being moved wholesale — a rename storm buries real
diffs. New language-surface tests go here; migration of the strays is tracked
separately.

## Naming

- A file is named for the **aspect of the surface** it maps
  (`slash-disambiguation`, `qualified-reference`), never for a bug or a wave.
- A `@test` name states the **claim**, not the mechanics:
  `"spaced slash stays division"`, not `"test slash 3"`.
- A test that is expected to be RED until a specific wave lands says so in a
  comment above it, naming the wave.

## RED matrices

A test file written **before** the implementation is a contract, and it belongs
in the same directory as the tests it will become. Two requirements:

1. A RED test must **run and fail its claim** — a file that fails to parse is
   not a RED matrix, it is a broken file. If the syntax under test does not
   parse yet, assert the *observable consequence* (missing DOM, missing signal,
   absent emit) rather than writing the unparseable syntax at file scope.
2. Every RED test names the wave that turns it GREEN, so a passing-too-early
   test is as visible as a failing one.

## Scratch state — never `/tmp`

A test that needs a throwaway project on disk writes it under
**`scratch/tests/<area>/`** (repo-local, gitignored via `/scratch/`), never
`std::env::temp_dir()`. Two reasons, both observed rather than theoretical:

1. **`/tmp` contention produces phantom failures.** Running `cargo test --lib`
   at default parallelism against a 69%-full `/tmp` yielded 133 failures
   (including `Disk quota exceeded`); every one passed at `--test-threads=4`.
   A suite whose result depends on the machine's temp pressure cannot gate
   anything.
2. **Leftovers stay inspectable.** When a test fails mid-run, its fixture is
   still in the workspace where you can read it, not scattered under a
   system directory keyed by pid.

Create the dir fresh, clean it up on success, and remove the shared root when
it empties — a green run must leave the working tree exactly as it found it.
See `scratch_root` / `scratch_dir` / `cleanup` in
`tests/lang/registry_addressing_diagnostics.rs` for the pattern.

## Running

```bash
cargo run --features headless -- test tests/lang/ --headless          # all
cargo run --features headless -- test tests/lang/<area>/ --headless   # one area
```

`--headless` is the fast V8 `logic` backend. Layout/timing/paint claims refuse
there by design — use `--cdp` for those. See `docs/testing/README.md`.
