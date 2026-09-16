/// MCP host bridge primitives and macros.
///
/// FEAT-127 region-isolated author syntax (one bridge PER region root):
///
/// ```st
/// @import "stdlib/__mcp__";
///
/// <div data-st-instance="inst-1">
///   <button class="approve">Approve</button>
/// </div>
///
/// [data-st-instance] {
///   @mcp-host
/// }
///
/// .approve {
///   @mcp-click(type: "approved", correlation: "review-1", value: "approve")
/// }
/// ```
///
/// `@mcp-host` is selector-scoped: it resolves the NEAREST ancestor
/// `[data-st-instance]` region root and attaches one `__stHost` bridge there.
/// `@mcp-click(...)` / `@mcp-action(...)` resolve the same nearest-ancestor bridge
/// at click time — no `window.__stHost` global, so two regions on one page carry
/// independent bridges with no cross-region event misrouting.
/// JavaScript stays here, inside owned primitives; pages stay declarative
/// Spacetime.

%primitive mcp-host(&root) {
  %emit js {
    var root = %&root;
    // Resolve the REGION ROOT: the nearest ancestor (or self) carrying
    // `data-st-instance`. One bridge per region, no page-global singleton.
    var hostEl = root && root.closest ? root.closest('[data-st-instance]') : null;
    var instance = hostEl ? (hostEl.getAttribute('data-st-instance') || '') : '';
    var origin = hostEl ? (hostEl.getAttribute('data-st-origin') || '') : '';
    var connected = !!instance;
    var lastMessage = null;
    var error = connected ? null : 'No data-st-instance on the bound element or an ancestor';
    // BUG-153 + FUP-111: a SINGLE ready-to-display string for the action toast.
    // The reactive `text:` binding cannot coalesce (`$a || $b` drops all but the
    // first term — FUP-111), so the bridge computes the one message here:
    // server error > human summary > raw action name. Null when idle.
    var actionText = null;
    var actionOk = true;

    function yieldState() {
      %yield connected -> $connected;
      %yield lastMessage -> $lastMessage;
      %yield error -> $error;
      %yield actionText -> $actionText;
      %yield actionOk -> $actionOk;
    }

    function dispatch(envelope) {
      if (!instance) {
        error = 'Missing MCP instance id';
        yieldState();
        return Promise.reject(new Error(error));
      }
      var msg = envelope || {};
      if (!msg.type) msg.type = 'message';
      return fetch('/__mcp/signal/' + encodeURIComponent(instance), {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(msg)
      }).then(function(res) {
        return res.json().catch(function() { return { ok: res.ok }; });
      }).then(function(data) {
        lastMessage = data;
        // BUG-153: an action FAILURE rides INSIDE the envelope. The signal sink
        // always returns a top-level `{ok:true, event, action}` (transport
        // succeeded); a rejected server action is `action.ok === false` with an
        // `action.error`. Checking only top-level `data.ok` made every action
        // failure look like success (a silent no-op). Classify BOTH levels: a
        // transport failure (`data.ok===false`) OR a nested action failure
        // (`data.action.ok===false`) is an error, and surface the server's OWN
        // message so the UI can echo it verbatim.
        var act = data && data.action;
        var actionName = (data && data.event && data.event.payload && data.event.payload.action) || '';
        if (data && data.ok === false) {
          error = (data.error) ? String(data.error) : 'MCP signal rejected';
          actionOk = false;
          actionText = error;
        } else if (act && act.ok === false) {
          error = (act.error) ? String(act.error) : 'action rejected';
          actionOk = false;
          actionText = error;
        } else {
          error = null;
          actionOk = true;
          // Success: prefer the server's human summary, else the raw action name.
          // PLAN-065 W_hygiene: SUPPRESS the success toast for high-frequency
          // field-edit commits (mcp_param / mcp_node_param) — they fire on every
          // keystroke-commit and a `Set title = …` toast per edit is pure noise;
          // the canvas updating IS the acknowledgement. FAILURES still toast (the
          // branches above set actionText/actionOk regardless of action name), so
          // a rejected edit is never silent.
          var quiet = (actionName === 'mcp_param' || actionName === 'mcp_node_param');
          actionText = quiet ? null : ((act && act.summary) ? String(act.summary) : String(actionName));
        }
        yieldState();
        // PLAN-064 U6: the action-echo toast binds `.is-shown` to
        // `lastMessage != null`. Reset it to null shortly after so the class
        // goes true->false->true across actions — making the toast TRANSIENT and
        // RE-TRIGGERABLE (each new action replays the flash keyframe) instead of
        // sticking on forever. The reset re-yields so the binding updates. Guard
        // against a newer message having arrived (only clear if still this one).
        //
        // The reset delay MUST outlast the CSS toast animation, else the text/red
        // state clears mid-animation (reviewer P2). styles.st holds a SUCCESS toast
        // 2.6s and an ERROR toast 4.6s (errors linger so the message is readable),
        // so reset just past the longer of the two per outcome.
        var settled = lastMessage;
        var resetAfter = actionOk ? 2800 : 4800;
        setTimeout(function() {
          if (lastMessage === settled) {
            lastMessage = null;
            actionText = null;
            actionOk = true;
            yieldState();
          }
        }, resetAfter);
        return data;
      }).catch(function(err) {
        error = err && err.message ? err.message : String(err);
        yieldState();
        throw err;
      });
    }

    // Attach the bridge to the REGION ROOT element — the same node that
    // `mcp-click` / `mcp-action` resolve via `target.closest('[data-st-instance]')`.
    // Each region carries its own `__stHost`; nested regions resolve to the
    // innermost (closest) ancestor.
    if (hostEl) {
      hostEl.__stHost = {
        instance: instance,
        origin: origin,
        dispatch: dispatch,
        signal: function(type, payload, correlation) {
          return dispatch({ type: type, payload: payload || {}, correlation: correlation || null });
        },
        get connected() { return connected; },
        get lastMessage() { return lastMessage; },
        get error() { return error; }
      };
    }

    yieldState();
  }

  %exports {
    $connected: bool
    $lastMessage: object
    $error: string
    $actionText: string
    $actionOk: bool
  }
}

