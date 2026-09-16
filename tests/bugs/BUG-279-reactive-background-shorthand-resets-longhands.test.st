/// Behavior gate: a reactive `background:` must not wipe its static longhands (BUG-279)
///
/// A declaration carrying a `$`-reference is deliberately NOT emitted into
/// spacetime.css (BUG-091): it becomes a runtime
/// `node.style.setProperty("background", …)`. But `background` is a SHORTHAND,
/// and setting a shorthand resets EVERY background longhand to its initial
/// value — at INLINE specificity, which outranks the static stylesheet. So the
/// author's `background-size` and `-webkit-background-clip` from the same rule
/// are destroyed the moment the signal renders.
///
/// The visible failure: a gradient-clipped headline (`background-clip: text`)
/// renders as a solid slab and the text disappears, because the clip fell back
/// to `border-box`. Seen live in demos/score-launch scene 4.
///
/// WHY `--cdp`: the whole defect lives in the CASCADE — inline shorthand vs
/// stylesheet longhand. `getComputedStyle` is the only place that resolution is
/// real, so these claims are `needs layout`. A source assertion on the emitted
/// JS (`js.contains("setProperty(\"background-image\"")`) proves a string was
/// written; it cannot see a longhand being reset. The Rust contract test in
/// tests/integration/reactive_properties.rs pins WHICH property is emitted;
/// this file pins that the page still looks right.

@import "stdlib/testing/test"

@data inline $brand : { "ink": "#101018", "accent": "#f0c060" };

.shimmer {
  background: linear-gradient(100deg, $brand.ink 42%, $brand.accent 50%, $brand.ink 58%);
  background-size: 260% 100%;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  font-size: 40px;
}

// A control that never touched a shorthand: the same statics, a reactive
// LONGHAND. It passed before the fix and must keep passing after — if it ever
// goes red, the fix broke the ordinary reactive path rather than the shorthand.
.shimmer-longhand {
  background-image: linear-gradient(100deg, $brand.ink 42%, $brand.accent 50%, $brand.ink 58%);
  background-size: 260% 100%;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  font-size: 40px;
}

// =============================================================================
// 1. THE RESET — the assertion that failed before the fix. `background-clip`
//    read `border-box` (initial) instead of the authored `text`.
// =============================================================================

@test "reactive background shorthand does not reset background-clip" needs layout {
  @fixture {
    <h1 class="shimmer">SPACETIME</h1>
  }

  @wait 250ms
  @eval (window.__clip = getComputedStyle(document.querySelector('.shimmer')).backgroundClip || getComputedStyle(document.querySelector('.shimmer')).webkitBackgroundClip)
  @assert (window.__clip === 'text')
    "the rule authored `background-clip: text` in the SAME block as a reactive `background:`; a computed `border-box` means the inline shorthand reset the longhand and the gradient now paints the whole box instead of the glyphs"
}

// =============================================================================
// 2. THE SECOND LONGHAND — background-size. Without this, a fix that only
//    rescued background-clip would look complete.
// =============================================================================

@test "reactive background shorthand does not reset background-size" needs layout {
  @fixture {
    <h1 class="shimmer">SPACETIME</h1>
  }

  @wait 250ms
  @eval (window.__size = getComputedStyle(document.querySelector('.shimmer')).backgroundSize)
  @assert (window.__size.indexOf('260%') === 0)
    "the rule authored `background-size: 260% 100%`; a computed `auto` means the inline shorthand reset it, and the shimmer sweep has no over-width gradient left to travel across"
}

// =============================================================================
// 3. THE GRADIENT STILL PAINTS. Without this, "emit nothing at all" would
//    satisfy tests 1 and 2 — the longhands survive precisely because nothing
//    reactive happened. The signal's value must still reach the element.
// =============================================================================

@test "the reactive gradient still applies" needs layout {
  @fixture {
    <h1 class="shimmer">SPACETIME</h1>
  }

  @wait 250ms
  @eval (window.__img = getComputedStyle(document.querySelector('.shimmer')).backgroundImage)
  @assert (window.__img.indexOf('gradient') !== -1)
    "the reactive value is a linear-gradient built from $brand.ink/$brand.accent; a computed `none` means the binding stopped applying — the longhands would then survive for the wrong reason"
}

