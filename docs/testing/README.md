# Spacetime Testing & Automation

One probe vocabulary, many worlds. A Spacetime program describes behavior; the
test runner executes it against a **world** and asserts. A unit test, an
animation test, and live browser automation differ only in *which world* and
*what you assert* — the directives (`@mount` / `@when` / `@then` / `@clock` /
`@drive`) are the same everywhere.

```spacetime
@import "stdlib/testing/test"

@test "counter increments on click" {
  @mount { <button class="inc" data-st-count="0">+</button> }
  @when .inc click
  @then .inc { state == "1"; }
}
```

Run it:

```sh
cargo run --features headless -- test path/to/file.test.st --headless
```

## The docs

| doc | what |
|-----|------|
| **[GUIDE.md](GUIDE.md)** | the core grammar — `@test` / `@mount` / `@when` / `@then{claims}` / `@let` / `@mock`. Start here. |
| **[GENERATIVE.md](GENERATIVE.md)** | `@property` / `@fuzz` — generate + shrink inputs from a type spec. |
| **[ANIMATION.md](ANIMATION.md)** | `@clock` virtual time + `@record-timeline` / `@timeline` — deterministic animation testing. |
| **[AUTOMATION.md](AUTOMATION.md)** | `@drive` a live URL with the same probes. Test ≡ automation. |
| **[FIDELITY_LADDER.md](FIDELITY_LADDER.md)** | the anti-false-green keystone: a backend that can't provide a rung *refuses* rather than fakes. |
| **[CDP_BACKEND.md](CDP_BACKEND.md)** | the warm-Chromium backend (`--cdp`) that supplies `layout`/`timing`/`paint`. |

## Backends at a glance

| flag | backend | rung ceiling | use for |
|------|---------|--------------|---------|
| `--headless` | V8 + LinkeDOM | `logic` | fast reactivity / state / event tests |
| `--cdp` | warm Chromium (CDP) | `paint` | layout, timing, screenshots, `@drive` |

A `layout`+ assertion (`be_visible`, `have_width`, `@clock`, `@capture
screenshot`) run on `--headless` **refuses to run** instead of passing against a
fake — see [FIDELITY_LADDER.md](FIDELITY_LADDER.md). That refusal *is* the
feature. NB the refusal is reported as **SKIP** and exits 0, so a `--headless`
green says nothing about a `needs layout` file — run those under `--cdp`.

## Running the real browser (`--cdp`)

Chromium is **auto-discovered** — nothing to install or configure if you have
any Chrome/Chromium on PATH:

```sh
# one file, real layout/timing/paint
cargo run --features cdp -- test tests/bugs/BUG-323-on-visible-ratio-freezes-at-one-threshold.test.st --cdp

# a directory
cargo run --features cdp -- test tests/score/ --cdp
```

```
3 passed, 0 failed, 0 skipped (6871ms)     # the same file: 3 SKIPPED under --headless
```

- Discovery order: `$SPACETIME_CHROME` / `$CHROME`, then `google-chrome-stable`
  / `google-chrome` / `chromium` / `chromium-browser` / `chrome` on PATH, then
  common install paths, then a Puppeteer-managed Chromium under
  `~/.cache/puppeteer/chrome/`. `cargo run -- doctor <site>` reports what it
  found. No browser is ever downloaded implicitly.
- The browser launches **once** per run (warm) and each test file gets a fresh
  tab, so the startup cost is paid a single time.
- First run pays a `--features cdp` compile; afterwards it is cached.

See [CDP_BACKEND.md](CDP_BACKEND.md) for the transport, provisioning and
version-pinning details.

## Useful flags

```sh
--rung <pure|logic|layout|timing|paint>   # advertise backend ceiling (default logic)
--coverage                                # report directive (macro-expansion) coverage
--min-directive <pct>                     # fail if directive coverage < pct
--format json                             # machine-readable results
```
