/// Behavior gate: PLAN-150 W5 — @scatter stamps positions sampled from a shape.
/// The "positions from a shape" of docs/language/film.st.md §6.
///
/// @scatter samples the rendered glyph coverage of a source and stamps per-child
/// custom properties naming the sampled point. This asserts the BEHAVIOUR — the
/// children receive DISTINCT positions that fall within the source's ink region
/// (BUG-252), not the emitted string. A generic burst would stamp nothing (all
/// children identical / unset); a real sampler stamps a spread of points over
/// the glyphs.
///
/// FIDELITY: paint rung (`--cdp`); needs real layout + a 2D canvas coverage pass.

@import "stdlib/testing/test"

.burst {
  position: relative;
  width: 400px; height: 120px;
  @scatter(from: &tag, count: 60) {
    :shape { --x: sample-x; --y: sample-y; }
  }
}
.tag { font: 700 96px sans-serif; position: absolute; left: 0; top: 0; }
.burst i { position: absolute; width: 3px; height: 3px; translate: var(--x) var(--y); }

@test "scatter stamps distinct sampled positions from the source glyphs" needs layout {
  @fixture {
    <div class="burst">
      <span class="tag">SCRIPT</span>
      <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
      <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
      <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
    </div>
  }

  // Sampling stamps once the glyph box is measurable — synchronously when layout
  // is already available, else on the next frame. Give it two frames to settle.
  @eval (window.__burst = document.querySelector('.burst'))
  @eval (window.__particles = Array.prototype.slice.call(window.__burst.querySelectorAll('i')))
  @eval (await new Promise(function (r) { requestAnimationFrame(function () { requestAnimationFrame(r); }); }))

  @eval (window.__xs = window.__particles.map(function (el) { return el.style.getPropertyValue('--x'); }))
  @eval (window.__ys = window.__particles.map(function (el) { return el.style.getPropertyValue('--y'); }))
  @eval (window.__stampedX = window.__xs.filter(function (v) { return v && v !== ''; }).length)
  @eval (window.__distinctX = window.__xs.filter(function (v, i, a) { return a.indexOf(v) === i; }).length)
  @eval (window.__nums = window.__xs.map(function (v) { return parseFloat(v); }).filter(function (n) { return !isNaN(n); }))
  @eval (window.__maxX = window.__nums.length ? Math.max.apply(null, window.__nums) : -1)
  @eval (window.__minX = window.__nums.length ? Math.min.apply(null, window.__nums) : -1)

  @assert (window.__stampedX >= 20)
    "every particle must receive a stamped --x — an unfired sampler leaves them unset"
  @assert (window.__distinctX >= 6)
    "the stamped positions must be DISTINCT across particles — identical values mean the sampler collapsed to one point instead of spreading over the glyphs"
  @assert (window.__minX >= 0 && window.__maxX <= 400)
    "sampled x must fall within the source's rendered box (0..400px), i.e. on the ink, not off-canvas"
}