%primitive mcp-click-event(&el, eventType: string = "clicked", correlation: string = "", value: string = "") {
  %emit js {
    var target = %&el;
    var type = '%eventType' || 'clicked';
    var correlation = '%correlation' || null;
    var value = '%value' || '';
    var sent = false;
    var lastMessage = null;
    var error = null;
    var handlerKey = ['mcp-click', type, correlation || '', value].join('|');

    function installClickHandler(el, key, handler) {
      if (!el) return;
      var handlers = el.__stMcpHandlers || (el.__stMcpHandlers = {});
      if (handlers[key]) el.removeEventListener('click', handlers[key]);
      handlers[key] = handler;
      el.addEventListener('click', handler);
    }

    function removeClickHandler(el, key, handler) {
      if (!el || !el.__stMcpHandlers || el.__stMcpHandlers[key] !== handler) return;
      el.removeEventListener('click', handler);
      delete el.__stMcpHandlers[key];
    }

    function yieldState() {
      %yield sent -> $sent;
      %yield lastMessage -> $lastMessage;
      %yield error -> $error;
    }

    function resolveHost() {
      // NEAREST ANCESTOR region root carries the bridge for THIS region.
      // Nested regions resolve to the innermost (closest) ancestor.
      var regionEl = target && target.closest ? target.closest('[data-st-instance]') : null;
      return regionEl ? regionEl.__stHost : null;
    }

    function payload() {
      var host = resolveHost();
      return {
        instance: host ? host.instance : '',
        text: (target && target.textContent ? target.textContent : '').trim(),
        value: value,
        at: new Date().toISOString()
      };
    }

    function onClick() {
      var host = resolveHost();
      if (!host || !host.signal) {
        sent = false;
        error = 'MCP host bridge is not installed on the nearest [data-st-instance]';
        yieldState();
        return;
      }
      host.signal(type, payload(), correlation).then(function(message) {
        sent = true;
        lastMessage = message;
        error = null;
        yieldState();
      }).catch(function(err) {
        sent = false;
        error = err && err.message ? err.message : String(err);
        yieldState();
      });
    }

    yieldState();
    installClickHandler(target, handlerKey, onClick);

    %cleanup {
      removeClickHandler(target, handlerKey, onClick);
    }
  }

  %exports {
    $sent: bool
    $lastMessage: object
    $error: string
  }
}

