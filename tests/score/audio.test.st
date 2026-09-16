/// Behavior gate: PLAN-150 W7 — @audio on the score.
/// The "audio is on the score" of docs/language/film.st.md §8.
///
/// `@audio(src:) as &bed at 0s;` puts sound on the score's ONE transport: the
/// score creates an `<audio>` element that SEEKS to follow the score's own
/// progress signal (the score is truth; the audio never runs its own clock).
/// This asserts the BEHAVIOUR — the audio element exists with the right source
/// and its currentTime tracks the score's progress as the clock advances
/// (BUG-252), not the emitted string.
///
/// A real audio device isn't needed: we assert the SEEK (currentTime), which the
/// transport drives whether or not playback is audible under the autoplay
/// policy. FIDELITY: timing rung (`--cdp`) under the score clock.

@import "stdlib/testing/test"

.stage {
  @score &.time(8s) as &film {
    @audio(src: "data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEAESsAACJWAAACABAAZGF0YQAAAAA=") as &bed at 0s;
    &open at 0s for 8s;
  }
}
.open { @on &.clip { opacity: 0 -> 1; } }

@test "@audio creates an element that seeks with the score" needs timing {
  @fixture { <div class="stage"><div class="open">shot</div></div> }

  @wait 150ms
  @eval (window.__audio = document.querySelector('.stage audio'))
  @eval (window.__hasAudio = !!window.__audio)
  @eval (window.__src = window.__audio ? window.__audio.getAttribute('src') : '')
  // The seek TARGET the transport derives (currentTime itself clamps to the
  // clip's real duration; the target is the score-derived playhead).
  @eval (window.__ct0 = window.__audio ? (window.__audio._stSeekTarget || 0) : -1)
  @wait 3000ms
  @eval (window.__ct1 = window.__audio ? (window.__audio._stSeekTarget || 0) : -1)

  @assert (window.__hasAudio === true)
    "@audio must create an <audio> element on the score — no element means the audio entry never lowered"
  @assert (window.__src.indexOf("data:audio") === 0)
    "the audio element must carry the entry's src"
  @assert (window.__ct1 > window.__ct0 + 0.3)
    "the audio's currentTime must ADVANCE as the score's clock advances (the audio seeks to follow the ONE transport) — a frozen currentTime means the audio never bound to the score progress"
}
