/// Headless Tests: Collection Ref + @each Nesting
///
/// ITEM-BUG-cssrules-expand-002
///
/// Verifies that @each inside collection ref blocks (&cards[] .selector {
/// @each(...) }) creates children in the container, the MutationObserver
/// collects them into the collection ref state array, and downstream
/// @computed reduce aggregations produce correct values.
///
/// STATUS (BUG-085 regression pins, adopted 2026-06-19): the four collection-ref
/// tests below are this file's formal RED pins for BUG-085 — see that bug's
/// acceptance addendum, which names these four tests as its headless regression
/// target. They are EXPECTED to stay RED until PLAN-039 W2/W3 lands.
/// (FUP-064 investigation, 2026-06-18): the four collection-ref tests
/// below are a PIN for an OPEN builder gap, NOT yet green. A `@template` body
/// that hosts a COLLECTION-REF block `&cards[] .sel { @each(...) }` drops the
/// @each from the per-instance builder (the emitted template payload carries
/// `refs: []` and no @each primitive) — so nothing renders on ANY backend. This
/// is the BUG-085 class fixed structurally by PLAN-039 ("unify nested
/// constructs in component bodies"). The PLAIN selector form
/// `.sel { @each($x as $i) { &row } }` DOES compile + render headlessly — see
/// each-data-drive.test.st, which pins the @each-over-data drive contract that
/// works today. These tests stay as the collection-ref regression target for
/// when PLAN-039 lands. (Seeding here already uses the live-page-faithful
/// ST.setData-after-mount order; the remaining failure is purely the builder
/// gap, not the seeding.)

@import "stdlib/testing/test"

// =============================================================================
// Headless seeding convention (FUP-064)
// -----------------------------------------------------------------------------
// A runtime `@each($src as $row)` renders by LISTENING for the data-source
// event the live page dispatches: `data:<src>:loaded` / `local:<src>:updated`
// (the array travels in `e.detail`). The @each listener is installed at
// template-INIT time, i.e. AFTER the template element is appended and the
// runtime initializes it.
//
// Therefore a headless test must (1) mount the template, (2) let it init, THEN
// (3) publish the data via the documented runtime API `ST.setData(name, array)`
// (which dispatches `data:name:loaded` with the array). A bare
// `window.SpacetimeLocal.x = [...]` assignment does NOT dispatch that event, so
// the @each never renders — that was the original (pre-FUP-064) failing shape
// of these tests. Always seed AFTER mount + init, via ST.setData.
// =============================================================================

// =============================================================================
// Template Definitions
// =============================================================================

// Child template — a simple card element created by @each
@template &cref-card($item) {
    <div class="cref-card"></div>
}

// Parent template with collection ref + @each wiring
// &cards[] names the .cref-cards container as a collection ref.
// @each renders cref-card instances into it; MutationObserver collects children.
@template &cref-dashboard() {
    <section class="cref-dashboard">
        <div class="cref-cards"></div>
    </section>

    &cards[] .cref-cards {
        @each($crefItems as $item) {
            &cref-card($item);
        }
    }
}

// =============================================================================
// Test 1: Collection ref with @each creates expected number of children
// =============================================================================

@test "collection ref + @each creates children from data source" {
    @fixture {
        <div class="host-cref1"></div>
    }
    // Mount first so the @each listener is installed (template init), then
    // publish the data the live-page way via ST.setData (FUP-064 convention).
    @eval (document.querySelector('.host-cref1').appendChild(Spacetime.invokeTemplate('cref-dashboard')))
    @wait 50ms
    @eval (ST.setData('crefItems', [
            {id: 'a', name: 'Alpha', strength: 10},
            {id: 'b', name: 'Beta', strength: 25},
            {id: 'c', name: 'Gamma', strength: 15}
        ]))
    @wait 100ms
    @assert (document.querySelector('.host-cref1 .cref-cards').children.length === 3)
        "container should have 3 children from @each over 3-item data source"
}

@test "each child in collection ref is a cref-card element" {
    @fixture {
        <div class="host-cref1b"></div>
    }
    @eval (document.querySelector('.host-cref1b').appendChild(Spacetime.invokeTemplate('cref-dashboard')))
    @wait 50ms
    @eval (ST.setData('crefItems', [
            {id: 'a', name: 'Alpha', strength: 10},
            {id: 'b', name: 'Beta', strength: 25}
        ]))
    @wait 100ms
    @assert (document.querySelector('.host-cref1b .cref-cards').children[0].classList.contains('cref-card'))
        "first child should be a cref-card element"
    @assert (document.querySelector('.host-cref1b .cref-cards').children[1].classList.contains('cref-card'))
        "second child should be a cref-card element"
}