%primitive mcp-action-event(&el, action: string, target: string = "", value: string = "", targetAttr: string = "", valueAttr: string = "", on: string = "click") {
  %emit js {
    var targetEl = %&el;
    var action = '%action' || 'action';
    var actionTarget = '%target' || '';
    var value = '%value' || '';
    var targetAttr = '%targetAttr' || '';
    var valueAttr = '%valueAttr' || '';
    // M4: the DOM event that fires the action. Default "click" (buttons); "change"
    // / "input" let a text field emit on edit (the editable param control). For a
    // form control the live `.value` is the natural value source.
    var evName = '%on' || 'click';
    var sent = false;
    var lastMessage = null;
    var error = null;
    var handlerKey = ['mcp-action', action, actionTarget, value, targetAttr, valueAttr, evName].join('|');

    function installClickHandler(el, key, handler) {
      if (!el) return;
      var handlers = el.__stMcpHandlers || (el.__stMcpHandlers = {});
      if (handlers[key]) el.removeEventListener(evName, handlers[key]);
      handlers[key] = handler;
      el.addEventListener(evName, handler);
    }

    function removeClickHandler(el, key, handler) {
      if (!el || !el.__stMcpHandlers || el.__stMcpHandlers[key] !== handler) return;
      el.removeEventListener(evName, handler);
      delete el.__stMcpHandlers[key];
    }

    function yieldState() {
      %yield sent -> $sent;
      %yield lastMessage -> $lastMessage;
      %yield error -> $error;
    }

    function attr(name) {
      return name && targetEl && targetEl.getAttribute ? (targetEl.getAttribute(name) || '') : '';
    }

    function resolveHost() {
      // NEAREST ANCESTOR region root carries the bridge for THIS region.
      var regionEl = targetEl && targetEl.closest ? targetEl.closest('[data-st-instance]') : null;
      return regionEl ? regionEl.__stHost : null;
    }

    function payload() {
      var resolvedTarget = actionTarget || attr(targetAttr);
      // M4: a form control's live `.value` is the natural value for change/input
      // events; fall back to the static value or a value-attr otherwise.
      // A CHECKBOX's `.value` is the HTML default `"on"` whatever its state, so
      // reading it sends `on` for both checking AND unchecking — a bool param
      // would be written as the literal token `on` and a binding override could
      // never express false. The state lives in `.checked` (PLAN-112 W5R).
      var isCheckable = targetEl && (targetEl.type === 'checkbox' || targetEl.type === 'radio');
      var elValue = isCheckable
        ? (targetEl.checked ? 'true' : 'false')
        : ((targetEl && typeof targetEl.value === 'string') ? targetEl.value : null);
      var resolvedValue = value || attr(valueAttr) || (elValue != null ? elValue : '');
      var host = resolveHost();
      // B4: forward EVERY `data-st-payload-*` dataset entry on the target as a
      // top-level payload field, camelCase->snake_case (e.g.
      // `data-st-payload-span-start="245"` -> payload.span_start = "245"). A
      // general rail: an action that needs MORE than target+value (e.g.
      // mcp_node_param's span_start/span_end addressing) declares the extra data
      // on its element, no per-action primitive change. Built-in keys are never
      // overwritten.
      var extra = {};
      if (targetEl && targetEl.dataset) {
        var reserved = { action: 1, target: 1, value: 1, text: 1, instance: 1, at: 1 };
        Object.keys(targetEl.dataset).forEach(function(k) {
          if (k.indexOf('stPayload') !== 0) return;
          var rest = k.slice('stPayload'.length);
          if (!rest) return;
          // stPayloadSpanStart -> span_start
          var snake = rest.charAt(0).toLowerCase() + rest.slice(1);
          snake = snake.replace(/[A-Z]/g, function(c) { return '_' + c.toLowerCase(); });
          if (!reserved[snake]) extra[snake] = targetEl.dataset[k];
        });
      }
      var base = {
        action: action,
        target: resolvedTarget,
        value: resolvedValue,
        text: (targetEl && targetEl.textContent ? targetEl.textContent : '').trim(),
        instance: host ? host.instance : '',
        at: new Date().toISOString()
      };
      Object.keys(extra).forEach(function(k) { base[k] = extra[k]; });
      return base;
    }

    function onClick(ev) {
      // BUG-114: for a value-carrying control event (change/input/blur), dispatch
      // ONLY on a genuine USER edit. A reactive `text:`/value binding sets the
      // control's value during hydration and may dispatch a SYNTHETIC event
      // (isTrusted=false); without this guard that synthetic event fired
      // mcp_source_edit and silently rewrote the inspected guest's source. A real
      // user keystroke/commit is isTrusted=true. A plain click (buttons) has no
      // such hydration hazard, so it is never gated.
      // A COMPOSITE widget (range slider, HSV picker) commits by writing its
      // twin/hex partner and dispatching an event FROM that partner — the user
      // acted, but the event the rail sees is synthetic. Such a commit carries
      // `stFieldCommit`, which a hydration write never does, so it is admitted
      // while the hydration hazard above stays closed (PLAN-112 W5R: without
      // this, every workbench slider and colour edit rendered but silently
      // emitted no action at all).
      if (evName !== 'click' && ev && ev.isTrusted === false && !ev.stFieldCommit) {
        return;
      }
      var host = resolveHost();
      if (!host || !host.signal) {
        sent = false;
        error = 'MCP host bridge is not installed on the nearest [data-st-instance]';
        yieldState();
        return;
      }
      var body = payload();
      host.signal('mcp-action', body, body.target || action).then(function(message) {
        sent = true;
        lastMessage = message;
        error = null;
        yieldState();
      }).catch(function(err) {
        sent = false;
        error = err && err.message ? err.message : String(err);
        yieldState();
      });
    }

    yieldState();
    installClickHandler(targetEl, handlerKey, onClick);

    %cleanup {
      removeClickHandler(targetEl, handlerKey, onClick);
    }
  }

  %exports {
    $sent: bool
    $lastMessage: object
    $error: string
  }
}

