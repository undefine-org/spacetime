/// BUG-213: the host pill's "Create project" click is TOTALLY SILENT when the
/// orchestrator is unreachable at the network level (live user report:
/// "when I click on create project in the host pill it does nothing").
///
/// Root cause chain (verified live against a black-holed spacetime.undefine.org):
///   - a network-level fetch rejection never builds a reply envelope --
///     stdlib/primitives/signal.st's runWithRetry catches it and publishes
///     ONLY the `_pending`/`_error` yields (the BUG-197 rail). No
///     `signal:<name>:reply` event fires, so NO @handle receive arm runs.
///   - stdlib/__host__/index.st's `@handle $createProject` only ever wrote
///     `$createProjectError` from its reply arms (Created/Rejected/Failed)
///     => the error <p> stayed hidden forever.
///   - the widget never consumed `$createProject_error` -- even though its
///     OWN push-preview section already established exactly that pattern
///     after the same bug class ($pushPreviewErrorText, BUG-197).
///
/// Fix (stdlib/__host__/index.st): fold the `_error` yield into the visible
/// error text, mirroring push-preview verbatim:
///   @data derive $createProjectErrorText string :
///       ($createProjectError || $createProject_error || "");
/// and gate/render from THAT. Plus `timeout 15s` on the signal's policy so a
/// black-holed host settles in seconds, not the browser's TCP timeout.
///
/// REPLICA CONVENTION (host-badge-union.test.st precedent): __host__/index.st
/// can't be imported standalone (its @host facts carry compile-time
/// __SPACETIME_HOST_* placeholders), so this file replicates the
/// create-project error chain verbatim. If the widget's chain changes,
/// change this replica with it.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"
@import "stdlib/macros/host"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/handle"

@host $testHostApi : http("https://bug213-create-project-test.invalid")

// ---- replica: the widget's $createProject signal (arm shapes verbatim) ----
@data signal $createProject($name string) to $testHostApi {
  send POST "/api/v1/projects" { name: $name }
  receive to CreateProjectResult {
    200 => Created($.body);
    409 => Rejected($.body);
    422 => Rejected($.body);
    _   => Failed($.statusText);
  }
  policy latest timeout 15s
}

@data inline $createProjectError string : "";

@handle $createProject {
  receive {
    Created(body) => { $createProjectError <- ""; }
    Rejected(body) => { $createProjectError <- "That project name is unavailable -- try another."; }
    Failed(_) => { $createProjectError <- "Could not create project -- try again shortly."; }
  }
}

// ---- THE PIN: the visible error text must ALSO carry the network-level
// `_error` yield (the only rail a rejected fetch ever reaches). Pre-fix the
// widget derived $hasCreateProjectError from $createProjectError ALONE, so
// this text stayed "" and the error <p> stayed hidden.
@data derive $createProjectErrorText string : ($createProjectError || $createProject_error || "");
@data derive $hasCreateProjectError bool : ($createProjectErrorText != "");

.bug213-error {
  .is-hidden: !$hasCreateProjectError;
}

@test "a network-level rejection surfaces in the create-project error text (BUG-213)" {
  @fixture {
    <p class="bug213-error">err</p>
  }

  // A dead orchestrator: fetch rejects before any HTTP status exists.
  // (Exactly what a black-holed host produces -- "TypeError: Failed to
  // fetch" observed live.)
  @eval (window.__realFetch213 = window.fetch)
  @eval (window.fetch = function(url, opts) { return Promise.reject(new Error('Failed to fetch')); })

  @eval ((ST.getSignalCall('createProject'))('backdesk-new'))

  @wait 200ms

  // The error text carries the network failure; the gate flips visible.
  @assert (ST.resolve(document.body, 'createProjectErrorText') === 'Failed to fetch')
    "the error text must carry a network rejection"
  @assert (ST.resolve(document.body, 'hasCreateProjectError') === true)
    "the error gate must flip visible on a network rejection"

  @eval (window.SpacetimeLocal.bug213Hidden = document.querySelector('.bug213-error').classList.contains('is-hidden'))
  @assert (window.SpacetimeLocal.bug213Hidden === false)
    "the error <p> must not be hidden while an error is shown"

  @eval (window.fetch = window.__realFetch213)
}

