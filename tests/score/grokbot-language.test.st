/// The shared language's contract, exercised by Chromium. Run once with the
/// movement declarations absent (RED), then with the same forms used by the
/// film (GREEN). No assertion on emitted source can stand in for these.
@import "stdlib/testing/test";
@import "../../demos/film-grokbot/design/index.st";

.gate-reel { position: relative; width: 1000px; height: 400px; @score &.time(4s) as &gate { &gate-move at 0s for 4s; &gate-strip at 0s for 4s; } }
.gate-move { @on &.clip { --film-arrive; } position: absolute; width: 80px; height: 80px; background: #168bfa; }
.gate-strip { @on &.clip { .filmstrip { --frame: 0 -> 119; } } position: absolute; left: 200px; top: 100px; width: 100px; height: 80px; overflow: hidden; --frame: 0; }

@test "film language: arrival and filmstrip move, settle and seek backward exactly" needs timing {
  @mount { <div class="gate-reel"><div class="gate-move"></div><div class="gate-strip"><--filmstrip src="" alt="Atlas cell position probe" aspect="1.25" /></div></div> }
  @wait 100ms
  @eval (window.__g = ST._transports.gate, window.__g.pause(), window.__g.seek(80))
  @wait 30ms
  @eval (window.__first = document.querySelector('.gate-move').getBoundingClientRect().top, window.__blur = getComputedStyle(document.querySelector('.gate-move')).filter)
  @eval (window.__g.seek(1600))
  @wait 30ms
  @eval (window.__last = document.querySelector('.gate-move').getBoundingClientRect().top, window.__sharp = getComputedStyle(document.querySelector('.gate-move')).filter)
  @assert (window.__first - window.__last > 30) "arrival must physically move the element in Chromium"
  @assert (parseFloat(window.__blur.replace('blur(', '')) > 1) "a moving arrival carries its reusable blur treatment"
  @assert (window.__sharp === 'none' || parseFloat(window.__sharp.replace('blur(', '')) < 0.1) "the arrival settles sharply"
  @eval (window.__g.seek(80))
  @wait 30ms
  @assert (Math.abs(document.querySelector('.gate-move').getBoundingClientRect().top - window.__first) < 0.1) "reverse seek shows the same frame"
  @eval (window.__g.seek(1000))
  @wait 100ms
  @eval (window.__a = document.querySelector('.gate-strip img').getBoundingClientRect().top)
  @eval (window.__g.seek(3000))
  @wait 100ms
  @eval (window.__b = document.querySelector('.gate-strip img').getBoundingClientRect().top)
  @assert (Math.abs(window.__a - window.__b) > 400) "the score changes the actual displayed atlas row"
  @eval (window.__g.seek(1000))
  @wait 100ms
  @assert (Math.abs(document.querySelector('.gate-strip img').getBoundingClientRect().top - window.__a) < 0.1) "atlas addressing is a pure function of the playhead"
}