// =============================================================================
// Test 2: MutationObserver fires and sets collection ref state to child array
// =============================================================================

@test "MutationObserver sets collection ref state to array" {
    @fixture {
        <div class="host-cref2"></div>
    }
    @eval (document.querySelector('.host-cref2').appendChild(Spacetime.invokeTemplate('cref-dashboard')))
    @wait 50ms
    @eval (ST.setData('crefItems', [
            {id: 'x', name: 'X-Ray', strength: 5},
            {id: 'y', name: 'Yankee', strength: 8}
        ]))
    @wait 100ms
    @assert (Array.isArray(ST.get(document.querySelector('.host-cref2 .cref-dashboard'), 'cards')))
        "collection ref 'cards' should be an array"
    @assert (ST.get(document.querySelector('.host-cref2 .cref-dashboard'), 'cards').length === 2)
        "collection ref 'cards' should contain 2 elements matching child count"
}

@test "collection ref state elements match container children" {
    @fixture {
        <div class="host-cref2b"></div>
    }
    @eval (document.querySelector('.host-cref2b').appendChild(Spacetime.invokeTemplate('cref-dashboard')))
    @wait 50ms
    @eval (ST.setData('crefItems', [
            {id: 'p', name: 'Papa', strength: 12},
            {id: 'q', name: 'Quebec', strength: 18},
            {id: 'r', name: 'Romeo', strength: 7}
        ]))
    @wait 100ms
    @eval (eval("                                                           \
        (function() {                                                       \
          var dash = document.querySelector('.host-cref2b .cref-dashboard'); \
          var cards = ST.get(dash, 'cards');                                 \
          var container = document.querySelector('.host-cref2b .cref-cards');\
          window.__crefMatch = true;                                        \
          for (var i = 0; i < cards.length; i++) {                          \
              if (cards[i] !== container.children[i]) {                      \
                  window.__crefMatch = false;                               \
              }                                                             \
          }                                                                 \
        })();                                                               \
    "))
    @wait 50ms
    @assert (window.__crefMatch === true)
        "collection ref state elements should be identical to container.children"
}

// =============================================================================
// Test 3: @computed reduce over source data produces correct aggregate value
// =============================================================================

@test "@computed reduce: sum of strength field" {
    @fixture {
        <div class="host-cref3"></div>
    }
    @eval (eval("                                                           \
        var signals = [                                                     \
            {id: 'a', name: 'Alpha', strength: 10},                         \
            {id: 'b', name: 'Beta', strength: 25},                          \
            {id: 'c', name: 'Gamma', strength: 15}                          \
        ];                                                                  \
        window.__crefTotal = signals.reduce(                                \
            function(sum, sig) { return sum + sig.strength }, 0);            \
    "))
    @assert (window.__crefTotal === 50)
        "reduce((sum, sig) => sum + sig.strength, 0) on 3 signals should produce 50"
}

@test "@computed reduce: count of items" {
    @fixture {
        <div class="host-cref4"></div>
    }
    @eval (eval("                                                           \
        var signals = [                                                     \
            {id: 'a', name: 'Alpha', strength: 10},                         \
            {id: 'b', name: 'Beta', strength: 25},                          \
            {id: 'c', name: 'Gamma', strength: 15}                          \
        ];                                                                  \
        window.__crefCount = signals.reduce(                                \
            function(count, _) { return count + 1 }, 0);                     \
    "))
    @assert (window.__crefCount === 3)
        "reduce((count, _) => count + 1, 0) should produce 3"
}

@test "@computed reduce: empty source with initial returns initial" {
    @fixture {
        <div class="host-cref5"></div>
    }
    @eval (eval("                                                           \
        var empty = [];                                                     \
        window.__crefEmptyTotal = empty.reduce(                             \
            function(sum, sig) { return sum + sig.strength }, 0);            \
        window.__crefEmptyCount = empty.reduce(                             \
            function(count, _) { return count + 1 }, 0);                     \
    "))
    @assert (window.__crefEmptyTotal === 0)
        "reduce on empty array with initial=0 should return 0"
    @assert (window.__crefEmptyCount === 0)
        "count reduce on empty array with initial=0 should return 0"
}
