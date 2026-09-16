/// gh-20 — raw CSS @font-face silently vanishes. Logic-rung gate.
///
/// The issue: a standard top-level `@font-face { font-family: "Test Font"; … }`
/// compiles clean but produces ZERO output — the rule is dropped at compile time
/// (evidence.txt: built CSS is only `body { font-family: "Test Font", sans-serif; }`;
/// `inspect --layer emit` shows no trace of the @font-face). Spacetime's real
/// macro is `@font("Family") { … }`, not `@font-face`.
///
/// The behavioral claim — "@font-face reached the stylesheet and registered the
/// family" — is CSSOM-dependent, and the headless logic backend cannot provide
/// it:
///   - `document.styleSheets` is undefined headless → the stylesheet rule list
///     (where a CSSFontFaceRule would appear) is not inspectable;
///   - `document.fonts` (FontFaceSet) is undefined headless → family registration
///     / `fonts.check(...)` is unavailable;
///   - `getComputedStyle(el).fontFamily` headless is a broken stub that just
///     mirrors the inline style — it cannot tell whether the family actually
///     resolved.
/// This file asserts those rung gaps explicitly so the deferral to the real gate
/// (`font-face.cdp.test.st`, needs layout) is auditable. The @font-face will be
/// SUPPORTED (per product ruling), so the cdp gate is written to go GREEN the
/// moment that lands.
@import "stdlib/testing/test"

@test "logic rung: @font-face registration is not assertable headless" {
  @fixture {
    <h1 class="font20">Hello</h1>
  }
  @eval (document.querySelector('.font20').style.fontFamily = 'Test Font, sans-serif')
  // RUNG GAP 1: no stylesheet object → cannot inspect for a CSSFontFaceRule.
  @eval (window.__sheets = typeof document.styleSheets)
  @assert (window.__sheets === 'undefined')
    "headless has no document.styleSheets — whether a @font-face rule reached the stylesheet is not assertable at the logic rung (deferred to font-face.cdp.test.st, --cdp)"
  // RUNG GAP 2: no FontFaceSet → cannot check family registration / fonts.check().
  @eval (window.__fonts = typeof document.fonts)
  @assert (window.__fonts === 'undefined')
    "headless has no document.fonts — family registration is not assertable at the logic rung (deferred to --cdp)"
  // getComputedStyle headless is a stub that mirrors the inline style, so it
  // cannot prove the family resolved to a registered @font-face — only that the
  // author declared it. Asserting the declaration is not the bug.
  @eval (window.__gcs = getComputedStyle(document.querySelector('.font20')).fontFamily)
  @assert (window.__gcs === 'Test Font, sans-serif')
    "author declared font-family is mirrored (getComputedStyle stub), which proves nothing about @font-face resolution — the stylesheet claim needs CSSOM (--cdp)"
}