// =============================================================================
// 4. THE CONTROL — the documented workaround path (`background-image:`) must be
//    unaffected by the fix.
// =============================================================================

@test "CONTROL: a reactive background-image longhand keeps its statics" needs layout {
  @fixture {
    <h1 class="shimmer-longhand">SPACETIME</h1>
  }

  @wait 250ms
  @eval (window.__lClip = getComputedStyle(document.querySelector('.shimmer-longhand')).backgroundClip || getComputedStyle(document.querySelector('.shimmer-longhand')).webkitBackgroundClip)
  @eval (window.__lImg = getComputedStyle(document.querySelector('.shimmer-longhand')).backgroundImage)
  @assert (window.__lClip === 'text' && window.__lImg.indexOf('gradient') !== -1)
    "a reactive LONGHAND resets nothing and must keep working exactly as before — this is the control that separates 'the shorthand fix works' from 'the reactive CSS path broke'"
}

// =============================================================================
// 5. MULTI-LAYER (FUP-184). CSS stacks background layers with commas, and a
//    reactive multi-layer value hit the SAME reset: `image_core` matched one
//    image, so `url(…), linear-gradient(…)` matched NEITHER arm of
//    `background_route` and fell through as an unrouted shorthand.
//
//    The trap this pins: `linear-gradient(#101018, #f0c060)` CONTAINS a comma
//    but is ONE layer. If the list production split on every comma instead of
//    respecting `balanced(')')`, this value would be read as four broken
//    layers and the gradient would not paint at all — which test 7 catches.
// =============================================================================

.layered {
  background: linear-gradient(180deg, rgba(16,16,24,0.82) 0%, rgba(16,16,24,0.4) 100%), linear-gradient(100deg, $brand.ink 0%, $brand.accent 50%, $brand.ink 100%);
  background-size: 100% 100%, 260% 100%;
  background-repeat: no-repeat, no-repeat;
  color: #f6f7fb;
  font-size: 40px;
}

@test "a reactive MULTI-LAYER background does not reset its longhands" needs layout {
  @fixture {
    <h1 class="layered">SPACETIME</h1>
  }

  @wait 250ms
  @eval (window.__mSize = getComputedStyle(document.querySelector('.layered')).backgroundSize)
  @assert (window.__mSize.indexOf('260%') !== -1)
    "the rule authored `background-size: 100% 100%, 260% 100%` beside a reactive two-layer `background:`; a computed `auto` means the inline shorthand reset it — the same BUG-279 defect, for the value shape FUP-184 left unclaimed"
}

@test "a reactive MULTI-LAYER background still paints BOTH layers" needs layout {
  @fixture {
    <h1 class="layered">SPACETIME</h1>
  }

  @wait 250ms
  @eval (window.__mImg = getComputedStyle(document.querySelector('.layered')).backgroundImage)
  @eval (window.__mCount = (window.__mImg.match(/gradient/g) || []).length)
  @assert (window.__mCount === 2)
    "both layers must survive routing: exactly two gradients should compute. 0 means the value stopped applying (so test 5 would pass for the wrong reason); 1 means a layer was dropped; more means a comma INSIDE a gradient split a layer — the balanced-args trap"
}

// =============================================================================
// 6. LAYER-COUNT PARITY. The static longhand lists ONE size per layer. Routing
//    to `background-image` must keep that correspondence — if the layers and
//    their sizes drift apart, CSS repeats the shorter list and the composite
//    silently renders wrong even though both earlier claims pass.
// =============================================================================

@test "layer count and the static per-layer longhand stay in correspondence" needs layout {
  @fixture {
    <h1 class="layered">SPACETIME</h1>
  }

  @wait 250ms
  @eval (window.__pImg = getComputedStyle(document.querySelector('.layered')).backgroundImage)
  @eval (window.__pSize = getComputedStyle(document.querySelector('.layered')).backgroundSize)
  @eval (window.__pLayers = (window.__pImg.match(/gradient/g) || []).length)
  @eval (window.__pSizes = window.__pSize.split(',').length)
  @assert (window.__pLayers === 2 && window.__pSizes === 2)
    "two image layers must line up with two authored sizes; a mismatch means the routed value and the static per-layer longhand disagree, and CSS will cycle the shorter list against the longer one"
}

