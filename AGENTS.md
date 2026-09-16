# Git management

- NEVER stash/reset/revert/checkout. If you need to check against the old code, ask for my input on what to do
- ALWAYS keep a record of your work in the org-mode files in @tasks and/or @research/. Colocate the tests + docs + PRD updates in the commits that implemented the work.

# Decisionmaking

- **The language must be learnable by pattern — knowing one thing should tell you the next.**

- Spacetime websites should NOT have custom javascript; they should ONLY rely on Spacetime for everything (style, animations, state management, server communications, etc.). If there is a feature that is legitimately missing, please explain what that feature is and switch to planning mode to design, test, and implement it properly, before continuing on the design of that website.

- **DO NOT write Rust-based parsers/fallbacks for constructs defined in Spacetime's stdlib (like %capture_type).** If stdlib patterns aren't loading, fix the stdlib loading mechanism - don't bypass it with hardcoded Rust implementations. The metasystem is designed to be self-describing.

- **One sigil, one meaning (harmony).** Each author-level sigil has exactly one role — no overloads. `` ` `` is THE hole form everywhere (bare `$x`/`&name` in HTML is literal text); `:` introduces a VALUE and space introduces a TYPE in any param declaration (`$price number`, never `name: type`); `( )` in `%form`/`%capture_type` is a grammar combinator. See docs/language/data-and-rendering.md §2. When adding a surface, reuse the existing sigil meaning — don't overload.
- **Elegance bar (before adding a metasystem mechanism).** A new mechanism earns its place only if it REDUCES net complexity. Adopt it now (vs deferring to a follow-up) only when ALL hold: (1) it's a small extension of EXISTING grammar/registry, no new subsystem; (2) its variants are DATA (registry/stdlib entries), not a Rust match-arm per case; (3) it DELETES at least one existing special-case (a hardcoded enum variant, a bespoke field, a parallel parser); (4) it's testable by the SAME machinery it generalizes (a RED→GREEN test, no bespoke harness). If any fails → file a follow-up, don't fold it in. Corollary: a closed Rust enum whose arms the runtime doesn't all enforce is rot — prefer a registry-keyed reference (kinds-as-data) over growing the enum.

# Debugging

- Enable trace logging with `RUST_LOG=trace cargo run -- serve projects/<name>/` or target specific modules: `RUST_LOG=spacetime::pipeline=trace,spacetime::metasystem=trace`
- Use `cargo run -- {check|inspect --layer <layer>|build}` to better understand how the complier procudes Output

# Instructions

- Always write a descriptive "alt" for the images you include with `<image>`. Read the file for that.

# Commands

- `cargo test --lib` -> unit tests (1574+)
- `cargo test --test integration_tests` -> integration tests (442+)
- `cargo test --test v8_runtime_test` -> JS runtime tests (139)
- `cargo run --features headless -- test tests/ --headless` -> headless Spacetime tests (341+), fast V8 `logic` backend
- `cargo run --features cdp -- test tests/<file>.test.st --cdp` -> REAL Chromium (`layout`/`timing`/`paint`). Required for any claim about what a browser DOES — a `needs layout`/`needs timing` test is SKIPPED (not failed) under `--headless`, so a behavior gate run only headlessly proves nothing. Chromium is auto-discovered (`google-chrome-stable`, `chromium`, `$SPACETIME_CHROME`, …); `cargo run -- doctor <site>` reports what it found.
- `cargo run -- serve projects/<name>/` -> dev server
- `cargo run -- check` -> compile check (also prints the open comments)
- `cargo run -- inspect --layer <layer>` -> layer inspection

## Comments (`//@`) — leave work where it belongs

A comment is a conversation pinned to a place: written inline in `.st` source,
with its status and thread in `<project>/.comments/` (committed to git). Read
**[docs/language/comments.md](docs/language/comments.md)** before using or
changing the feature.

```spacetime
//@todo: replace the placeholder photography
//@bug(severity: high): nav overlaps the CTA below 380px
//@> continuation lines carry the explicit marker
```

- **AS AN AGENT**: `st_comments_list` shows the open work, and every row carries
  its type's `agent_hint` — the follow-up contract travels WITH the work. Claim
  with `st_comments_update(status: "in-progress")`, resolve with a reply saying
  what you did. `st_comments_add` files something you found but should not fix
  unasked.
- Comment TYPES are data (`%comment_type`): stdlib ships six; a project adds its
  own in `<project>/_prelude.st` — THE project overlay, auto-loaded into the
  registry before any page compiles, never served as a page.
- An unmarked `//` line is NEVER absorbed into a comment. Inside HTML markup
  `//` is literal page text, so it is not a comment surface at all.
- Comments are TRIVIA: nothing about them can change emitted output or fail a
  build. Keep it that way — the whole evolution story depends on it.

# Testing & Automation

**Read [docs/testing/README.md](docs/testing/README.md) before writing or changing tests.** Spacetime has its own first-party test + browser-automation infrastructure — one probe vocabulary (`@mount`/`@when`/`@then`/`@clock`/`@drive`), many worlds (unit, animation, live page).