%primitive mcp-submit-event(&el, action: string = "kit-submit") {
  %emit js {
    var targetEl = %&el;
    var action = '%action' || 'kit-submit';
    var sent = false;
    var lastMessage = null;
    var error = null;
    var handlerKey = ['mcp-submit', action].join('|');

    function installClickHandler(el, key, handler) {
      if (!el) return;
      var handlers = el.__stMcpHandlers || (el.__stMcpHandlers = {});
      if (handlers[key]) el.removeEventListener('click', handlers[key]);
      handlers[key] = handler;
      el.addEventListener('click', handler);
    }

    function removeClickHandler(el, key, handler) {
      if (!el || !el.__stMcpHandlers || el.__stMcpHandlers[key] !== handler) return;
      el.removeEventListener('click', handler);
      delete el.__stMcpHandlers[key];
    }

    function yieldState() {
      %yield sent -> $sent;
      %yield lastMessage -> $lastMessage;
      %yield error -> $error;
    }

    function resolveRegion() {
      // NEAREST ANCESTOR region root carries the bridge AND scopes the controls
      // this submit collects — so two regions on one page never cross-read.
      return targetEl && targetEl.closest ? targetEl.closest('[data-st-instance]') : null;
    }

    // The control-gathering loop lives in the runtime (ST._mcpCollectValues):
    // the %emit js statement splitter mangles a loop-bearing helper function
    // here, but a single call expression serializes intact (FUP-063-class). The
    // helper gathers every input[name]/select[name]/textarea[name] in the region
    // into { name: value } — the form-read step the fixed-payload primitives
    // (mcp-click/mcp-action) cannot do (FUP-068).
    function payload(regionEl) {
      var instance = regionEl && regionEl.getAttribute ? (regionEl.getAttribute('data-st-instance') || '') : '';
      var values = (window.ST && ST._mcpCollectValues) ? ST._mcpCollectValues(regionEl) : {};
      return {
        action: action,
        values: values,
        instance: instance,
        at: new Date().toISOString()
      };
    }

    function onClick() {
      var regionEl = resolveRegion();
      var host = regionEl ? regionEl.__stHost : null;
      if (!host || !host.signal) {
        sent = false;
        error = 'MCP host bridge is not installed on the nearest [data-st-instance]';
        yieldState();
        return;
      }
      host.signal('mcp-action', payload(regionEl), action).then(function(message) {
        sent = true;
        lastMessage = message;
        error = null;
        yieldState();
      }).catch(function(err) {
        sent = false;
        error = err && err.message ? err.message : String(err);
        yieldState();
      });
    }

    yieldState();
    installClickHandler(targetEl, handlerKey, onClick);

    %cleanup {
      removeClickHandler(targetEl, handlerKey, onClick);
    }
  }

  %exports {
    $sent: bool
    $lastMessage: object
    $error: string
  }
}

