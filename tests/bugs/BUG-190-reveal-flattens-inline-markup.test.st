/// BUG: `@reveal` (kinetic-typography split primitive,
/// `stdlib/primitives/reveal.st`) FLATTENS and DISCARDS inline markup inside
/// the target element. Any `<br>`, `<em>`, `<a>`, `<span>`, etc. is silently
/// lost the moment the reveal runs, because `splitText()` rebuilds the
/// element purely from `el.textContent` (which collapses every child element
/// to nothing — a `<br>` doesn't even survive as a newline) rather than
/// walking the element's actual child nodes.
///
/// Root cause (confirmed by reading `stdlib/primitives/reveal.st`,
/// `splitText()`, all three split modes):
///   const text = el.textContent;      // inline markup already gone
///   el.innerHTML = '';                // wipe the original DOM
///   // ...rebuild per-char/per-word <span> children from the flat `text`...
/// `originalHTML` (`el.innerHTML`, captured before the wipe) is saved but
/// ONLY ever used to restore on cleanup — never consulted by `splitText()`
/// itself. So a heading like `<h2>NOT PRODUCTS.<br>EXPRESSIONS.</h2>` with
/// `@reveal(split:"words")` renders as `NOT PRODUCTS.EXPRESSIONS.` on one
/// line — the `<br>` never makes it into the rebuilt DOM.
///
/// Impact: silent (no error/warning) markup loss on EVERY split mode
/// (chars/words/lines — all three read `el.textContent` the same way).
/// Blocks a real, common typography pattern: a display heading with a
/// mid-line `<br>` or an `<em>`-emphasized word cannot use `@reveal` without
/// losing that structure. Forces authors to avoid `@reveal` on any heading
/// with line breaks or inline emphasis — a real expressivity limit, not a
/// cosmetic nit.
///
/// TEST-HARNESS NOTE (found while building this pin): `@reveal` must be used
/// in its real, production shape — SELECTOR-SCOPED (`.sel { @reveal(...) }`,
/// as every live call site in `projects/ora-ventures.com` uses it) inside an
/// `@mount` block, NOT the inline-body form the pre-existing
/// `tests/unit/primitives/reveal.test.st` suite uses inside `@fixture`.
/// `@fixture`'s markup is raw innerHTML text — an inline `@reveal(...)` typed
/// directly in that HTML string is NEVER parsed as a directive at all, so
/// every assertion in that pre-existing suite (`.reveal-chars should exist`)
/// passes VACUOUSLY: the primitive never ran, it just checked the static
/// fixture HTML rendered. Confirmed empirically this session: reveal-engine's
/// FIRST line (`window.matchMedia(...)`) throws under the headless V8/
/// LinkeDOM runner (`matchMedia` is unshimmed there — a separate, real gap;
/// tracked in this file's Notes below, not the subject of this bug), which
/// silently aborts the whole primitive body before `splitText()` ever runs —
/// so an inline-body `@reveal` test would report a false green even if
/// `matchMedia` were fixed, because `@fixture` never compiles it as a
/// directive in the first place. This test instead uses `@mount` (which DOES
/// compile nested directives/selectors) with `@reveal` in selector-scoped
/// form, and shims `window.matchMedia` before mounting — verified via a
/// throwaway probe to actually produce `.st-reveal-mask` spans (i.e. the
/// primitive demonstrably RAN) before trusting any assertion about what it
/// preserved or dropped.
///
/// Expected RED until `splitText()` walks child nodes instead of rebuilding
/// from `textContent` (see BUG-190.org).

@import "stdlib/testing/test"

// LinkeDOM (the headless test DOM) does not implement `window.matchMedia`,
// which `reveal-engine`'s very first statement calls unconditionally. Shim it
// so the primitive body actually runs instead of throwing before `splitText()`.
// (This gap is real and impacts every headless reveal test, existing or new —
// see the Notes section at the end of this file.)
@eval (window.matchMedia = window.matchMedia || function(q) { return { matches: false, addListener: function(){}, removeListener: function(){} }; })

