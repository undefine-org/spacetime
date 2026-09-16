/// `@data signal` send-body-template runtime tests (BUG-181 fix).
///
/// Before this fix, `signal.st`'s compiled `fire(payload)` sent the call's
/// RAW positional argument(s) as the JSON request body, completely
/// ignoring the declared `send POST "..." { field: $param }` template --
/// confirmed empirically via a live browser test capturing the real
/// outgoing `fetch` body (documented in BUG-181's own filing). These tests
/// pin the FIXED behavior against a REAL, compiled `@data signal` (not a
/// hand-registered fake fire fn, unlike signal-call-fire.test.st's own
/// `ST.registerSignalCall` tests, which only exercise `runMutations`'
/// by-name DISPATCH, never the body-construction logic this bug lived in).

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"
@import "stdlib/macros/host"

@host $testApi : http("https://signal-body-template-test.invalid")

@data signal $singleField($name string) to $testApi {
  send POST "/single" { fieldName: $name }
  receive to SingleResult {
    200 => Ok($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

@data signal $multiField($a string, $b string) to $testApi {
  send POST "/multi" { fieldA: $a, fieldB: $b }
  receive to MultiResult {
    200 => Ok($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

@data signal $bareBody() to $testApi {
  send GET "/bare"
  receive to BareResult {
    200 => Ok($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

@test "a single-field body template sends a structured JSON object, not the raw argument" {
  @fixture {
    <div class="sbt-single">x</div>
  }

  @let (el = document.querySelector('.sbt-single'))

  @eval (window.__capturedBody = null)
  @eval (window.__realFetch = window.__realFetch || window.fetch)
  @eval (window.fetch = function(url, opts) { window.__capturedBody = opts ? opts.body : null; return Promise.resolve({ status: 200, ok: true, statusText: 'OK', headers: {}, text: function() { return Promise.resolve('{}'); } }); })

  @eval ((ST.getSignalCall('singleField'))('typed-value'))

  // fire() is async (returns a Promise chain through doHttpRequest); give the
  // microtask queue a turn before asserting on the captured body.
  @wait 200ms

  @eval (window.fetch = window.__realFetch)

  @assert (window.__capturedBody === '{"fieldName":"typed-value"}')
    "single-field body template must send a structured JSON object"
}

@test "a two-field body template sends BOTH fields, correctly named and ordered" {
  @fixture {
    <div class="sbt-multi">x</div>
  }

  @let (el = document.querySelector('.sbt-multi'))

  @eval (window.__capturedBody = null)
  @eval (window.__realFetch = window.__realFetch || window.fetch)
  @eval (window.fetch = function(url, opts) { window.__capturedBody = opts ? opts.body : null; return Promise.resolve({ status: 200, ok: true, statusText: 'OK', headers: {}, text: function() { return Promise.resolve('{}'); } }); })

  @eval ((ST.getSignalCall('multiField'))('value-A', 'value-B'))

  @wait 200ms

  @eval (window.fetch = window.__realFetch)

  @assert (window.__capturedBody === '{"fieldA":"value-A","fieldB":"value-B"}')
    "two-field body template must send BOTH fields, named and ordered"
}

@test "a signal with no body clause (a GET) attaches no request body" {
  @fixture {
    <div class="sbt-bare">x</div>
  }

  @let (el = document.querySelector('.sbt-bare'))

  @eval (window.__capturedHadBody = 'unset')
  @eval (window.__realFetch = window.__realFetch || window.fetch)
  @eval (window.fetch = function(url, opts) { window.__capturedHadBody = (opts && 'body' in opts) ? 'has-body' : 'no-body'; return Promise.resolve({ status: 200, ok: true, statusText: 'OK', headers: {}, text: function() { return Promise.resolve('{}'); } }); })

  @eval ((ST.getSignalCall('bareBody'))())

  @wait 200ms

  @eval (window.fetch = window.__realFetch)

  @assert (window.__capturedHadBody === 'no-body')
    "a signal with no body clause (a GET) must attach no request body"
}
