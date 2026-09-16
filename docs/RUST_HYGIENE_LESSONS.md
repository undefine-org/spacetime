# Rust Hygiene Report — What Would Have Prevented This Cleanup

**Context:** `verse`'s `src/` crate accumulated 471 `clippy --all-targets`
warnings (and a handful of genuinely broken pre-existing example binaries)
by the time this cleanup pass ran. This document explains *why* that
backlog formed and what concrete, low-cost process changes prevent it from
recurring — written for a maintainer, not an AI agent, so it's prose and
recommendations rather than an audit log.

## 1. CI never gated on `clippy --all-targets`

The single biggest root cause: **nothing in CI failed the build on new
clippy warnings.** If it had, none of the 471 could have accumulated one
at a time.

**Fix:** add a CI job that runs

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

and gates merge on it (`commercial/host`'s workspace was already
completely clean under exactly this invocation — proof it's practical to
maintain zero warnings in a Rust codebase this size when it's enforced
from day one, vs. `verse`'s 471 when it isn't). Retrofit note: the day
this gate is turned on, first run it with `--all-targets` to fix the
current backlog once (this session did that part), then never let it
regress.

`--all-features` matters too — this repo has a `headless` feature whose
own code path had a duplicated `#[cfg(feature = "headless")]` attribute
that neither a default-features build nor a `--lib`-only clippy run would
ever surface. A gate that doesn't sweep every feature combination misses
whole code paths.

## 2. `--lib`-only warnings are a trap for test-only imports

**The concrete bug this caused:** `src/pipeline/resolve.rs` had a
top-level `use crate::parser::meta_ast::{..., MacroBodyItem,
MetaIfCondition, ...};` used ONLY by 32 tests inside that file's own
`#[cfg(test)] mod tests`. Under `cargo build --lib` (or `clippy --lib`),
`#[cfg(test)]` code is stripped entirely, so the import looks dead. Under
`cargo build --tests`, it's genuinely used. `cargo clippy --fix --lib`
mechanically deleted it based on the WRONG compilation unit's warning —
twice, even after a first manual fix, because a later `--fix --tests` run
still saw the file's top-level import as "unused from --lib's
perspective" during its own multi-target sweep.

**Why this matters for hygiene, not just this one incident:** *any*
import that's used exclusively by test code but declared at file
top-level is silently living in this trap. It will pass `cargo build`
(nobody runs `--lib` alone routinely) but will be a landmine the moment
someone runs an automated "unused import" cleanup tool scoped too
narrowly.

**Fix, as a standing rule:** an import used *only* by
`#[cfg(test)] mod tests { ... }` code belongs *inside* that module, not
at file top-level. `use super::*;` at the top of `mod tests` already
brings in everything from the outer scope that the outer scope itself
declared — a test-only dependency should be its own explicit `use` line
inside `mod tests`, right next to the tests that need it. This is not
just cosmetic: it makes the import's *only* consumer immediately visible
in a diff, and it structurally cannot be mistaken for dead code by any
tool that only compiles one target.

**How to check for this class of bug across the whole repo in one
command**, if you want to audit the rest of the codebase for the same
landmine before it bites again:

```sh
cargo clippy --lib --message-format=json 2>/dev/null \
  | jq -r 'select(.message.message | startswith("unused import")) | .message.spans[0].file_name' \
  | sort -u > /tmp/lib_unused.txt
cargo clippy --tests --message-format=json 2>/dev/null \
  | jq -r 'select(.message.message | startswith("unused import")) | .message.spans[0].file_name' \
  | sort -u > /tmp/tests_unused.txt
comm -23 /tmp/lib_unused.txt /tmp/tests_unused.txt
```

Any file that appears under `--lib` but NOT under `--tests` has an
import that's real (test code needs it) but *looks* dead from a
`--lib`-only vantage point — exactly the shape of bug this section
describes.

## 3. Never run `cargo clippy --fix` (or any auto-fix) without `--all-targets`

Related to #2 but a standalone rule on its own: if you ever run
`cargo clippy --fix` by hand, **always pass `--all-targets`** (or
scope it to `--tests` at minimum). `--fix --lib` alone is a footgun for
exactly the reason above — it happily "fixes" a warning that's only true
for the narrower compilation unit it's currently building, and there is
no confirmation step before it writes the file.

**Standing rule:** `cargo clippy --fix --all-targets --allow-dirty
--allow-staged`, always, or don't use `--fix` at all and apply
suggestions by hand from `cargo clippy --all-targets`'s output.