%primitive mcp-input(refresh: number = 0) {
  %emit js {
    var data = {};
    var loading = true;
    var error = null;
    var refreshMs = Number('%refresh' || 0);
    var timer = null;

    // PLAN-064 B4a P2: the workbench poll republishes EVERY source each interval
    // (every 1s). Re-publishing an UNCHANGED source re-runs its `@each`/binding
    // consumers, which rebuilds editable input rows and can RESET a field the user
    // is mid-typing. Guard with a per-source fingerprint: skip the publish when the
    // JSON-serialized value is byte-identical to the last one. This makes an
    // unchanged poll a true no-op, so ALL workbench editors (per-element bindings,
    // node-params, frame params) survive the poll while focused — and it trims
    // needless re-renders across the whole workbench. A genuine change still
    // publishes + repaints immediately.
    var __lastPublished = {};
    function setSource(name, value) {
      var body = document.body || document.documentElement;
      var fp;
      try { fp = JSON.stringify(value); } catch (e) { fp = null; }
      // Only skip when we have a comparable fingerprint AND it is unchanged.
      if (fp !== null && __lastPublished[name] === fp) return;
      __lastPublished[name] = fp;
      if (window.ST && ST.set && body) ST.set(body, name, value);
      if (window.ST && ST.setData) ST.setData(name, value);
    }

    function publishWorkbenchSources() {
      var env = data.environments || {};
      setSource('mcpDiscoveredEnvironments', env.discovered || []);
      setSource('mcpOpenEnvironments', env.open || []);
      setSource('mcpTabs', data.tabs || []);
      // PLAN-049 W2 (COLLAPSE): flattened `.st` entries across open envs feeding
      // the open-env card's one-click entry picker (each row carries its env).
      setSource('mcpEnvEntries', data.env_entries || []);
      setSource('mcpFunctions', data.functions || []);
      // M2 gallery: composable guests only (server filters out the host page +
      // uncompilable fns). Falls back to the full list if absent (older server).
      setSource('mcpGalleryFunctions', data.gallery_functions || data.functions || []);
      setSource('mcpContractParams', data.contract_params || []);
      // M3' in-stage selection: the guest composed on the stage right now + its
      // contract params. The inspector binds these when a frame is selected, so
      // it reflects the SELECTED frame rather than the whole registry. `stage_mount`
      // is null when the stage is empty — publish empty shapes then.
      var stageMount = data.stage_mount || null;
      setSource('mcpStageMountName', stageMount ? (stageMount.name || stageMount.function_id || '') : '');
      setSource('mcpStageParams', stageMount ? (stageMount.contract_params || []) : []);
      // PLAN-049 W4.S8: the frame strip (regions composed on the stage host).
      setSource('mcpStageFrames', data.stage_frames || []);
      // PLAN-051 B1: the stage STRUCTURE — the composed guest's template graph
      // (flat depth-tagged nodes), feeding the builder navigator. Empty when the
      // stage is empty or the guest fails to compile.
      setSource('mcpStageStructure', data.stage_structure || []);
      // PLAN-051 B4: the FLAT per-node param rows (each with its node_id +
      // invoke_span), feeding the inspector's editable per-instance param widgets.
      // The inspector filters to the SELECTED node's rows.
      setSource('mcpStageNodeParams', data.stage_node_params || []);
      // PLAN-064 B4a: per-element binding edit rows (the mcp_param value rail).
      setSource('mcpStageNodeBindings', data.stage_node_bindings || []);
      // M6: the selected guest's source, for the inline source editor.
      setSource('mcpStageSource', stageMount ? (stageMount.source || '') : '');
      setSource('mcpInstances', data.instances || []);
      // M5: the comms feed binds the agent-audience conversation (newest first),
      // not the full control-plane event log. Falls back to all events if the
      // server doesn't provide the filtered list (older server).
      setSource('mcpEvents', data.comms_events || data.events || []);
      // Kit sources (PLAN-043): a mounted kit function (picker/confirm/form)
      // carries its shape in the SAME input.json. Publishing these generic
      // scalars/arrays lets a kit page render with the proven `text:`/`@each`
      // idiom — no per-kit JS, one input primitive for workbench AND kit.
      setSource('mcpPrompt', data.prompt || '');
      setSource('mcpOptions', data.options || []);
      setSource('mcpFields', data.fields || []);
      setSource('mcpReviewBody', data.body || '');
    }

    // BUG-156: %yield's ST.set(el, name, value) call ONLY dispatches the
    // body-scope `local:<name>:updated` event (which @data derive/@each's
    // dependency-watch listen for) when the exported name is in
    // ST._bridgedExports -- an allowlist the COMPILER populates per-page, ONLY
    // for names some markup directly interpolates (src/emit/html_hydrate.rs).
    // A page that ONLY derives/@each's off $mcpInput (never interpolates it
    // directly) never gets bridged, so the derive/each silently never fires --
    // contingent on unrelated sibling markup, which is not a real contract.
    // $mcpInput/$mcpInputLoading/$mcpInputError are this primitive's ONLY two
    // call sites' aliases (mcp-input / mcp-input-refresh, both -> the same three
    // names) -- bridge them unconditionally so every consumer path (direct
    // interpolation, @data derive, @each) works identically regardless of what
    // else is on the page.
    if (typeof ST !== 'undefined' && ST._bridgeExport) {
      ST._bridgeExport('mcpInput');
      ST._bridgeExport('mcpInputLoading');
      ST._bridgeExport('mcpInputError');
    }

    function yieldState() {
      %yield data -> $data;
      %yield loading -> $loading;
      %yield error -> $error;
    }

    function loadInput() {
      loading = true;
      yieldState();
      return fetch('input.json', { cache: 'no-store' }).then(function(res) {
        if (!res.ok) throw new Error('input.json ' + res.status);
        return res.json();
      }).then(function(body) {
        data = body && body.input ? body.input : {};
        loading = false;
        error = null;
        yieldState();
        publishWorkbenchSources();
        return data;
      }).catch(function(err) {
        data = {};
        loading = false;
        error = err && err.message ? err.message : String(err);
        yieldState();
        publishWorkbenchSources();
        return data;
      });
    }

    yieldState();
    publishWorkbenchSources();
    loadInput();
    if (refreshMs > 0) {
      timer = setInterval(loadInput, refreshMs);
    }

    %cleanup {
      if (timer) clearInterval(timer);
    }
  }

  %exports {
    $data: object
    $loading: bool
    $error: string
  }
}

