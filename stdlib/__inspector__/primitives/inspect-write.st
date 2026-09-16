/// Inspector EditAst transport (PLAN-112 W3).
///
/// The panel owns the values; this primitive only serializes ONE guarded write
/// onto the dev WebSocket that already exists on every dev page. It is the sole
/// place JS may live in this module (stdlib/AGENTS.md), and it deliberately does
/// nothing clever: no retry, no queue, no local state patching. The server writes
/// the file and the file watcher reloads the page — the migrations-pill rail.
///
/// GUARD STYLE (BUG-212, FIXED 2026-07-25): the guards here are POSITIVE-condition
/// blocks. That was once REQUIRED — a top-level bodyless `if (cond) return;`
/// followed by an arrow declaration was silently mis-emitted (guard inverted,
/// `return` deleted, declaration dropped). The emitter no longer does that, so the
/// style is now a preference rather than a constraint.
%primitive inspect-write(&el, signal: string = "", entry: string = "") {
  %emit js {
    (() => {
      const host = %&el;
      // %exports publishes through %yield — an `ST.set(host, …)` here writes a
      // LOCAL value that never reaches the bound signal, which is why the panel's
      // Saved/error line stayed blank on every write (both rails).
      let statusText = '';
      let errorText = '';
      const sig = %signal;
      const entryFile = %entry;
      if (sig) {
        const sigHead = sig.split('.')[0];
        let lastSerialized = '';
        const fire = () => {
          const request = ST.resolve(host, sigHead);
          // Re-entry guard: the watch fires for every write to the signal, and a
          // status yield below is itself a signal write. Only act when the
          // REQUEST actually changed.
          const serialized = request ? JSON.stringify(request) : '';
          const changed = serialized !== '' && serialized !== lastSerialized;
          if (changed) {
            lastSerialized = serialized;
            // `file` is empty for a row whose span indexes the ENTRY's own
            // source — the projection only names a file for an invocation that
            // came from an IMPORTED one. Resolving that here is what makes an
            // ordinary single-file page editable at all.
            const targetFile = request.file || entryFile;
            const ws = window.__stDevWs;
            const addressed = !!(targetFile && request.param && request.source_hash && request.span_text);
            const connected = !!(ws && ws.connected);
            if (addressed && connected) {
              const opId = 'ins-' + Date.now() + '-' + Math.random().toString(36).slice(2);
              ws.onAck(opId, () => {
                errorText = ''; %yield errorText -> $error;
                statusText = 'Saved'; %yield statusText -> $status;
              });
              ws.onReject(opId, (message) => {
                const reason = (message && (message.reason || message.error || message.message)) || 'Edit rejected';
                errorText = reason; %yield errorText -> $error;
                statusText = ''; %yield statusText -> $status;
              });
              const patch = {
                __invoke_span: [Number(request.start), Number(request.end)],
                __expect_hash: request.source_hash,
                __expect_span_text: request.span_text
              };
              // BARE value: the server's patcher re-quotes to match what the
              // source had (`emit_patch_value`). Quoting here double-quotes it.
              patch[request.param] = request.value;
              ws.send({ type: 'EditAst', file: targetFile, selector: '', op_id: opId, patch: patch });
              errorText = ''; %yield errorText -> $error;
              statusText = 'Saving…'; %yield statusText -> $status;
            }
            if (addressed && !connected) {
              errorText = 'Dev WebSocket is disconnected'; %yield errorText -> $error;
              statusText = ''; %yield statusText -> $status;
            }
            // An UNADDRESSED request is a display-only row (its snapshot could
            // not be verified, so it carries no hash): silently ignored, never
            // written — the stylesheet already renders those inert.
          }
        };
        // ST.watchScoped ALREADY subscribes the global `local:<sig>:updated`
        // channel as well as every element scope (see its own implementation), so
        // a second document listener here would fire `fire` TWICE per change.
        // Its return value is the unsubscribe — dropping it leaves a detached
        // watcher alive after the panel is torn down, which can re-send a write.
        let stopWatching = null;
        if (ST.watchScoped) { stopWatching = ST.watchScoped(host, sigHead, fire); }
        %cleanup { if (stopWatching) { stopWatching(); } }
      }
    })();
  }
  %exports {
    $error: string
    $status: string
  }
}

%macro inspector-write {
  %form { @inspector-write(on: $signal:binding, entry: $entry:string = "") }
  %binds {
    inspect-write(&self, signal: $signal, entry: $entry) -> {
      $error as $insWriteError
      $status as $insWriteStatus
    }
  }
}
