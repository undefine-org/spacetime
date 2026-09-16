# Git management

- NEVER stash. If you need to check against the old code, ask for my input on what to do
- ALWAYS keep a record of your work in the org-mode files in @tasks and/or @research/. Colocate the tests + docs + PRD updates in the commits that implemented the work.

# Decisionmaking

- Spacetime websites should NOT have custom javascript; they should ONLY rely on Spacetime for everything (style, animations, state management, server communications, etc.). If there is a feature that is legitimately missing, please explain what that feature is and switch to planning mode to design, test, and implement it properly, before continuing on the design of that website.

- **DO NOT write Rust-based parsers/fallbacks for constructs defined in Spacetime's stdlib (like %capture_type).** If stdlib patterns aren't loading, fix the stdlib loading mechanism - don't bypass it with hardcoded Rust implementations. The metasystem is designed to be self-describing.

# Debugging

- Enable trace logging with `RUST_LOG=trace cargo run -- serve projects/<name>/` or target specific modules: `RUST_LOG=spacetime::pipeline=trace,spacetime::metasystem=trace`
- Use `cargo run -- {check|inspect --layer <layer>|build}` to better understand how the complier procudes Output

# Instructions

- Always write a descriptive "alt" for the images you include with `<image>`. Read the file for that.

# Task Management

- Commit task status updates alongside the code changes they describe
- Read `@tasks/reference.org` for full PRD system documentation
- Validate after editing @tasks files: `emacsclient -e '(spacetime--prd-validate-all-cli :format json)'`

# Commands

- `cargo test --lib` -> unit tests (1574+)
- `cargo test --test integration_tests` -> integration tests (442+)
- `cargo test --test v8_runtime_test` -> JS runtime tests (139)
- `cargo run --features headless -- test tests/ --headless` -> headless Spacetime tests (341+)
- `cargo run -- serve projects/<name>/` -> dev server
- `cargo run -- check` -> compile check
- `cargo run -- inspect --layer <layer>` -> layer inspection
- `emacsclient -e '(spacetime--prd-validate-all-cli :format json)'` -> validate PRD tasks


# IMPORTANT

Read [docs/antipatterns.md] to know what you need to not do


# Project layout

- `projects/` — private Spacetime websites (gitignored; separate git repo). Each `<name>/` builds to `projects/<name>/dist/`, which is gitignored inside `projects/`. Use `cargo run -- serve projects/<name>/`.
- `tests/fixtures/` — compiler integration test fixtures (test-runner UI, landing pages). Tracked.
- `scratch/` — gitignored staging area for non-Spacetime debris.
- `stdlib/`, `examples/`, `docs/` — Spacetime toolchain assets, tracked in this repo.
- `spacetime init <name>` from this repo root creates the project at `projects/<name>/` automatically when `projects/` exists.