/// Install the MCP host bridge on the mounted function's region root.
///
/// Selector-scoped over `[data-st-instance]` (or any descendant): resolves the
/// NEAREST ancestor region root and attaches one `__stHost` bridge per region.
/// No page-global singleton — two regions on one page carry independent bridges.
%macro mcp-host {
  %scope selector
  %order 50

  %form {
    @mcp-host
  }

  %binds {
    mcp-host(&self) -> {
      $connected as $mcpConnected,
      $lastMessage as $mcpLastMessage,
      $error as $mcpError,
      $actionText as $mcpActionText,
      $actionOk as $mcpActionOk
    }
  }
}

/// Load the mounted instance's `input.json` payload into `$mcpInput`.
%macro mcp-input {
  %scope file
  %order 51

  %form {
    @mcp-input
  }

  %binds {
    mcp-input(refresh: 0) -> {
      $data as $mcpInput,
      $loading as $mcpInputLoading,
      $error as $mcpInputError
    }
  }
}

/// Load `input.json` and poll for state changes every `refresh` milliseconds.
%macro mcp-input-refresh {
  %scope file
  %order 51

  %form {
    @mcp-input(refresh: $refresh:number = 0)
  }

  %binds {
    mcp-input(refresh: $refresh) -> {
      $data as $mcpInput,
      $loading as $mcpInputLoading,
      $error as $mcpInputError
    }
  }
}

