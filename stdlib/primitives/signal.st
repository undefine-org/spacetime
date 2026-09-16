/// Signal-call primitive — drives reactive OUTPUT (@data signal / @data stream).
/// PLAN-038 W1, pillar 2. The outbound sibling of the data-source (fetch) primitive.
///
/// Binds a CALLABLE `$name` that, when invoked with the signal's params, builds a
/// request from the `send` clause (verb + target + body), sends it over the host's
/// transport, awaits the reply, and decodes it through the `receive` arms into a
/// tagged-sum variant published under `$name`. Exposes `$name_pending` (in-flight),
/// `$name_error` (last failure), and `$name_cancel()` (abort in-flight), mirroring
/// @data fetch's `$x` / `$x_loading` / `$x_error` / `$x_refetch`.
///
/// Drives BOTH transports the @host binding may name:
///   http  — resolves the base url, POSTs the JSON body, routes the response by
///           status class into the matching `receive` arm.
///   ws    — opens a POOLED WebSocket per host (shared by all signals to it),
///           sends `{ type, payload, __corr }`, correlates the reply by `__corr`,
///           and routes by message `type`. An uncorrelated inbound frame (no
///           `__corr`) is published to any listening signal — the server-PUSH
///           channel (the substrate a LiveView assign-diff / push_event rides).
/// The `policy` (latest/queue/parallel/drop) governs races; `latest` (default)
/// aborts an in-flight call when a new one fires. `live` (Phoenix LiveView)
/// routes through `window.__stLiveBridge` — the pushEvent dispatcher the host
/// LiveView's SpacetimeBridge hook mounts — so a signal fire travels the
/// Phoenix channel and the `{:reply, %{tag, reply}}` maps back through the
/// SAME receive arms (discriminator = tag, `$.reply` = the reply payload).
/// Correlation is Phoenix's own ref — no client-side __corr bookkeeping.
///
/// @param name    - the signal binding name (JS identifier, no `$`)
/// @param params  - the call signature (param_list)
/// @param host    - the @host binding the signal sends to
/// @param verb    - the send HTTP method (POST/PUT/PATCH/GET/DELETE)
/// @param target  - the send target url (relative to the host base)
/// @param body    - the send request body (object expression text)
/// @param sum     - the response sum NAME (the `to <Sum>` through-line; may be empty)
/// @param arms    - the receive decode arms (status => variant constructors)
/// @param policy  - the race policy mode (latest|queue|parallel|drop)
/// @param stream  - true for @data stream (many replies), false for one-shot signal
///
/// @yields $call    - the callable that fires the signal
/// @yields $pending - whether a call is in flight
/// @yields $error   - last error message
/// @yields $cancel  - function to abort the in-flight call
%primitive signal-call(
  name: ident,
  params: any?,
  host: binding,
  verb: ident,
  target: string,
  body: any?,
  sum: ident?,
  arms: any?,
  policy: ident?,
  timeout: any?,
  retry: any?,
  ephemeral: any?,
  stream: bool = false
) %uses host-register {
  %emit prelude-js {
    window.__stSignals = window.__stSignals || {};
    window.__stLiveStreams = window.__stLiveStreams || {};
    window.__stLiveStreamRegistrations = window.__stLiveStreamRegistrations || {};
  }

  %emit js {
    (function() {
      var signalName = %name;
      var verb = (%verb || 'POST').toUpperCase();
      var target = %target;
      var policyMode = %policy || 'latest';
      // `send emit "…" ephemeral` is a live-only, latest-wins presence update.
      // Its bridge call intentionally has no reply callback or client correlation.
      var ephemeral = %ephemeral;
      // The receive arms (decode table) + response sum name, rendered from the
      // captured `receive` block: each arm is
      //   { pat: {num|name|lit}, cons: {variant:{ctor,payload}} | {expr}, final }
      // `decodeReply` selects the first arm whose pattern matches the envelope
      // discriminator (http status / ws message type) and constructs the variant.
      var decodeArms = %arms || [];
      // BUG-181 fix: %params is the captured call signature
      // ([{name, kind, optional, type}, ...], confirmed empirically against
      // this session's own probe of @template's identical param_list
      // capture -- the SAME shape signal-call's own %params carries,
      // verified directly by instrumenting THIS primitive and inspecting
      // its real compiled output before writing this fix). %bodyTemplate
      // is the send clause's declared body -- the RAW INNER-OBJECT text
      // (e.g. `fieldA: $a, fieldB: $b`, confirmed the SAME way), never the
      // braces themselves. Together these let `buildBody` turn the call's
      // POSITIONAL arguments into the actual named, structured JSON body
      // the `send POST "..." { ... }` clause declares -- the feature this
      // primitive's own doc comment (`@param body`) already documents but
      // (before this fix) never implemented: `fire(payload)` sent the raw
      // positional argument(s) untouched, ignoring the template entirely.
      var callParams = %params || [];
      var bodyTemplate = %body;
      function buildBody(args) {
        if (bodyTemplate == null || bodyTemplate === '') {
          // No `{ ... }` body clause at all -- e.g. a plain
          // `send GET "/x"` with no body, or a single-arg signal whose
          // call argument IS meant to be the whole body verbatim
          // (documented convention: a bodyless send clause passes the
          // raw payload through, matching this primitive's PRE-fix
          // behavior for that one case so it stays backward compatible).
          return args.length === 1 ? args[0] : (args.length === 0 ? undefined : args);
        }
        // `$`-PREFIXED parameter names, not bare ones: the body template's
        // token spelling is LITERALLY `$a`/`$b` (confirmed against this
        // session's own probe: `"fieldA: $a, fieldB: $b"`), and `$` is a
        // perfectly legal JS identifier character -- naming the Function's
        // OWN parameters `$a`/`$b` lets them bind directly against the
        // template text with NO string substitution needed. A first,
        // wrong version of this fix used bare `a`/`b` parameter names,
        // which silently left `$a`/`$b` inside the template referring to
        // nothing (a ReferenceError, caught by the try/catch below and
        // silently falling back to the raw-args array) -- caught via a
        // live browser test capturing the real outgoing fetch body, not
        // assumed correct from reading the code.
        // eslint-disable-next-line no-new-func
        var paramNames = callParams.map(function(p) { return '$' + p.name; });
        try {
          var fn = new Function(paramNames.join(','), 'return ({' + bodyTemplate + '});');
          return fn.apply(null, args);
        } catch (e) {
          // A malformed template (should be caught at compile time, but
          // fail SAFE here rather than silently sending garbage) --
          // fall back to the pre-fix raw-payload behavior rather than
          // throwing out of fire() entirely.
          console.error('[ST] signal body template eval failed for ' + JSON.stringify(bodyTemplate) + ':', e);
          return args.length === 1 ? args[0] : args;
        }
      }
      var responseSum = %sum || null;
      // Lifecycle tails (optional): timeout aborts a call that runs too long;
      // retry re-fires a failed call up to N extra times. Durations arrive as a
      // ms number or a duration string (`3s`); parse leniently.
      function parseMs(v) {
        if (v == null) return 0;
        if (typeof v === 'number') return v;
        var s = String(v).trim();
        var m = s.match(/^([\d.]+)\s*(ms|s|m)?$/);
        if (!m) return parseFloat(s) || 0;
        var n = parseFloat(m[1]);
        if (m[2] === 's') return n * 1000;
        if (m[2] === 'm') return n * 60000;
        return n;
      }
      var timeoutMs = parseMs(%timeout);
      var retryMax = Number(%retry) || 0;

      var pending = false;
      var error = null;
      var inFlight = null; // AbortController for policy:latest|drop
      // policy:queue serializes calls; policy:parallel runs them concurrently.
      var queueTail = Promise.resolve(); // tail of the serialized queue
      var activeCount = 0;               // in-flight count (parallel/drop guards)
      var generation = 0;                // monotonically correlates latest fires

      // Does an arm pattern match the envelope discriminator?
      //   number  → exact http status (200)
      //   ident   → status class (`2xx`/`4xx`), `ok`/`error`, or `_` wildcard
      //   string  → ws message `type`
      function armMatches(pat, env) {
        if (!pat) return false;
        if (pat.num != null) return Number(env.status) === Number(pat.num);
        if (pat.lit != null) return String(env.type != null ? env.type : env.status) === String(pat.lit);
        if (pat.name != null) {
          var n = String(pat.name);
          if (n === '_') return true;
          if (n === 'ok') return !!env.ok;
          if (n === 'error' || n === 'err') return !env.ok;
          // status class: `2xx`, `4xx`, `5xx`
          var m = n.match(/^([1-5])xx$/i);
          if (m) { var cls = Number(m[1]); return Math.floor(Number(env.status) / 100) === cls; }
          // ws message type as a bare ident (`previewed`)
          if (env.type != null) return String(env.type) === n;
          return false;
        }
        return false;
      }

      // Evaluate a payload/consequence expression (`$.body as Todo`) against the
      // envelope. `$.` is the envelope; the `as Type` ascription is compile-time
      // only (stripped here). Falls back to the raw text on eval failure.
      function evalPayload(expr, env) {
        if (expr == null || expr === '') return null;
        var src = String(expr).replace(/\bas\s+[A-Za-z_][\w.<>\[\]]*\s*$/, '').trim();
        // `$.` → the envelope object; bare `$.x` reads env.x.
        src = src.replace(/\$\./g, '__env.');
        try {
          // eslint-disable-next-line no-new-func
          return (new Function('__env', 'return (' + src + ');'))(env);
        } catch (e) {
          return src;
        }
      }

      // Decode the envelope into a tagged variant via the receive arms.
      // Returns { variant, value, final } — variant is the constructor name
      // (`Created`), value the carried payload, final whether this arm is terminal.
      // With no arms (bare signal) the raw envelope passes through untagged.
      function decodeReply(env) {
        for (var i = 0; i < decodeArms.length; i++) {
          var arm = decodeArms[i];
          if (!arm || !armMatches(arm.pat, env)) continue;
          var isFinal = arm.final === 'final';
          var cons = arm.cons || {};
          if (cons.variant) {
            return {
              variant: cons.variant.ctor,
              value: evalPayload(cons.variant.payload, env),
              final: isFinal,
              sum: responseSum
            };
          }
          // bare-expr consequence: auto-named passthrough
          return { variant: null, value: evalPayload(cons.expr, env), final: isFinal, sum: responseSum };
        }
        // No arm matched — expose the raw envelope (forward-compatible).
        return { variant: null, value: env, final: true, sum: responseSum };
      }

      function yieldState() {
        %yield pending -> $pending;
        %yield error -> $error;
      }

      // Resolve the host config (base url + headers) registered by `@host`.
      // The host binding name carries the leading `$`; strip it to the JS key.
      function resolveHost() {
        var key = (%host || '').replace(/^\$/, '');
        return (window.__stHosts && window.__stHosts[key]) || { url: '', headers: {} };
      }

      // Publish a decoded reply: store under the signal name + dispatch the
      // `:reply` event @handle consumers dispatch on.
      function publishReply(envelope, generation, latest) {
        var reply = decodeReply(envelope);
        reply.envelope = envelope;
        // A `latest` signal can receive an older LiveView reply after a newer
        // fire (Phoenix requests are not abortable). Carry the fire generation
        // through the one decode vocabulary so every @handle ignores that stale
        // completion rather than rolling its optimistic state backward.
        reply.generation = generation;
        reply.latest = !!latest;
        if (window.__stSignals) window.__stSignals[signalName] = reply;
        if (typeof document !== 'undefined' && typeof CustomEvent !== 'undefined') {
          document.dispatchEvent(new CustomEvent('signal:' + signalName + ':reply', { detail: reply }));
        }
        return reply;
      }

      // A LiveView stream is a passive consumer: the bridge publishes the raw
      // push_event envelope and this exact same receive-arm decoder emits its
      // tagged value. Re-seeding replaces the prior registration, never stacks it.
      // Returns true once registration is settled (registered, or not a stream).
      function registerLiveStream() {
        if (!%stream || typeof window === 'undefined') return true;
        var host = resolveHost();
        var module = host.module || '';
        // The @host registration IIFE can be emitted AFTER this factory in the
        // bundle: the module is not resolvable yet. Report not-settled so the
        // caller retries instead of silently dropping the stream.
        if (!module) return false;
        var registrationKey = module + '\u0000' + signalName;
        var previous = window.__stLiveStreamRegistrations[registrationKey];
        if (previous) {
          var previousEntries = window.__stLiveStreams[previous.module];
          if (previousEntries) {
            previous.events.forEach(function(event) {
              var listeners = previousEntries[event] || [];
              previousEntries[event] = listeners.filter(function(listener) { return listener !== previous.listener; });
            });
          }
        }
        var events = decodeArms.map(function(arm) {
          var pat = arm && arm.pat || {};
          return pat.lit != null ? String(pat.lit) : (pat.name != null && pat.name !== '_' ? String(pat.name) : null);
        }).filter(function(event) { return event && !/^(ok|error|err|[1-5]xx)$/i.test(event); });
        var listener = function(payload, event) {
          return publishReply({ type: event, payload: payload, reply: payload, ok: true, status: 0 });
        };
        var entries = window.__stLiveStreams[module] || (window.__stLiveStreams[module] = {});
        events.forEach(function(event) { (entries[event] || (entries[event] = [])).push(listener); });
        window.__stLiveStreamRegistrations[registrationKey] = { module: module, events: events, listener: listener };
        return true;
      }

      // Host registration may land after this factory (bundle emission order).
      // Retry until the module resolves — the bridge polls __stLiveStreams the
      // same way, so a late registration is still picked up. Non-stream signals
      // compile %stream to false and never install the timer.
      if (%stream && typeof window !== 'undefined' && !registerLiveStream()) {
        var stStreamRegTries = 0;
        var stStreamRegTimer = setInterval(function() {
          if (registerLiveStream() || ++stStreamRegTries > 100) clearInterval(stStreamRegTimer);
        }, 50);
      }

      // Resolve-or-open a pooled WebSocket for a ws host. The socket is cached on
      // the host entry so all signals to the same host share one connection. A
      // `__pending` map correlates replies to their request by correlation id;
      // a `__corr` counter mints ids.
      function resolveWsSocket(host) {
        var key = (%host || '').replace(/^\$/, '');
        var entry = (window.__stHosts && window.__stHosts[key]) || {};
        if (entry.__ws && entry.__ws.readyState <= 1) return entry; // OPEN or CONNECTING
        var sockUrl = entry.socket || host.socket || '';
        // A socket binding may resolve to a relative path; default the scheme.
        if (sockUrl && !/^wss?:/i.test(sockUrl) && typeof location !== 'undefined') {
          var proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
          sockUrl = proto + '//' + location.host + (sockUrl[0] === '/' ? '' : '/') + sockUrl;
        }
        entry.__pending = entry.__pending || {};
        entry.__corr = entry.__corr || 0;
        try {
          var ws = new WebSocket(sockUrl);
          ws.addEventListener('message', function(ev) {
            var msg; try { msg = JSON.parse(ev.data); } catch (e) { msg = { type: 'message', payload: ev.data }; }
            // The ws envelope: { type, payload }. Correlate by __corr when present,
            // else broadcast to all pending (server may not echo the id).
            var envelope = { type: msg.type, payload: msg.payload != null ? msg.payload : msg, ok: true, status: 0 };
            var corr = msg.__corr;
            if (corr != null && entry.__pending[corr]) {
              entry.__pending[corr](envelope);
              delete entry.__pending[corr];
            } else {
              // Streaming/uncorrelated message: publish under any signal listening.
              publishReply(envelope);
            }
          });
          entry.__ws = ws;
          if (window.__stHosts) window.__stHosts[key] = entry;
        } catch (e) { /* WebSocket unavailable (SSR) */ }
        return entry;
      }

      // Perform ONE ws send+await attempt. Sends `{ type, payload, __corr }` over
      // the host socket and resolves when the correlated reply arrives. `target`
      // is the message TYPE (the `send emit "<type>"` target); `verb` is `emit`.
      function doWsRequest(payload, generation) {
        var host = resolveHost();
        var entry = resolveWsSocket(host);
        var ws = entry.__ws;
        if (!ws) return Promise.reject(new Error('ws socket unavailable'));
        return new Promise(function(resolve, reject) {
          var corr = ++entry.__corr;
          var settled = false;
          var timer = null;
          entry.__pending[corr] = function(envelope) {
            if (settled) return; settled = true;
            if (timer) clearTimeout(timer);
            resolve(publishReply(envelope, generation, policyMode === 'latest'));
          };
          if (timeoutMs > 0) {
            timer = setTimeout(function() {
              if (settled) return; settled = true;
              delete entry.__pending[corr];
              reject(new Error('ws timeout'));
            }, timeoutMs);
          }
          var frame = JSON.stringify({ type: target, payload: payload, __corr: corr });
          var sendNow = function() { try { ws.send(frame); } catch (e) { if (!settled) { settled = true; reject(e); } } };
          if (ws.readyState === 1) sendNow();
          else ws.addEventListener('open', sendNow, { once: true });
        });
      }

      // Perform ONE live (Phoenix LiveView) event + await its {:reply} (FEAT-137).
      // The wire is the host LiveView's OWN channel: window.__stLiveBridge is the
      // pushEvent dispatcher mounted by the SpacetimeBridge hook (the legitimate
      // glue layer); this primitive owns only the envelope normalisation + decode.
      // `target` is the phx event name (the `send emit "<event>"` target).
      function doLiveRequest(payload, generation) {
        var bridge = (typeof window !== 'undefined') && window.__stLiveBridge;
        if (!bridge || typeof bridge.pushEvent !== 'function') {
          return Promise.reject(new Error('live bridge unavailable — the host LiveView must mount the SpacetimeBridge hook'));
        }
        return new Promise(function(resolve, reject) {
          var settled = false;
          var timer = null;
          if (timeoutMs > 0) {
            timer = setTimeout(function() {
              if (settled) return; settled = true;
              reject(new Error('live reply timeout'));
            }, timeoutMs);
          }
          bridge.pushEvent(target, payload === undefined || payload === null ? {} : payload, function(reply) {
            if (settled) return; settled = true;
            if (timer) clearTimeout(timer);
            // The live Reply envelope: the server answers {:reply, %{tag, reply}}
            // — tag is the discriminator (the ws message-`type` position), reply
            // carries the payload (`$.reply` in the receive arms). Both `reply`
            // and `payload` keys are set so either spelling resolves.
            var tag = (reply && (reply.tag || reply.type)) || 'ok';
            var body = reply && (reply.reply !== undefined ? reply.reply : reply.payload);
            var envelope = { type: tag, payload: body, reply: body, ok: tag === 'ok', status: 0 };
            resolve(publishReply(envelope, generation, policyMode === 'latest'));
          });
        });
      }

      // Fire one transient LiveView update without waiting for a server reply.
      // Coalescing is owned by SpacetimeBridge: it retains only the latest payload
      // until the next animation frame, so all declarative callers share one rail.
      function doLiveEphemeral(payload) {
        var bridge = (typeof window !== 'undefined') && window.__stLiveBridge;
        if (!bridge || typeof bridge.pushEphemeral !== 'function') {
          return Promise.reject(new Error('live bridge unavailable — the host LiveView must mount the SpacetimeBridge hook'));
        }
        bridge.pushEphemeral(target, payload === undefined || payload === null ? {} : payload);
        return Promise.resolve();
      }

      // Perform ONE http request attempt. Resolves with the decoded reply, or
      // rejects on transport/timeout error. Honors `timeout` (AbortController-based
      // deadline) and exposes its controller via `inFlight` so policy:latest/drop
      // and cancel() can abort it.
      function doHttpRequest(payload, generation) {
        var host = resolveHost();
        var controller = (typeof AbortController !== 'undefined') ? new AbortController() : null;
        inFlight = controller;
        var url = (host.url || '') + target;
        var opts = {
          method: verb,
          headers: Object.assign({ 'content-type': 'application/json' }, host.headers || {}),
          signal: controller ? controller.signal : undefined
        };
        if (verb !== 'GET' && payload !== undefined && payload !== null) {
          opts.body = JSON.stringify(payload);
        }
        var timer = null;
        // BUG-213: a deadline abort and a policy:latest supersede-abort both
        // surface as AbortError from fetch, but they are DIFFERENT outcomes --
        // runWithRetry treats latest+AbortError as "superseded, not an error"
        // and publishes nothing, which previously made `timeout` under
        // policy:latest a silent forever-hang (pending stuck true, no _error
        // yield). Track deadline expiry here and re-brand it as a plain Error
        // so it retries/errors like the ws/live timeout paths already do.
        var timedOut = false;
        if (timeoutMs > 0 && controller) {
          timer = setTimeout(function() { timedOut = true; try { controller.abort(); } catch (e) {} }, timeoutMs);
        }
        return fetch(url, opts).then(function(res) {
          return res.text().then(function(text) {
            if (timer) clearTimeout(timer);
            var parsed;
            try { parsed = text ? JSON.parse(text) : null; } catch (e) { parsed = text; }
            return {
              status: res.status,
              ok: res.ok,
              body: parsed,
              statusText: res.statusText,
              headers: res.headers
            };
          });
        }).then(function(envelope) {
          return publishReply(envelope, generation, policyMode === 'latest');
        }, function(err) {
          if (timer) clearTimeout(timer);
          if (timedOut) throw new Error('request timed out after ' + timeoutMs + 'ms');
          throw err;
        });
      }

      // Dispatch one attempt by the host's TRANSPORT: live fires a phx event
      // over the host LiveView's channel and awaits its tagged {:reply}; ws
      // sends a correlated message frame and awaits its typed reply; http POSTs
      // and awaits the status-coded response. The receive arms decode all three
      // envelopes uniformly (live/ws discriminator = type/tag; http = status).
      function doRequest(payload, generation) {
        var host = resolveHost();
        if (host.transport === 'live') {
          return ephemeral ? doLiveEphemeral(payload) : doLiveRequest(payload, generation);
        }
        if (host.transport === 'ws') return doWsRequest(payload, generation);
        return doHttpRequest(payload, generation);
      }

      // Run a request with retry: on failure, re-attempt up to `retryMax` extra
      // times (a timeout abort counts as a failure worth retrying). Settles
      // pending/error state once the final attempt resolves or all attempts fail.
      function runWithRetry(payload, generation) {
        var attempt = 0;
        function once() {
          return doRequest(payload, generation).then(function(reply) {
            pending = false; inFlight = null; yieldState();
            return reply;
          }, function(err) {
            if (err && err.name === 'AbortError' && policyMode === 'latest') {
              // superseded by a newer call — not an error
              return;
            }
            if (attempt < retryMax) { attempt++; return once(); }
            pending = false; inFlight = null;
            error = err && err.message ? err.message : String(err);
            yieldState();
          });
        }
        return once();
      }

      // Fire the signal under its race POLICY:
      //   latest   (default) — abort any in-flight call, start fresh (last wins)
      //   queue              — serialize: each call awaits the previous one
      //   parallel           — run concurrently, no coordination
      //   drop               — ignore a new call while one is in flight
      function fire() {
        // BUG-181 fix: fire is now VARIADIC (was `fire(payload)`, a single
        // positional parameter) -- a multi-arg call site
        // (`fire.apply(null, args)` from ST.runMutations's own `$sig(a, b)`
        // dispatch, confirmed against the real compiled output this
        // session) previously silently dropped every argument past the
        // first. `payload` is now BUILT from the declared body template +
        // the call's REAL arguments (buildBody above), not just the raw
        // first positional value.
        var callArgs = Array.prototype.slice.call(arguments);
        var payload = buildBody(callArgs);
        // drop: a call is already running → ignore this one entirely.
        if (policyMode === 'drop' && activeCount > 0) return Promise.resolve();
        // latest: abort the in-flight call before starting a new one.
        if (policyMode === 'latest' && inFlight) {
          try { inFlight.abort(); } catch (e) {}
        }
        var fireGeneration = ++generation;
        pending = true;
        error = null;
        // Notify @handle consumers that a call has STARTED so their `optimistic`
        // clause runs immediately (before the reply). The handler subscribes to
        // `signal:<name>:fire`; the payload rides along for optimistic UI.
        if (typeof document !== 'undefined' && typeof CustomEvent !== 'undefined') {
          document.dispatchEvent(new CustomEvent('signal:' + signalName + ':fire', { detail: { payload: payload, generation: fireGeneration, latest: policyMode === 'latest' } }));
        }
        yieldState();

        activeCount++;
        var settle = function(r) { activeCount--; return r; };
        if (policyMode === 'queue') {
          // Chain onto the queue tail so calls run strictly in order.
          var run = queueTail.then(function() { return runWithRetry(payload, fireGeneration); });
          queueTail = run.then(settle, settle);
          return run;
        }
        // latest | parallel | drop — run immediately.
        return runWithRetry(payload, fireGeneration).then(settle, settle);
      }

      function cancel() {
        if (inFlight) {
          try { inFlight.abort(); } catch (e) {}
          inFlight = null;
          pending = false;
          yieldState();
        }
      }

      window.__stSignals[signalName] = null;
      // Register the fire fn under the by-name signal-call rail so a mutation /
      // effect body can invoke it via the documented `@on { $sig(args) }` surface
      // (ST.runMutations dispatches a bare `$name(args)` statement here). PLAN-053.
      if (typeof ST !== 'undefined' && ST.registerSignalCall) {
        ST.registerSignalCall(signalName, fire);
      }
      %yield fire -> $call;
      %yield cancel -> $cancel;
      yieldState();
    })();
  }

  %exports {
    $call: fn() ~> effect
    $pending: bool
    $error: string?
    $cancel: fn() ~> effect
  }
}


