/// `_pending` / `_error` signal yields never propagated (BUG-197) regression tests.
///
/// Found via a live user report: the host widget's "Push preview" click was
/// totally silent for the push's entire ~20s in-flight window -- no orange
/// dot, no "Working…", no button change -- even though the push itself worked
/// (verified server-side). Two independent platform breaks, both fixed:
///
///   1. WRITE side (src/pipeline/resolve.rs): the %binds output-alias remap
///      (BUG-154) skipped INTERPOLATED aliases outright ("${$name}_pending"
///      -- the exact shape every @data signal declares for its
///      _pending/_error/_cancel yields). The primitive's
///      `%yield pending -> $pending` therefore emitted
///      `ST.set(el, "pending", …)` -- the raw per-instance export name -- and
///      nothing ever wrote `slowSig_pending`. Interpolated aliases now resolve
///      against the invocation's captures (resolve_yield_name, the BUG-126
///      helper) into the concrete remap key.
///   2. READ side (stdlib/primitives/data/derived-signal.st +
///      computed-source.st + src/emit/html_reactive.rs Global scope):
///      body-scope consumers read `SpacetimeLocal[name]` + listen for
///      `local:<name>:updated`, but element-scoped yields only cross that
///      plane when the name was registered via `ST._bridgeExport` (the
///      BUG-069 rail). The derive watcher and the Global reactive-class
///      subscription never armed it. Now they do.
///
/// The tests below pin the whole chain end to end: fire a signal whose fetch
/// is held open by hand, observe the pending derive flip TRUE mid-flight,
/// then settle and observe it flip back. Pre-fix, `$slowBusy` NEVER left
/// false (and `SpacetimeLocal` had no `slowSig_pending` key at all).

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"
@import "stdlib/macros/host"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/handle"

@host $testApi : http("https://bug197-pending-test.invalid")

@data signal $slowSig() to $testApi {
  send GET "/slow"
  receive to SlowResult {
    200 => Ok($.body);
    // Same arm shape as the host widget's $pushPreview: error bodies are
    // JSON `{"error": "…"}`; surface the message, fall back to statusText
    // for a non-JSON body (string property access is undefined, no throw).
    _   => Failed($.body.error || $.statusText);
  }
  policy latest
}

// The exact consumer shape that was dead: a derive over the auto-yielded
// `_pending` / `_error` flags (mirrors the host widget's $isOrange).
@data derive $slowBusy bool : $slowSig_pending;
@data derive $slowErrText string : ($slowSig_error || "");

@data inline $okSeen bool : false;
@data inline $failMsg string : "";

@handle $slowSig {
  receive {
    Ok(_) => { $okSeen <- true; }
    Failed(m) => { $failMsg <- m; }
  }
}

// Reactive-class surface (the html_reactive Global-scope bridge): a raw
// `_pending` yield gating a class directly, no derive in between.
.bug197-flag {
  .is-on: $slowSig_pending;
}

@test "pending derive flips true mid-flight and false after settle (BUG-197)" {
  @fixture {
    <div class="bug197-flag">x</div>
  }

  // Hold the fetch open manually: __resolveSlow is the only way out.
  @eval (window.__realFetch197 = window.fetch)
  @eval (window.fetch = function(url, opts) { return new Promise(function(resolve) { window.__resolveSlow = resolve; }); })

  @eval ((ST.getSignalCall('slowSig'))())

  // Let the fire's synchronous pending=true plus the bridge/derive cascade
  // settle (ST.set batches dependent notifications into a microtask).
  @wait 200ms

  // THE pin: pre-fix this was NEVER true -- the derive read a global key
  // nothing ever wrote.
  @assert (ST.resolve(document.body, 'slowBusy') === true)
    "derive over the auto-yielded _pending must flip $slowBusy true mid-flight"
  @assert (ST.resolve(document.body, 'slowSig_pending') === true)
    "the auto-yielded _pending flag must be true mid-flight"

  // The class-gate bridge: the raw yield drives .is-on with no derive between.
  @eval (window.SpacetimeLocal.flagOnMidFlight = document.querySelector('.bug197-flag').classList.contains('is-on'))
  @assert (window.SpacetimeLocal.flagOnMidFlight === true)
    "the class-gate bridge must apply .is-on while _pending is true"

  // Settle: 200 with a JSON body -> Ok arm.
  @eval (window.__resolveSlow({ status: 200, ok: true, statusText: 'OK', headers: {}, text: function() { return Promise.resolve('{"ok":true}'); } }))
  @wait 200ms

  @assert (ST.resolve(document.body, 'slowBusy') === false)
    "derive $slowBusy must flip false after settle"
  @assert (ST.resolve(document.body, 'okSeen') === true)
    "the Ok handle arm must set $okSeen true after a 200"

  @eval (window.fetch = window.__realFetch197)
}

@test "error yield propagates on a network-level rejection (BUG-197)" {
  @fixture {
    <div class="bug197-err">x</div>
  }

  // A rejected fetch never reaches a receive arm at all -- only the `_error`
  // yield carries it. Pre-fix $slowErrText could therefore never show anything.
  @eval (window.__realFetch197e = window.fetch)
  @eval (window.fetch = function(url, opts) { return Promise.reject(new Error('boom-197')); })

  @eval ((ST.getSignalCall('slowSig'))())

  @wait 200ms

  @assert (ST.resolve(document.body, 'slowErrText') === 'boom-197')
    "the _error yield must surface a network rejection's message"
  @assert (ST.resolve(document.body, 'slowBusy') === false)
    "$slowBusy must be false after a rejection"

  @eval (window.fetch = window.__realFetch197e)
}

@test "Failed arm surfaces the JSON error body's message (BUG-197)" {
  @fixture {
    <div class="bug197-fail">x</div>
  }

  // The dev server's HostPushErrorResponse shape: `{"error": "…"}`. The
  // arm must surface the message STRING, never `[object Object]`.
  @eval (window.__realFetch197f = window.fetch)
  @eval (window.fetch = function(url, opts) { return Promise.resolve({ status: 401, ok: false, statusText: 'Unauthorized', headers: {}, text: function() { return Promise.resolve('{"error":"Not logged in -- run spacetime host login first"}'); } }); })

  @eval ((ST.getSignalCall('slowSig'))())

  @wait 200ms

  @assert (ST.resolve(document.body, 'failMsg') === 'Not logged in -- run spacetime host login first')
    "the Failed arm must surface the JSON error body's message string"
  @assert (ST.resolve(document.body, 'slowBusy') === false)
    "$slowBusy must be false after a Failed reply"

  @eval (window.fetch = window.__realFetch197f)
}
