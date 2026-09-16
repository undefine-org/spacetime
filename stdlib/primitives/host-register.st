/// Host-register primitive — populates the runtime host table (PLAN-038, FUP-081 #4).
///
/// `@host $api : http("…") { headers: {…} }` binds this GLOBAL-phase primitive so
/// the emitted page registers the host's transport config under
/// `window.__stHosts[<name>]` at load time. The `signal-call` primitive then
/// resolves `__stHosts[key]` (base url + headers for http; socket for ws) when a
/// `@data signal`/`@data stream` fires.
///
/// Before this primitive, `@host` only registered a COMPILE-TIME fact (read by the
/// signal macro to pick its transport), so `resolveHost()` always saw an empty
/// table and every signal POSTed to a relative `target` with no base/headers. This
/// closes that gap: the host config now exists at runtime, exactly where the wire
/// call needs it.
///
/// Harmony: one fact, two consumers — the COMPILE-TIME `host(...)` fact types the
/// receive subject + chooses the transport; this RUNTIME registration carries the
/// same config (url/headers/socket) into the browser. The primitive is config-only
/// (no element), mirroring `signal-call`'s global phase.
///
/// @param name      - the host binding name (JS identifier, no `$`)
/// @param transport - "http" | "ws" | "live"
/// @param url       - the http base url expression (empty for ws/live)
/// @param headers   - the http headers object expression (may be empty)
/// @param socket    - the ws socket expression (empty for http/live)
%primitive host-register(
  name: ident,
  transport: ident,
  url: any? = "",
  headers: any? = {},
  socket: any? = ""
) {
  %emit prelude-js {
    window.__stHosts = window.__stHosts || {};
  }

  %emit js {
    (function() {
      var key = (%name || '').replace(/^\$/, '');
      var transport = %transport || 'http';
      var entry = { transport: transport };
      if (transport === 'http') {
        entry.url = %url || '';
        entry.headers = %headers || {};
      } else if (transport === 'ws') {
        // The socket expression resolves to a url string or a live socket binding;
        // signal-call opens/correlates the actual WebSocket. Store the raw value.
        entry.socket = %socket || '';
      } else {
        // live (Phoenix LiveView) — the module name; the live transport reads it.
        entry.module = %socket || %url || '';
      }
      window.__stHosts[key] = entry;
    })();
  }
}