- **[docs/testing/GUIDE.md](docs/testing/GUIDE.md)** — core grammar: `@test`/`@mount`/`@when`/`@then{claims}`/`@let`/`@mock`.
- **[docs/testing/GENERATIVE.md](docs/testing/GENERATIVE.md)** — `@property`/`@fuzz` (generate + shrink from a type).
- **[docs/testing/ANIMATION.md](docs/testing/ANIMATION.md)** — `@clock` virtual time + `@timeline` (deterministic animation).
- **[docs/testing/AUTOMATION.md](docs/testing/AUTOMATION.md)** — `@drive` a live URL with the same probes (test ≡ automation).
- **[docs/testing/FIDELITY_LADDER.md](docs/testing/FIDELITY_LADDER.md)** — why a layout assertion *refuses* on a layout-less backend instead of faking a green.
- **[docs/testing/CDP_BACKEND.md](docs/testing/CDP_BACKEND.md)** — the warm-Chromium backend (`--cdp`).

`--headless` = fast V8 `logic` backend · `--cdp` = real Chromium (`layout`/`timing`/`paint`). Layout/timing/paint assertions refuse on `--headless` by design.

## A source assertion is not a behavior assertion (BUG-252)

**If a feature's job is to MOVE something in a browser, it needs at least one
`--cdp` gate asserting that it moved.** Asserting on emitted JS (`assert!(js.contains("match: \"true\""))`)
proves a string was written, not that anything runs.

This is not a style preference — it is the difference between a green suite and
a working feature. `@on $signal` arms, `@on $sig.change`, `@handle` and
`@effect` all shipped DEAD for file-scope signals, each with passing string
gates, because the strings were emitted correctly into code that returned early,
watched a name that could not exist, or discarded the first change. Three
separate silent breaks, none visible to any assertion in the suite.

Source gates also actively obstruct: one pinned `@effect`'s literal document
listener, so it failed a *correct* refactor while remaining incapable of
detecting a broken subscription.

So: assert the CONTRACT in Rust (which signal, which body, through which shared
helper), and assert the BEHAVIOR on `--cdp` (the element animated, exactly once,
and not on mount). And write the behavior test FIRST — a gate never seen to fail
is not yet a gate.

**A skipped gate reads exactly like a passing one.** `needs layout` / `needs
timing` claims are SKIPPED under `--headless` and the run still exits 0 —
`0 passed, 0 failed, 3 skipped` is what a whole dead file looks like in a green
suite. So a behavior gate is only proven when you have SEEN it run under
`--cdp`, and seen it FAIL against the unfixed code. Anything less is a string
assertion wearing a browser's clothes.

Worse, a headless shim can hide the very defect under test: the V8 backend's
`IntersectionObserver` fires once with `ratio: 1`, which is the only reason
`tests/repro/gh-6-*` passes — BUG-323 (`@on &.visible` frozen mid-animation)
was invisible to it by construction, and shipped into a demo page.

## Docs as a feature (`md` module + `@doc`)

Spacetime renders Markdown to HTML with the **`md` module** (`@import "stdlib/md"`),
so a `.st` page can BE its own documentation — no hand-written HTML, no Rust
markdown parser (AGENTS rule: vendor + own behind a primitive, don't hand-roll).

- `@doc(src: "docs/testing/GUIDE.md")` — the compiler reads the file at BUILD time
  and inlines its bytes; the page renders the canonical doc, so it **cannot drift**
  from the source. Edit the `.md`, rebuild, the page updates.
- `@doc(content: "# inline **markdown**")` — render a literal markdown string.
- Engine: vendored [`snarkdown`](stdlib/md/vendor/) (~2KB IIFE, git submodule,
  demand-injected only on pages that use `@doc`) — same vendoring rail as `stdlib/text`.
- Live example: **[demos/spacetime-docs/](demos/spacetime-docs/)** is a `.st`-only docs
  *reader* that renders all of `docs/testing/*.md` (`cargo run -- serve demos/spacetime-docs/`).

## Literate Spacetime (`.st.md`)

A **`.st.md`** file is a Markdown document whose ` ```st ` fences ARE the
program — prose is prose, code runs where it sits, document order preserved.
Use it when a page's prose and behavior belong together (tutorials, brand
guidelines, runnable specs). See **[docs/language/literate.md](docs/language/literate.md)**.
- Fence flags: ` ```st ` runs · ` ```st src ` runs AND shows the source ·
  ` ```st hidden ` runs silently · any other language is inert prose (that is
  how you quote code without executing it).
- Fences share ONE file scope; the `stdlib/md` import is synthesized; parse
  errors report YOUR `.st.md` line.
- Live: **[demos/starter-pack/index.st.md](demos/starter-pack/index.st.md)** —
  the tutorial IS the program it teaches.
- **Literate vs `@doc(src:)`**: literate hosts prose the page OWNS; `@doc(src:)`
  references a document that belongs elsewhere (canonical spec, standalone
  doc). They compose — don't inline a copy of a file someone else owns.


# IMPORTANT

Read [docs/antipatterns.md] to know what you need to not do


# Project layout

- `projects/` — private Spacetime websites (gitignored; separate git repo). Each `<name>/` builds to `projects/<name>/dist/`, which is gitignored inside `projects/`. Use `cargo run -- serve projects/<name>/`.
- `tests/fixtures/` — compiler integration test fixtures (test-runner UI, landing pages). Tracked.
- `scratch/` — gitignored staging area for non-Spacetime debris.
- `stdlib/`, `examples/`, `docs/` — Spacetime toolchain assets, tracked in this repo.
- `spacetime init <name>` from this repo root creates the project at `projects/<name>/` automatically when `projects/` exists.