/// Emit an MCP event when the current selector is clicked.
%macro mcp-click {
  %scope selector
  %order 60

  %form {
    @mcp-click(type: $eventType:string = "clicked", correlation: $correlation:string = "", value: $value:string = "")
  }

  %binds {
    mcp-click-event(&self, eventType: $eventType, correlation: $correlation, value: $value) -> {
      $sent as $mcpClickSent,
      $lastMessage as $mcpClickLastMessage,
      $error as $mcpClickError
    }
  }
}

/// Emit a semantic MCP action request when the current selector is clicked.
///
/// S0 actions are event envelopes (`type = "mcp-action"`) consumed by the agent
/// or a later workbench dispatcher. They intentionally share the one generic
/// `/__mcp/signal/{instance}` sink instead of creating per-action HTTP routes.
%macro mcp-action {
  %scope selector
  %order 61

  %form {
    @mcp-action(action: $action:string, target: $target:string = "", value: $value:string = "", targetAttr: $targetAttr:string = "", valueAttr: $valueAttr:string = "", on: $on:string = "click")
  }

  %binds {
    mcp-action-event(&self, action: $action, target: $target, value: $value, targetAttr: $targetAttr, valueAttr: $valueAttr, on: $on) -> {
      $sent as $mcpActionSent,
      $lastMessage as $mcpActionLastMessage,
      $error as $mcpActionError
    }
  }
}

/// Emit an MCP action that COLLECTS the region's named controls (FUP-068).
///
/// On click, gathers every `input[name]` / `select[name]` / `textarea[name]`
/// inside the nearest `[data-st-instance]` region into `payload.values =
/// { name: value }` (checkbox/radio -> boolean), then emits `mcp-action` with
/// `payload.action` through the SAME generic `/__mcp/signal/{instance}` sink.
/// This is the form-read step the fixed-payload `@mcp-click`/`@mcp-action`
/// cannot do; the kit `form`/`review` functions use it so a submit/approve
/// carries the human's typed input.
%macro mcp-submit {
  %scope selector
  %order 62

  %form {
    @mcp-submit(action: $action:string = "kit-submit")
  }

  %binds {
    mcp-submit-event(&self, action: $action) -> {
      $sent as $mcpSubmitSent,
      $lastMessage as $mcpSubmitLastMessage,
      $error as $mcpSubmitError
    }
  }
}

