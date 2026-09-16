/// BUG-329 cutover: `@on &.<event> { $cell <- … }` is the ONE event spelling.
///
/// BEHAVIOUR gates by necessity. Grepping emitted JS for `addEventListener('click'`
/// proves nothing here: the handler registers through a variable
/// (`const eventType = "click"; el.addEventListener(eventType, …)`), so a source
/// assertion reports "dead" for working code — it did during this investigation,
/// and the retraction cost a full detour. Only clicking settles it.
///
/// These ran GREEN against the bare spelling before the cutover, which is what
/// licensed migrating 175 call sites: equivalence was proven, not assumed.

@import "stdlib/testing/test"

@test "@on &.click wires a listener and mutates the cell" {
  @fixture {
    <button class="bug329-inc">go</button>
  }
  @given .bug329-inc {
    @on &.click {
      $bug329Count <- $bug329Count + 1;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.bug329Count = 0)

  @when .bug329-inc click
  @wait 50ms
  @assert (window.SpacetimeLocal.bug329Count === 1)
    "@on &.click must increment the cell exactly once";
}

/// The position where the spelling actually mattered: a selector scope, not a
/// `@given` block. Both spellings were pinned here before the cutover.
@test "@on &.click fires from a selector scope" {
  @fixture {
    <button class="ss-sigil">go</button>
  }
  @given .ss-sigil {
    @on &.click { $ssSigil <- 1; }
  }
  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.ssSigil = 0)
  @when .ss-sigil click
  @wait 50ms
  @assert (window.SpacetimeLocal.ssSigil === 1)
    "the sigil spelling must fire in selector scope";
}
