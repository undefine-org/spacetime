/// File audio follows the same seek/play/pause transport as picture and synth.
@import "stdlib/testing/test";
.transport-audio {
  @score &.time(6s) as &audiofilm {
    @audio(src: "data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEAESsAACJWAAACABAAZGF0YQAAAAA=") as &bed at 1s;
    &transport-picture at 0s for 6s;
  }
}
.transport-picture { @on &.clip { opacity: 0 -> 1; } }
@test "file audio is silent during a paused seek and follows play/pause" needs timing {
  @mount { <div class="transport-audio"><div class="transport-picture">Picture</div></div> }
  @wait 150ms
  @eval (window.__t = ST._transports.audiofilm, window.__a = document.querySelector('.transport-audio audio'), window.__plays = 0, window.__pauses = 0)
  @assert (!!document.querySelector('.transport-audio')._stStageObjects.bed) "the authored audio name addresses its channel proxy"
  // Instrument the device boundary: the transport contract must not depend on
  // autoplay policy or on the test machine having an audio device.
  @eval (window.__a.play = function() { window.__plays++; return Promise.resolve(); }, window.__a.pause = function() { window.__pauses++; })
  @eval (window.__t.pause(), window.__t.seek(3000))
  @wait 80ms
  @assert (window.__plays === 0) "a paused seek may position the recording but must not ask it to play"
  @assert (Math.abs(window.__a._stSeekTarget - 2) < 0.001) "audio source time is score time minus its at offset"
  @eval (window.__t.play())
  @wait 150ms
  @assert (window.__plays > 0) "resuming the existing transport starts its file audio"
  @eval (window.__n = window.__plays, window.__t.pause())
  @wait 100ms
  @assert (window.__pauses > 0) "pausing the transport must silence the file immediately"
  @assert (window.__plays === window.__n) "paused audio cannot restart itself"
  @eval (window.__t.seek(500))
  @wait 80ms
  @assert (window.__a._stSeekTarget < 0) "a backward seek before the clip restores its waiting state"
  @assert (window.__plays === window.__n) "waiting before the clip remains silent"
}
