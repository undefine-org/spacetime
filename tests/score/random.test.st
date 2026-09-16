/// Behavior gate: PLAN-150 W6 — `random()` as a deterministic per-element value.
/// The "randomness as data" of docs/language/film.st.md §7.
///
/// `random()` in a static value is stamped per element, seeded from the element's
/// document index: distinct across elements, independent per stream, and STABLE
/// across builds (so `spacetime render` is bit-identical). This asserts the
/// BEHAVIOUR — the resolved custom properties differ element-to-element and the
/// two streams differ from each other (BUG-252), not the emitted string.
///
/// FIDELITY: paint rung (`--cdp`); needs a real DOM to stamp + read computed
/// custom properties.

@import "stdlib/testing/test"

.dust i {
  --seed: random();
  left: calc(var(--seed) * 100%);
  opacity: calc(0.2 + 0.6 * random(2));
}

@test "random() stamps distinct, deterministic per-element values" needs timing {
  @fixture {
    <div class="dust">
      <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
    </div>
  }

  // The stamp runs on a deferred sweep (after the fixture DOM is in place).
  @wait 200ms
  @eval (window.__motes = Array.prototype.slice.call(document.querySelectorAll('.dust i')))
  @eval (window.__seeds = window.__motes.map(function (el) { return el.style.getPropertyValue('--st-rnd-0'); }))
  @eval (window.__stream2 = window.__motes.map(function (el) { return el.style.getPropertyValue('--st-rnd-2'); }))
  @eval (window.__seedsSet = window.__seeds.filter(function (v) { return v && v !== ''; }))
  @eval (window.__distinctSeeds = window.__seeds.filter(function (v, i, a) { return a.indexOf(v) === i; }).length)
  @eval (window.__seed0 = parseFloat(window.__seeds[0]))
  @eval (window.__s2_0 = parseFloat(window.__stream2[0]))
  @eval (window.__inRange = window.__seedsSet.every(function (v) { var n = parseFloat(v); return n >= 0 && n <= 1; }))

  @assert (window.__seedsSet.length === 8)
    "every mote must receive a stamped --st-rnd-0 seed — an unresolved random() leaves it unset"
  @assert (window.__distinctSeeds >= 6)
    "seeds must DIFFER element-to-element — identical seeds mean random() collapsed to one value instead of being per-element"
  @assert (window.__inRange === true)
    "each seed must be a 0..1 value"
  @assert (Math.abs(window.__seed0 - window.__s2_0) > 0.0001)
    "the two streams (random() and random(2)) must be INDEPENDENT — an equal pair means the stream index was ignored"
}
