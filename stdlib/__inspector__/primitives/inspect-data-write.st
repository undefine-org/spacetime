/// Inspector EditJson transport — the `@data` half of the pill's write rail.
///
/// A template-less page has no invocation to edit: its editable surface is the
/// JSON records behind `@each`. Those live in a JSON file, addressed by
/// (source, index, field) — NOT by `.st` byte spans, so they have no source hash
/// to guard and cannot use the EditAst rail in `inspect-write.st`.
///
/// This is deliberately the SAME `EditJson` message the admin CMS already sends
/// (`ClientMessage::EditJson { file, path, value, op_id }`, handled in
/// src/sync/handlers.rs), so the server-side path validation, JSON-path update,
/// write-back and ack/reject protocol all apply unchanged. Nothing about the
/// persistence layer is new here; only the address is.
///
/// Kept as a sibling primitive rather than a branch inside `inspect-write`: the
/// two rails carry different addresses and different guarantees, and folding them
/// together would mean one code path deciding, per event, which kind of write it
/// is — exactly the sort of overload that hides a mis-targeted write.
///
/// GUARD STYLE (BUG-212, FIXED 2026-07-25): the guards here are POSITIVE-condition
/// blocks. That was once REQUIRED — a top-level bodyless `if (cond) return;`
/// followed by an arrow declaration was silently mis-emitted (guard inverted,
/// `return` deleted, declaration dropped). The emitter no longer does that, so the
/// style is now a preference rather than a constraint.
%primitive inspect-data-write(&el, signal: string = "") {
  %emit js {
    (() => {
      const host = %&el;
      // %exports publishes through %yield — an `ST.set(host, …)` here writes a
      // LOCAL value that never reaches the bound signal, which is why the panel's
      // Saved/error line stayed blank on every write (both rails).
      let statusText = '';
      let errorText = '';
      const sig = %signal;
      if (sig) {
        const sigHead = sig.split('.')[0];
        let lastSerialized = '';
        const fire = () => {
          const request = ST.resolve(host, sigHead);
          // Re-entry guard: a status yield below is itself a signal write, so act
          // only when the REQUEST actually changed (mirrors inspect-write).
          const serialized = request ? JSON.stringify(request) : '';
          const changed = serialized !== '' && serialized !== lastSerialized;
          if (changed) {
            lastSerialized = serialized;
            const ws = window.__stDevWs;
            const addressed = !!(request.file && request.path);
            const connected = !!(ws && ws.connected);
            if (addressed && connected) {
              const opId = 'insdata-' + Date.now() + '-' + Math.random().toString(36).slice(2);
              ws.onAck(opId, () => {
                errorText = ''; %yield errorText -> $error;
                statusText = 'Saved'; %yield statusText -> $status;
              });
              ws.onReject(opId, (message) => {
                const reason = (message && (message.reason || message.error || message.message)) || 'Edit rejected';
                errorText = reason; %yield errorText -> $error;
                statusText = ''; %yield statusText -> $status;
              });
              ws.send({
                type: 'EditJson',
                file: request.file,
                path: request.path,
                value: request.value,
                op_id: opId
              });
              errorText = ''; %yield errorText -> $error;
              statusText = 'Saving…'; %yield statusText -> $status;
            }
            if (addressed && !connected) {
              errorText = 'Dev WebSocket is disconnected'; %yield errorText -> $error;
              statusText = ''; %yield statusText -> $status;
            }
            // An unaddressed request (no backing file in the data registry, or a
            // derived/ambiguous field that was never stamped) is silently ignored,
            // never written.
          }
        };
        // ST.watchScoped already subscribes the global channel as well as every
        // element scope, so a second document listener would fire twice. Its
        // return value is the unsubscribe — dropping it leaves a detached watcher
        // alive after teardown, which can re-send a write.
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

%macro inspector-data-write {
  %form { @inspector-data-write(on: $signal:binding) }
  %binds {
    inspect-data-write(&self, signal: $signal) -> {
      $error as $insDataWriteError
      $status as $insDataWriteStatus
    }
  }
}

/// The pill needs a LIVE dev WebSocket to write with. `dev-ws` is a shared
/// primitive (`stdlib/__dev__/primitives/dev-ws.st`) that installs the singleton
/// bridge on `window.__stDevWs`, but a primitive only runs when something BINDS
/// it — importing the file is not enough. The inspector imported it and never
/// bound it, so on an ordinary `spacetime serve` no bridge existed and every
/// write reported "Dev WebSocket is disconnected".
///
/// The admin tree instantiates it through its own `@dev-bridge` macro, which also
/// composes `dev-save` and lives under `__admin__` — binding the primitive here
/// keeps the pill self-contained rather than reaching across trees for a macro
/// that carries state the pill does not use. The bridge itself is a singleton, so
/// a second binding reuses the same socket.
%macro inspector-dev-bridge {
  %form { @inspector-dev-bridge }
  %binds {
    dev-ws() -> {
      $connected as $insWsConnected
      $error as $insWsError
    }
  }
}
