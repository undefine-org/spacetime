/// GH-37: `@data signal $cursor(x number, y number)` uses BARE param names in the
/// call signature, but param_list only accepts `$binding` / `&element` (typed
/// form `$x number`).
///
/// The bare form is a COMPILE ERROR today (W3 consume-or-refuse): a custom
/// capture (`param_list`) that consumes ZERO tokens from the non-empty paren
/// group fails with E0946 instead of silently emitting `callParams = []`.
/// `tests/repro/gh-37-data-signal-params/check-contract.sh` pins that
/// diagnostic; the bare declaration cannot live in this behavioral file because
/// it would fail to compile (inverted polarity).
///
/// This file pins the CANONICAL spelling's wire contract with the same idiom
/// as tests/integration/signal-body-template.test.st: a REAL compiled
/// `@data signal` is fired via ST.getSignalCall and the outgoing fetch body is
/// captured. `$add($text string)` MUST send a structured `{"text":…}` object
/// (GREEN control — proves the harness + capture idiom work).

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"
@import "stdlib/macros/host"

@host $gh37Api : http("https://gh-37.invalid")

// Control: the CANONICAL typed form `$text string`.
@data signal $add($text string) to $gh37Api {
  send POST "/add" { text: $text }
  receive to AddResult {
    200 => Ok($.body)
    _   => Failed($.statusText)
  }
}

@test "canonical typed @data signal param sends a structured {text} payload" {
  @fixture {
    <div class="g37-add">x</div>
  }

  @eval (window.__capturedBody = null)
  @eval (window.__realFetch = window.__realFetch || window.fetch)
  @eval (window.fetch = function(url, opts) { window.__capturedBody = opts ? opts.body : null; return Promise.resolve({ status: 200, ok: true, statusText: 'OK', headers: {}, text: function() { return Promise.resolve('{}'); } }); })

  // Fire the canonical typed signal.
  @eval ((ST.getSignalCall('add'))('typed-value'))

  // Give the 50ms real-timer fetch mock a tick to land.
  @wait 100ms

  @eval (window.fetch = window.__realFetch)

  @assert (window.__capturedBody !== null)
    "fetch mock must have been invoked (fire reached the host)"
  @assert (window.__capturedBody === '{"text":"typed-value"}')
    "canonical typed param must send a structured {text} payload"
}
