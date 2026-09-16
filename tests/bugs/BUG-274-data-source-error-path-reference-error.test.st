/// BUG-274: a failing @data fetch crashed its OWN error path with
/// `Uncaught (in promise) ReferenceError: resolvedLocale is not defined`
/// (live report: :3335/brand/), masking the real fetch error.
///
/// Root cause (c51a096e, 2026-04-07 — latent ~4 months): `const resolvedLocale`
/// was declared INSIDE the fetch `else { … }` branch of load()'s try, but read
/// in the `catch` — a different block scope. Any fetch rejection (404, network,
/// parse error) made the error REPORTER throw before it could report.
///
/// Fix (stdlib/primitives/data/source.st): resolve the locale ONCE at load()
/// function scope — the fetch path and the catch reporter read the same binding.
///
/// Gate: store.error is set BEFORE the crash line, so it exists in both worlds
/// and cannot discriminate. The `__ST_DEBUG__.recordError` rail sits AFTER the
/// formerly-crashing line — pre-fix it stays empty, post-fix every failed load
/// records a 'data-source' entry with the resolved src.

@import "stdlib/testing/test"

@data fetch $brokenItems string[] : "/nonexistent-bug-274.json"
@data fetch $locItems string[] : "/data-{locale}.json"

@test "a failing @data fetch completes its error report — no resolvedLocale ReferenceError" {
  @eval (window.__spacetimeData.brokenItems.reload())
  @eval (window.__spacetimeData.locItems.reload())
  @wait 500ms
  @assert (window.__spacetimeData.brokenItems.error.indexOf('No mock') !== -1) "the underlying fetch error must be recorded in the store"
  @assert (JSON.stringify(window.__ST_DEBUG__.getErrors()).indexOf('data-source') !== -1) "the catch must REACH the recordError rail past the resolvedUrl line — pre-fix it crashes on `resolvedLocale is not defined` first"
  @assert (JSON.stringify(window.__ST_DEBUG__.getErrors()).indexOf('/data-en.json') !== -1) "the error report must carry the {locale}-resolved URL"
}
