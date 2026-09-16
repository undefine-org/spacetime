# CDP Browser Backend (PLAN-027 W2)

The `cdp` feature provides a **warm Chromium** test backend over the Chrome
DevTools Protocol (via the pure-Rust `chromiumoxide` crate). It supplies the
`layout` / `timing` / `paint` fidelity rungs that the V8 + LinkeDOM backend
cannot — real `getComputedStyle`, `Intl.Segmenter`, Canvas, real `rAF`/layout —
so `@balance`, pretext reflow, visibility and dimension assertions run *for real*
instead of refusing (W1) or faking.

It is a **compiler-crate dependency**, not a vendored `.st` (the gingerbill
`%vendor` path is for `.st`-side JS only). It replaces the removed Playwright
npm-driver crate.

## Usage

```sh
# Run a suite in a warm Chromium (real layout/timing/paint):
cargo run --features cdp -- test tests/unit/runtime/color.test.st --cdp

# Fast V8 logic backend (no browser; layout+ tests refuse — see FIDELITY_LADDER.md):
cargo run --features headless -- test … --headless

# The real-browser integration suite:
cargo test --features cdp --test playwright_st_test
```

## How it works

- **Warm browser, per-test page.** The browser is launched **once** (process-global
  `OnceLock` + a dedicated multi-thread Tokio runtime that drives the CDP `Handler`
  for the browser's lifetime). Each test file runs in a fresh page (tab) and the
  tab is closed afterwards — launch cost is paid once, not per test.
- **Script execution.** Test HTML is staged to a temp file and loaded via a
  `file://` URL with `goto` (not `setContent`, which injects DOM but does not run
  `<script>` tags under CDP; and **not** a top-level `data:` URL, whose opaque
  origin makes modern Chrome refuse to execute the page's inline `<script>`
  blocks). The embedded runtime JS is also `</script`-escaped (`<\/script`) so a
  literal close sequence inside it — e.g. the `<script>alert(1)</script>` example
  in editable.js's sanitizer comment — can't prematurely close the inline
  `<script>` element and truncate the runtime.
- **Direct harness invocation.** The runner calls `window.__spacetime_run_tests()`
  and awaits its promise — no `window.onload` race. Results come from
  `window.__spacetime_test_results`.
- **Backend rung.** The browser-side harness advertises the `paint` rung via
  `ST._rung.setBackend('paint')` so layout+ assertions execute (override with
  `window.__ST_BACKEND_RUNG`).
- **Swappable endpoint.** The same client design connects to a locally-launched
  Chromium today and (W7) to a remote CDP websocket (`via kernel <wsUrl>`,
  kernel.sh `:9222`) with no change to the test-facing surface.

## Chromium provisioning

Settled decision (a) + (c):

- **(a) Local discovery.** `find_chrome()` checks, in order: `SPACETIME_CHROME` /
  `CHROME` env vars, then `which` for `google-chrome-stable` / `google-chrome` /
  `chromium` / `chromium-browser` / `chrome`, then common install paths, then a
  Puppeteer-managed Chromium under `~/.cache/puppeteer/chrome/<rev>/` (highest
  revision wins; Linux + macOS layouts). `cargo run -- doctor <site>` reports
  whether a Chrome was found. No browser is downloaded implicitly.

- **(c) CI / canonical.** Use the kernel-images headless-Chromium Docker image as
  the pinned CI substrate — it speaks the same CDP surface, so local↔CI parity is
  exact. Point the runner at it by exporting `SPACETIME_CHROME` to the in-container
  Chrome, or (W7) by connecting to its `:9222` websocket.

We deliberately do **not** auto-fetch a browser by default (avoids a heavy implicit
download); a doctor-driven opt-in fetch may be added later if friction warrants.

## chromiumoxide version

Pinned to `chromiumoxide` **0.9** (was 0.7). 0.7's CDP type definitions used
`snake_case` field names for some `Page.*` events (e.g.
`Page.frameRequestedNavigation` sends `frameId`, 0.7 expected `frame_id`), so
newer Chrome builds produced `WS Connection error: data did not match any variant
of untagged enum Message` and the handler connection **died** — every subsequent
`evaluate` silently returned nothing and the harness never loaded
("`__spacetime_run_tests` never became available"). 0.9 fixes the event
deserialization. NB: 0.9 dropped the `tokio-runtime` cargo feature (tokio is now
baked into `async-tungstenite`), so the dep is declared with
`default-features = false` and no feature list.

## Removed: multi-engine Playwright parity

The legacy `--browser firefox,chromium,webkit` parity runner was removed with the
Playwright crate. chromiumoxide is Chromium-only, so cross-engine timing parity is
not available through this backend; it is tracked as a follow-up. Use `--cdp` for
real-browser testing and `--headless` for the fast V8 logic backend.
