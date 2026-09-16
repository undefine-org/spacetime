/// Host-bridge-auth primitive — deployed-site widget auth handshake (PLAN-004).
///
/// PROBLEM this closes: a `@host` binding's `headers` value is a COMPILE-TIME
/// LITERAL only (confirmed empirically: `headers: { authorization: $token }`
/// compiles the literal 6-character string `"$token"`, never the signal's
/// runtime value — same limitation `stdlib/__host__/index.st`'s own header
/// comment already documents for `url`). `src/server.rs::compile_host_widget`
/// works around this LOCALLY by substituting a real token server-side before
/// serving the bundle — but that substitution reads the DEVELOPER's own local
/// credentials file, which has no equivalent once the SAME bundle is served
/// STATICALLY by `spacetime-host-edge` (a Cloudflare Worker with no local disk
/// to read from). PLAN-004's bridge (`commercial/host`'s
/// `spacetime-host-server::bridge`) mints a real, short-lived, project-scoped
/// bearer token via a popup+postMessage handshake instead; THIS primitive is
/// the receiving end of that handshake, running in the widget's own page.
///
/// WHY A PRIMITIVE, NOT INLINE WIDGET JS: this repo's own root AGENTS.md
/// forbids hand-written JS on Spacetime sites — stdlib primitives ARE the
/// sanctioned escape hatch for runtime browser mechanics Spacetime's own
/// grammar doesn't yet express (the SAME rationale `host-register.st`/
/// `dev-hooks.st` already establish for `window.__stHosts` registration and
/// global error listeners, respectively). This primitive is scoped
/// analogously: open a popup, listen for its `postMessage`, and publish the
/// result into the SAME reactive-signal registry every other data primitive
/// uses (`ST.setData`, confirmed via `stdlib/primitives/data/collection.st`'s
/// own usage) — no new state-management mechanism invented.
///
/// MECHANISM:
///   1. Call-site fires this primitive's emitted `window.__stHostBridgeAuth
///      .start(bridgeBaseUrl, projectId, hostBindingName)` function (from a
///      `$noop <- ...` mutation, the SAME escape-hatch idiom
///      `stdlib/__host__/index.st` already uses for `window.open`).
///   2. Opens a popup to `<bridgeBaseUrl>/bridge/login?project_id=<id>`.
///   3. Listens for a `message` event whose `event.source` is EXACTLY that
///      popup (rejects any other origin/window — the ONLY integrity check
///      this side of the handshake can make, since the bridge's own
///      `postMessage` targetOrigin is deliberately `"*"`, per that route's
///      own doc comment: it cannot know the widget's origin ahead of time).
///   4. On a valid message, writes the token into `sessionStorage` (keyed by
///      project id, so a page reload does not require a fresh popup within
///      the token's own 15-minute lifetime) AND mutates
///      `window.__stHosts[hostBindingName].headers.authorization` IN PLACE
///      (the SAME object `host-register.st` populated at load time, and the
///      SAME object `resolveHost()` in `stdlib/primitives/signal.st` reads on
///      every subsequent signal call) — so every `@data signal` against that
///      `@host` binding immediately starts carrying the real token, with zero
///      changes needed to `@host`/`signal.st` themselves.
///   5. Publishes `<signalNamePrefix>BridgeReady` = true via `ST.setData` so a
///      `@data inline`/`@handle`-driven re-fire of the status signal can react
///      (the call site's own responsibility — this primitive only completes
///      the token handoff, it does not know which signal to re-fire).
///
/// @param bindingName - the JS identifier this primitive's start()/
///                      sessionStorage key are exposed under (no `$`) — lets
///                      multiple widgets on one page (unlikely today, but
///                      keeps this primitive instance-scoped rather than a
///                      single hardcoded global name) coexist.
%primitive host-bridge-auth(bindingName: ident) %uses host-register {
  %emit prelude-js {
    window.__stHostBridgeAuth = window.__stHostBridgeAuth || {};
  }

  %emit js {
    (function() {
      var name = '%bindingName';
      window.__stHostBridgeAuth[name] = window.__stHostBridgeAuth[name] || {};
      var state = window.__stHostBridgeAuth[name];

      // sessionStorage read on load: a prior handshake this SAME tab
      // session already completed (e.g. a page reload within the
      // token's 15-minute lifetime) needs no fresh popup.
      function storageKey(projectId) {
        return 'spacetime-host-bridge-token:' + projectId;
      }

      function applyToken(hostBindingName, token) {
        var key = (hostBindingName || '').replace(/^\$/, '');
        // __stHosts is guaranteed by host-register's prelude (%uses host-register).
        var entry = window.__stHosts[key] || { transport: 'http', url: '', headers: {} };
        entry.headers = entry.headers || {};
        entry.headers.authorization = 'Bearer ' + token;
        window.__stHosts[key] = entry;
      }

      // Publish readiness the SAME way `@data derive`/reactive-source
      // primitives write a plain global signal (stdlib/primitives/data/
      // reactive-source.st's own `window._localState[name] = value` +
      // `local:<name>:updated` dispatch) -- confirmed via live browser
      // testing this session as the CORRECT channel `ST.watchScoped`
      // (the mechanism `@effect(on: $sig)` subscribes through) actually
      // reads: `SpacetimeLocal[name]` on the `local:<name>:updated`
      // event. `ST.setData` (an EARLIER, WRONG version of this function
      // used) writes a SEPARATE `_dataRegistry` map consumed only by
      // `@data collection`/`@each`-style bindings -- NOT this channel;
      // using it here silently left `$bridgeReady`'s consumers unfired,
      // caught only by an actual live-browser postMessage-handshake test,
      // not by reading collection.st's usage and assuming it generalized.
      function publishReady(readySignalName, ready) {
        if (!readySignalName) return;
        if (typeof window === 'undefined') return;
        window._localState = window._localState || {};
        window._localState[readySignalName] = ready;
        if (!window.SpacetimeLocal) window.SpacetimeLocal = window._localState;
        if (typeof document !== 'undefined') {
          document.dispatchEvent(new CustomEvent('local:' + readySignalName + ':updated', { detail: ready }));
        }
      }

      // Tries a previously-stored token for `projectId` before opening a
      // fresh popup -- returns true if a stored token was applied.
      state.tryRestore = function(hostBindingName, projectId, readySignalName) {
        try {
          var token = window.sessionStorage.getItem(storageKey(projectId));
          if (token) {
            applyToken(hostBindingName, token);
            publishReady(readySignalName, true);
            return true;
          }
        } catch (e) { /* sessionStorage unavailable (private mode etc) -- fall through */ }
        return false;
      };

      // Opens the bridge popup and completes the handshake asynchronously.
      state.start = function(bridgeBaseUrl, hostBindingName, projectId, readySignalName) {
        var popup = window.open(
          bridgeBaseUrl + '/bridge/login?project_id=' + encodeURIComponent(projectId),
          'spacetime-host-bridge',
          'width=480,height=640'
        );
        if (!popup) return; // popup blocked -- the widget's own Blue-state click target remains the retry affordance

        function onMessage(event) {
          if (event.source !== popup) return; // integrity check: only the popup we opened
          var data = event.data || {};
          if (data.source !== 'spacetime-host-bridge') return;
          window.removeEventListener('message', onMessage);
          if (data.token) {
            try { window.sessionStorage.setItem(storageKey(projectId), data.token); } catch (e) { /* best-effort */ }
            applyToken(hostBindingName, data.token);
            publishReady(readySignalName, true);
          }
        }
        window.addEventListener('message', onMessage);
      };
    })();
  }
}
