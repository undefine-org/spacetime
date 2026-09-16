/// Behavior gate: PLAN-150 W3 — a camera form drives a DOM rig transform.
/// The film-surface camera of docs/language/film.st.md §4.
///
/// A camera move is motion: `@form camera --crane { position: … }` consumed by
/// `@on &.clip { --crane; }`. Its axes become `--cam-*` custom-property tracks
/// (the W1 engine animates them) and a static rig transform reads them, so the
/// host's contents move as the camera does. This asserts the BEHAVIOUR (the
/// element's computed transform matrix changes across the window as the dolly
/// pulls in), not the emitted string (BUG-252).
///
/// NB `--cdp` under the virtual clock: a computed-transform-at-time claim is
/// timing-dependent; `--headless` refuses by design.

@import "stdlib/testing/test"

@form camera --dolly { position: 0 0 12 -> 0 0 2; }

.reel {
  @score &.time(10s) as &film { &stage at 0s for 10s; }
}
.stage {
  width: 300px; height: 300px;
  @on &.clip { --dolly; }
}

@test "camera: the dolly changes the rig transform across the window" needs timing {
  @fixture { <div class="reel"><div class="stage">shot</div></div> }

  @eval (window.__camMats = [], window.__camt = setInterval(function () {
    window.__camMats.push(getComputedStyle(document.querySelector('.stage')).transform);
  }, 250))

  @wait 5200ms
  @eval (clearInterval(window.__camt))
  @eval (window.__camFirst = window.__camMats.at(0))
  @eval (window.__camDistinct = window.__camMats.filter(function (m, i, a) { return a.indexOf(m) === i; }).length)
  @eval (window.__camNone = window.__camMats.at(0) === 'none')

  @assert (window.__camNone === false)
    "the rig must apply a transform at all — a camera form that emitted no static transform leaves the host at `none`"
  @assert (window.__camDistinct > 3)
    "the rig transform must CHANGE across the window as --cam-z dollies 12 -> 2 — a frozen matrix means the animated custom property never reached the transform"
}