/// Signal-subscribe primitive — maps one declared LiveView assign into a local
/// reactive target. The bridge writes `st-set` diffs through SpacetimeLocal; this
/// primitive consumes that existing local update channel and records the complete
/// module + assign + target subscription triple for the live transport.
%primitive signal-subscribe(
  name: ident,
  host: binding,
  assign: ident
) %uses host-register {
  %emit prelude-js {
    window.__stLiveSubscriptions = window.__stLiveSubscriptions || {};
  }

  %emit js {
    (function() {
      var target = (%name || '').replace(/^\$/, '');
      var hostName = (%host || '').replace(/^\$/, '');
      var assign = %assign || '';
      var host = window.__stHosts[hostName] || {};
      var module = host.module || '';
      var key = module + '\u0000' + assign + '\u0000' + target;
      var subscriptions = window.__stLiveSubscriptions;

      // Re-seeding/re-running a bundle must reuse its one listener rather than
      // stacking handlers that write the same target repeatedly.
      if (!subscriptions[key]) {
        var publish = function(value) {
          // A declarative @flip bound to this target measures before this sole
          // subscription write. The write itself remains on SpacetimeLocal: no
          // second bridge or signal-update vocabulary is introduced.
          if (typeof document !== 'undefined' && typeof CustomEvent !== 'undefined') {
            document.dispatchEvent(new CustomEvent('signal:' + target + ':before-write'));
          }
          // The bridge calls this per declared (module, assign) subscription;
          // publish through the established local signal setter so views rerender.
          if (window.SpacetimeLocal && window.SpacetimeLocal.__stProxy) {
            window.SpacetimeLocal[target] = value;
          }
        };
        var setter = function(value) { publish(value); };
        var byModule = subscriptions[module] || (subscriptions[module] = {});
        var setters = byModule[assign] || (byModule[assign] = []);
        setters.push(setter);
        subscriptions[key] = { module: module, assign: assign, target: target, setter: setter };
      }

      // A bridge diff may arrive before this declaration is emitted. Seed from
      // the authoritative local channel when that already happened.
      if (window.SpacetimeLocal && assign in window.SpacetimeLocal) {
        var entry = subscriptions[key];
        entry.setter(window.SpacetimeLocal[assign]);
      }
    })();
  }

  %exports {
    $value: any
  }
}