@test "reveal(split: words) actually runs (control — proves the primitive executed, not a vacuous pass)" {
    @mount {
        <h2 class="reveal-sanity-check">Hello World</h2>
        .reveal-sanity-check {
            @reveal(split: "words", trigger: load)
        }
    }
    @wait 30ms
    @let (el = document.querySelector('.reveal-sanity-check'))
    @let (maskCount = el.querySelectorAll('.st-reveal-mask').length)
    @assert (maskCount > 0)
        "sanity: reveal must produce .st-reveal-mask spans, proving splitText() ran"
}

@test "reveal(split: words) preserves a <br> inside the target element (RED, BUG-190)" {
    @mount {
        <h2 class="reveal-br-words">NOT PRODUCTS.<br>EXPRESSIONS.</h2>
        .reveal-br-words {
            @reveal(split: "words", trigger: load)
        }
    }
    @wait 30ms
    @let (el = document.querySelector('.reveal-br-words'))
    @let (maskCount = el.querySelectorAll('.st-reveal-mask').length)
    @assert (maskCount > 0) "reveal must have run (sanity)"
    @let (brCount = el.querySelectorAll('br').length)
    @assert (brCount === 1)
        "the <br> element must survive the word split, not be flattened into text"
}

@test "reveal(split: chars) preserves a <br> inside the target element (RED, BUG-190)" {
    @mount {
        <h2 class="reveal-br-chars">NOT PRODUCTS.<br>EXPRESSIONS.</h2>
        .reveal-br-chars {
            @reveal(split: "chars", trigger: load)
        }
    }
    @wait 30ms
    @let (el = document.querySelector('.reveal-br-chars'))
    @let (maskCount = el.querySelectorAll('.st-reveal-mask').length)
    @assert (maskCount > 0) "reveal must have run (sanity)"
    @let (brCount = el.querySelectorAll('br').length)
    @assert (brCount === 1)
        "the <br> element must survive the character split, not be flattened into text"
}

@test "reveal(split: lines) preserves a <br> inside the target element (RED, BUG-190)" {
    @mount {
        <p class="reveal-br-lines">First line here.<br>Second line here.</p>
        .reveal-br-lines {
            @reveal(split: "lines", trigger: load)
        }
    }
    @wait 30ms
    @let (el = document.querySelector('.reveal-br-lines'))
    @let (brCount = el.querySelectorAll('br').length)
    @assert (brCount === 1)
        "the <br> element must survive the line split, not be flattened into text"
}

@test "reveal(split: chars) preserves an <em> inline emphasis element (RED, BUG-190)" {
    @mount {
        <h2 class="reveal-em-chars">This is <em>emphasized</em> text.</h2>
        .reveal-em-chars {
            @reveal(split: "chars", trigger: load)
        }
    }
    @wait 30ms
    @let (el = document.querySelector('.reveal-em-chars'))
    @let (maskCount = el.querySelectorAll('.st-reveal-mask').length)
    @assert (maskCount > 0) "reveal must have run (sanity)"
    @let (emCount = el.querySelectorAll('em').length)
    @assert (emCount === 1)
        "the <em> element must survive the character split, not be flattened into text"
}

@test "reveal(split: words) preserves an <a> link element (RED, BUG-190)" {
    @mount {
        <p class="reveal-a-words">Click <a href="https://example.test">this link</a> now.</p>
        .reveal-a-words {
            @reveal(split: "words", trigger: load)
        }
    }
    @wait 30ms
    @let (el = document.querySelector('.reveal-a-words'))
    @let (aCount = el.querySelectorAll('a').length)
    @assert (aCount === 1)
        "the <a> element must survive the word split, not be flattened into text (link target and click behavior would otherwise be silently lost)"
}

// =============================================================================
// Notes
// =============================================================================
//
// A related, separate gap surfaced while verifying this pin (not BUG-190
// itself, filed here for traceability): `window.matchMedia` has no shim in
// the headless V8/LinkeDOM test backend (`src/headless_shims.js`). Any real
// site page using `@reveal` (or anything else that reads
// `prefers-reduced-motion` via `matchMedia`) is therefore UNTESTABLE headless
// without an author-supplied `@eval` shim like the one at the top of this
// file — and the pre-existing `tests/unit/primitives/reveal.test.st` suite
// (19 tests, all `@fixture` + `should exist`) does not shim it, AND uses the
// non-compiling inline-body form, so none of those 19 tests actually observe
// reveal-engine executing at all. Both gaps are worth their own follow-up
// items; kept out of scope here to keep this pin focused on the inline-markup
// flattening bug itself.