@test "an HTTP-level Failed reply still surfaces through the same text (must stay green)" {
  @fixture {
    <p class="bug213-error">err</p>
  }

  @eval (window.__realFetch213b = window.fetch)
  @eval (window.fetch = function(url, opts) { return Promise.resolve({ status: 500, ok: false, statusText: 'Internal Server Error', headers: {}, text: function() { return Promise.resolve('{}'); } }); })

  @eval ((ST.getSignalCall('createProject'))('backdesk-new'))

  @wait 200ms

  @assert (ST.resolve(document.body, 'createProjectErrorText') === 'Could not create project -- try again shortly.')
    "an HTTP-level Failed reply must surface its message"
  @assert (ST.resolve(document.body, 'hasCreateProjectError') === true)
    "the error gate must stay visible on an HTTP-level failure"

  @eval (window.fetch = window.__realFetch213b)
}

@test "a success reply keeps the error surface clear (must stay green)" {
  @fixture {
    <p class="bug213-error">err</p>
  }

  @eval (window.__realFetch213c = window.fetch)
  @eval (window.fetch = function(url, opts) { return Promise.resolve({ status: 200, ok: true, statusText: 'OK', headers: {}, text: function() { return Promise.resolve('{"id":"proj-bug213"}'); } }); })

  @eval ((ST.getSignalCall('createProject'))('backdesk-new'))

  @wait 200ms

  @assert (ST.resolve(document.body, 'createProjectErrorText') === '')
    "a success reply must leave the error text empty"
  @assert (ST.resolve(document.body, 'hasCreateProjectError') === false)
    "a success reply must keep the error gate hidden"

  @eval (window.fetch = window.__realFetch213c)
}

// ---- the timeout half of the fix: `timeout` under policy:latest ----------
// Pre-fix, a deadline abort surfaced as AbortError, which runWithRetry
// deliberately swallows under policy:latest ("superseded by a newer call").
// The deadline therefore produced NO _error yield and pending stuck true
// forever. signal.st now re-brands deadline expiry as a plain Error.
@data signal $slowCreate() to $testHostApi {
  send POST "/api/v1/projects" { name: "x" }
  receive to SlowCreateResult {
    200 => Created($.body);
    _   => Failed($.statusText);
  }
  policy latest timeout 100ms
}

@data derive $slowCreateErr string : ($slowCreate_error || "");
@data derive $slowCreateBusy bool : $slowCreate_pending;

@test "a deadline expiry under policy latest publishes the _error yield (BUG-213)" {
  @fixture {
    <p class="bug213-timeout">x</p>
  }

  // A black-holed host: fetch never settles on its own, but honors the
  // abort signal (exactly what a real fetch against a dead TCP peer does
  // when its AbortController fires).
  @eval (window.__realFetch213t = window.fetch)
  @eval (window.fetch = function(url, opts) { return new Promise(function(resolve, reject) { if (opts && opts.signal) { opts.signal.addEventListener('abort', function() { var e = new Error('The operation was aborted'); e.name = 'AbortError'; reject(e); }); } }); })

  @eval ((ST.getSignalCall('slowCreate'))())

  // Let the 100ms deadline fire and the error cascade settle.
  @wait 300ms

  @assert (ST.resolve(document.body, 'slowCreateErr') === 'request timed out after 100ms')
    "a deadline expiry under policy latest must publish the _error yield"
  @assert (ST.resolve(document.body, 'slowCreateBusy') === false)
    "$slowCreateBusy must be false after a deadline expiry"

  @eval (window.fetch = window.__realFetch213t)
}

