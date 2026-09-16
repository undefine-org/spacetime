/// Host-device-poll primitive — device-flow login completion (local dev only).
///
/// PROBLEM this closes: a real, user-reported bug this session. The widget's
/// device-flow login (`$startLogin` in `stdlib/__host__/index.st`) opened the
/// WorkOS confirm-code popup and correctly walked the user through to "You
/// are all set!" -- but nothing then polled `POST /api/v1/auth/exchange` to
/// actually retrieve the minted token, mirroring the poll-until-done loop
/// `spacetime-host-cli::main::cmd_login` performs (verified against that
/// real source: `start -> loop { sleep(interval); exchange(device_code) } ->
/// save_credentials`). The widget only ever did the START half.
///
/// WHY A PRIMITIVE, NOT `@data signal`: `signal.st`'s grammar has a bounded
/// `retry: N` (a fixed extra-attempt count) but no "poll indefinitely on a
/// SERVER-DICTATED, DYNAMICALLY GROWING interval until a terminal state" --
/// exchange's own contract (425 Too Early + optional slow_down=true meaning
/// "add 5s to your OWN poll interval", 200 done, 410 expired, 403 denied) is
/// exactly the shape `cmd_login`'s hand-written Rust loop already handles,
/// which this primitive mirrors 1:1 in JS. Same rationale `host-bridge-
/// auth.st` already established for `window.open`+`postMessage`: raw JS is
/// the sanctioned escape hatch for runtime browser mechanics this language's
/// grammar doesn't yet express.
///
/// MECHANISM:
///   1. Call-site fires `window.__stHostDevicePoll[bindingName].start(
///      apiBase, deviceCode, intervalMs, onDoneSignalName, onCodeSignalName)`
///      -- the SAME `$noop <- ...` raw-JS-call escape hatch `$startLogin`'s
///      own `window.open` already uses.
///   2. Loops: `setTimeout` for the current interval, `POST
///      <apiBase>/api/v1/auth/exchange { device_code }`, branch on status:
///        200        -> done. Calls THIS server's OWN
///                      `/__spacetime/dev/save-host-credentials` (a plain
///                      relative fetch -- same-origin as the widget's own
///                      page, exactly like `$initDeploy`'s `$selfApi`
///                      binding already establishes for `deploy-init`) to
///                      persist the token to disk, then reloads the page so
///                      the freshly-baked-in token takes effect (mirrors
///                      `deploy_init_handler`'s OWN doc comment: `$hostApiToken`
///                      is a COMPILE-TIME literal, nothing short of a reload
///                      can ever change it).
///        425        -> still pending. `slow_down: true` in the body adds
///                      5000ms to the NEXT interval (exchange's own
///                      documented remediation, mirrored verbatim from
///                      `cmd_login`'s real Rust loop).
///        410 / 403  -> terminal failure (expired / denied). Stops polling,
///                      publishes the failure reason.
///   3. Publishes `user_code` (via `onCodeSignalName`) IMMEDIATELY on start
///      (from the SAME `DeviceFlowResult` the call site already has, no
///      network round-trip needed) -- this is the SEPARATE, second real bug
///      this session's live report surfaced: WorkOS's own "confirm this code
///      is shown on your device" screen exists specifically so the user can
///      visually compare it against a code THEIR OWN device displayed; the
///      widget never displayed one, so that WorkOS screen was correctly
///      showing a security prompt with nothing on this side to compare it
///      against.
///
/// @param bindingName - the JS identifier this primitive's start() is
///                      exposed under (no `$`) -- instance-scoped, mirrors
///                      `host-bridge-auth.st`'s identical param.
%primitive host-device-poll(bindingName: ident) {
  %emit prelude-js {
    window.__stHostDevicePoll = window.__stHostDevicePoll || {};
  }

  %emit js {
    (function() {
      var name = '%bindingName';
      window.__stHostDevicePoll[name] = window.__stHostDevicePoll[name] || {};
      var state = window.__stHostDevicePoll[name];

      // Same publish channel `host-bridge-auth.st` already established
      // (and documents finding via live testing this session):
      // `window._localState[name]` + `local:<name>:updated`, NOT
      // `ST.setData` (a separate registry consumed only by `@data
      // collection`/`@each`).
      function publish(signalName, value) {
        if (!signalName) return;
        if (typeof window === 'undefined') return;
        window._localState = window._localState || {};
        window._localState[signalName] = value;
        if (!window.SpacetimeLocal) window.SpacetimeLocal = window._localState;
        if (typeof document !== 'undefined') {
          document.dispatchEvent(new CustomEvent('local:' + signalName + ':updated', { detail: value }));
        }
      }

      state.start = function(apiBase, deviceCode, intervalMs, onCodeName, code, onStatusName) {
        publish(onCodeName, code);
        publish(onStatusName, 'polling');

        var interval = Math.max(Number(intervalMs) || 5000, 1000);

        function saveAndReload(apiToken) {
          fetch('/__spacetime/dev/save-host-credentials', {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ api_token: apiToken })
          }).then(function() {
            // Reload regardless of the save call's own outcome: even a
            // failed local write still leaves the user logged in against
            // the REAL server for the rest of THIS page session (the
            // in-memory token isn't persisted here, matching this
            // primitive's own scope -- see the module doc comment), and a
            // reload is the only way `$hasAccountToken` can ever flip
            // (compile-time literal) regardless of outcome.
            if (typeof window !== 'undefined' && window.location) {
              window.location.reload();
            }
          }).catch(function() {
            if (typeof window !== 'undefined' && window.location) {
              window.location.reload();
            }
          });
        }

        function poll() {
          fetch(apiBase + '/api/v1/auth/exchange', {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ device_code: deviceCode })
          }).then(function(res) {
            if (res.status === 200) {
              return res.json().then(function(body) {
                publish(onStatusName, 'done');
                saveAndReload(body.api_token);
              });
            }
            if (res.status === 425) {
              return res.json().then(function(body) {
                if (body && body.slow_down) interval += 5000;
                setTimeout(poll, interval);
              });
            }
            if (res.status === 410) {
              publish(onStatusName, 'expired');
              return;
            }
            if (res.status === 403) {
              publish(onStatusName, 'denied');
              return;
            }
            // Any other status (network hiccup, 5xx): keep polling at the
            // current interval rather than giving up on a transient
            // failure -- mirrors cmd_login's own loop, which only ever
            // stops on the 4 explicit terminal/pending outcomes above.
            setTimeout(poll, interval);
          }).catch(function() {
            setTimeout(poll, interval);
          });
        }

        setTimeout(poll, interval);
      };
    })();
  }
}
