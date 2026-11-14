@import "stdlib/testing/test";
@import "../../demos/film-grokbot/design/index.st";
.wave-reel { @score &.time(4s) as &activity { &wave-probe at 0s for 4s; } }
.wave-probe { @on &.clip { .wave { --voice-activity: 0 -> 1 at 50% -> 0; } } }
@test "voice activity grows bars while quiet dots retain their round geometry" needs timing {
  @mount { <div class="wave-reel"><div class="wave-probe"><--wave /></div></div> }
  @wait 100ms
  @eval (window.__wave = ST._transports.activity, window.__wave.pause(), window.__wave.seek(100))
  @wait 80ms
  @eval (window.__quiet = document.querySelector('.wave i:nth-child(8)').getBoundingClientRect())
  @assert (Math.abs(window.__quiet.height - window.__quiet.width) < 0.5) "quiet voice activity is a row of circles, not flattened dashes"
  @eval (window.__wave.seek(2000))
  @wait 80ms
  @assert (document.querySelector('.wave i:nth-child(8)').getBoundingClientRect().height > window.__quiet.height * 3) "the score physically grows the speaking bars"
  @eval (window.__wave.seek(100))
  @wait 80ms
  @assert (Math.abs(document.querySelector('.wave i:nth-child(8)').getBoundingClientRect().height - window.__quiet.height) < 0.1) "reverse seek restores the same quiet dots"
}
