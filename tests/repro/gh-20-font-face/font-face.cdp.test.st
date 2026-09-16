/// gh-20 — RAW @font-face REACHES THE BROWSER STYLESHEET (REQUIRES `--cdp`).
///
/// The issue: standard CSS `@font-face { font-family: "Test Font"; … }` compiled
/// clean but VANISHED — no CSS, no IR, plus a misleading W0714 "not a known
/// directive" (because `@font-face` is the macro NAME whose `%form` is the
/// `@font("Family")` spelling). Product ruled: the host CSS spelling must WORK.
///
/// This is the true behavioral gate. `check-contract.sh` asserts the emitted
/// stylesheet TEXT contains the rule; only a real browser can prove the bytes
/// parse as a CSSFontFaceRule and the declared family survives the round trip.
///
/// GATE-DESIGN NOTE (2026-08-06): this file previously declared no `@font-face`
/// of its own and scanned `document.styleSheets` after mounting a `@fixture`.
/// That asserted nothing about the fix: a test file's fixture document carries
/// THIS file's compiled CSS, not the sibling `index.st`'s, so the scan looked in
/// a document where the rule was never supposed to appear and failed for a
/// reason unrelated to @font-face support. A gate must exercise the artifact it
/// claims to test — so the rule is declared HERE, and the assertion reads the
/// document that rule was compiled into.
@import "stdlib/testing/test"

@font-face {
  font-family: "Test Font";
  src: url("data:font/woff2;base64,") format("woff2");
  font-style: normal;
  font-weight: 400;
}

@test "a raw @font-face reaches the browser as a CSSFontFaceRule" needs layout {
  @fixture {
    <h1 class="font20c">Hello</h1>
  }
  // type === 5 is CSSFontFaceRule. Cross-origin sheets throw on .cssRules, so
  // each sheet is probed defensively — a sheet we cannot read is not a failure,
  // only a sheet that SHOULD carry the rule and does not.
  @assert (Array.from(document.styleSheets).some(s => {
    try {
      return Array.from(s.cssRules).some(r => r.type === 5 && String(r.style.fontFamily).indexOf('Test Font') !== -1);
    } catch (e) { return false; }
  })) "the raw @font-face block must reach the browser stylesheet and declare 'Test Font'"
}