## 4. `cargo clippy --fix` needs verification *between* every pass, not just at the end

This session ran `--fix` three separate times, and it silently
mis-corrected something on the first AND second run (see #2). The
pattern that caught both: **`cargo check --tests` after every single
`--fix` invocation, before running `--fix` again.** `check` is orders of
magnitude faster than `test` and catches every case where a fix broke
something the type system can see (which import-removal always is,
since a missing type resolves to a compile error, not a silent runtime
bug).

**Standing rule for any future mechanical cleanup:**
1. `cargo clippy --fix --all-targets --allow-dirty`
2. `cargo check --all-targets` immediately after — if it errors, STOP
   and hand-fix before running `--fix` again (never chain a second
   automated pass on top of an already-broken tree)
3. Only once `check` is clean, consider running the fix tool again for
   a subsequent category of warnings

## 5. Dead code accumulates because nothing removes it — `#[warn(dead_code)]` needs a policy, not just a lint

At the end of this cleanup, ~30 warnings remained, and roughly a third
of those are genuine dead code (`parse_component_body`,
`validate_component_body`, `is_stdlib_path`,
`Origin::from_source_file`, `EDITABLE_JS`, an unused enum variant,
`emit_element_root_scoped`). None of these are false positives — they're
real unused functions/constants that should either be deleted or
(if intentionally kept for a near-future feature) marked
`#[allow(dead_code)]` with a comment explaining why.

**Fix:** dead code should be triaged, not just silenced. A
`#![warn(dead_code)]`-clean codebase requires periodically asking "is
this genuinely needed" rather than reflexively adding `#[allow(...)]`
everywhere a warning appears — the latter defeats the lint's entire
purpose and is how a codebase ends up with 500+ warnings again in a
year. Recommend a lightweight recurring practice: whenever `dead_code`
fires, either delete the code in the SAME PR that introduced it as dead
(most common case — a refactor left an old helper behind) or add an
explicit `#[allow(dead_code)] // kept for <reason>, tracked in <issue>`
if it's deliberately staged for later use.

## 6. `clippy::approx_constant` is a real trap for numeric test fixtures

Six separate test files in this repo used `3.14` as an arbitrary "some
decimal number" test value (verifying number-literal parsing / JS
emission / value classification — nothing to do with π). Clippy's
`approx_constant` lint (which defaults to `deny`/`error` severity, not
merely `warn`) flags ANY literal close to a well-known math constant,
regardless of intent.

**Fix, as a repo-wide convention going forward:** when a test genuinely
needs "some arbitrary decimal," prefer a value that isn't close to π/e/τ
etc. — e.g. `2.5`, `1.75`, `9.99` — precisely to avoid ever needing an
`#[allow(clippy::approx_constant)]` escape hatch. It costs nothing at
write time and avoids the lint entirely rather than suppressing it after
the fact. (Where the escape hatch IS necessary — this session added six
of them — always pair it with a one-line comment stating the value is
NOT meant to approximate the constant, so a future reader isn't left
guessing whether it's a suppressed real bug.)

## 7. Duplicated attributes (`#[cfg(test)]` twice, `#[cfg(feature = "...")]` twice) are a copy-paste smell with no compiler safety net

Two files had a literal duplicated attribute
(`#[cfg(test)] #[cfg(test)] mod ...`, and
`#[cfg(feature = "headless")]` appearing twice around a doc comment in
between). Rust's compiler does NOT reject this — a duplicate `cfg`
attribute is silently harmless (redundant, not an error), so it can sit
in the tree indefinitely without ever surfacing as a build failure. Only
`clippy::duplicated_attributes` catches it, and only if clippy is
actually run.

**Fix:** this is a `clippy --all-targets -D warnings` CI gate problem
again (#1) — it's the single change that would have caught this the day
it was introduced, rather than 471 warnings later.

## Summary: the one change that would have prevented all of this

If a single CI job had run

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

on every PR from early on, none of the above could have accumulated
past a single commit — each of these seven categories would have been a
same-day fix by whoever introduced it, with full context, instead of a
471-warning archaeological cleanup months or years later. `commercial/host`
(a newer part of this same monorepo) already runs clean under exactly
this invocation, which is the concrete existence proof that this is
achievable and sustainable for a Rust codebase of this shape and size —
it just needs to be enforced, not merely available as a command someone
could theoretically run.
