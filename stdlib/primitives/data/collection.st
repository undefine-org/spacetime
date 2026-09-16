/// Collection Primitive (FEAT-075)
///
/// Backs `@data collection $rows T[] from "url" via dev-ws { initial: [] }`: a reactive
/// array signal PLUS three auto-persisting mutators registered on ST._collections[name]:
///   .insert(item, index?)  .delete(indexOrPredicate)  .reorder(fromIndex, toIndex)
///
/// Each mutator collapses the optimistic-CRUD pattern that the CMS admin hand-wired at
/// FOUR sites (and got WRONG — none restored the local model on reject):
///   1. SNAPSHOT the current array (the exact pre-op state)
///   2. OPTIMISTICALLY apply to the local signal -> ST.setData re-renders @each immediately
///   3. SEND the EditJsonArray op over the named transport with a fresh op_id
///   4. on Ack: confirm (nothing to undo); on Reject/timeout: REVERT to the snapshot and
///      raise $<name>_error. Concurrent ops each carry their own snapshot, so a failed op
///      reverts independently.
///
/// `via dev-ws` is the FIRST transport; the WS bridge (window.__stDevWs) is reused, not
/// duplicated. The `transport` seam lets a production REST/PATCH transport drop in later
/// without changing author syntax.
///
/// @param name      - the collection signal name (without `$`)
/// @param file      - the backing data file (the `from "..."` source path)
/// @param transport - persistence transport id ("dev-ws")
/// @param initial   - initial array value when the source has not yet loaded

%primitive collection(
  name: string = "",
  file: string = "",
  transport: string = "dev-ws",
  initial: expr = []
) %uses data-channel {
  %emit js {
    (() => {
      const name = %name;
      const file = %file;
      const initial = %initial;
      if (!name) return;

      // Seed the array signal if nothing has loaded it yet (the data-source bind, if
      // present, will overwrite with the fetched value).
      if (typeof ST !== 'undefined' && ST.getData) {
        if (ST.getData(name) === undefined || ST.getData(name) === null) {
          ST.setData(name, Array.isArray(initial) ? initial : []);
        }
      }

      const read = () => {
        // Prefer the global signal store (the dual channel's authoritative half,
        // also written by a direct `$name <- ...` mutation) so a mutator always
        // plans against the CURRENT array; fall back to the data registry.
        let v = (typeof window !== 'undefined' && window.SpacetimeLocal)
          ? window.SpacetimeLocal[name] : undefined;
        if (!Array.isArray(v)) {
          v = (typeof ST !== 'undefined' && ST.getData) ? ST.getData(name) : null;
        }
        return Array.isArray(v) ? v : [];
      };
      // Publish through the DUAL channel (mirrors data-source / reactive-source):
      //   1. ST.setData(name, arr) -> the data registry, so @each + afterData
      //      consumers re-render.
      //   2. SpacetimeLocal[name] + a deferred `local:<name>:updated` event -> the
      //      global signal store, so @data derive / @data query / file-scope
      //      bindings that read `$name` re-run. WITHOUT this half a collection
      //      mutation (.insert/.update/...) never reached a derive or query
      //      sourced from the collection (the two data planes were desynced).
      const publish = (arr) => {
        // Dual channel owned by the data-channel module (`%uses data-channel`).
        window.__stPublish(name, arr);
      };
      const raiseError = (reason) => {
        const msg = (reason && reason.reason) ? reason.reason : (reason || 'persist failed');
        if (typeof window !== 'undefined') {
          window._localState = window._localState || {};
          window._localState[name + '_error'] = msg;
          if (window.SpacetimeLocal && (name + '_error') in window.SpacetimeLocal) {
            window.SpacetimeLocal[name + '_error'] = msg;
          }
          document.dispatchEvent(new CustomEvent('local:' + name + '_error:updated', { detail: msg }));
        }
      };

      // Transport: send an EditJsonArray op + register per-op ack/reject. Returns the op_id.
      // dev-ws reuses window.__stDevWs (the singleton bridge) — no WS logic duplicated here.
      const send = (op, onReject) => {
        const ws = (typeof window !== 'undefined') ? window.__stDevWs : null;
        if (!ws || !ws.connected) {
          // No live transport (e.g. production build): the optimistic apply stands; there is
          // nothing to revert against and no ack to wait for.
          return null;
        }
        const opId = 'op_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
        ws.send({ type: 'EditJsonArray', file: file, path: '', op: op, op_id: opId });
        if (ws.onReject) ws.onReject(opId, (reason) => onReject(reason));
        return opId;
      };

      // One mutator template. `plan(arr)` returns { next, op, inverse } where:
      //   next    = the new array after optimistically applying the op (or undefined to bail)
      //   op      = the EditJsonArray op to persist
      //   inverse = (curr) => arr that UNDOES this op, applied to the CURRENT array at reject
      //             time — NOT a stale whole-array snapshot. So a rejected earlier op reverts
      //             only ITSELF and preserves later in-flight optimistic ops (spec: 'revert
      //             only the failed op, not the whole queue').
      const mutate = (plan) => {
        const planned = plan(read().slice());
        if (!planned) return; // no-op mutator (e.g. out-of-range) bails
        publish(planned.next);
        send(planned.op, (reason) => { publish(planned.inverse(read().slice())); raiseError(reason); });
      };

      const coerceIndex = (v) => (typeof v === 'number' ? v : (v == null ? NaN : Number(v)));

      ST._collections = ST._collections || {};
      ST._collections[name] = {
        insert: (item, index) => mutate((arr) => {
          const idx = (index === undefined || index === null) ? arr.length : coerceIndex(index);
          arr.splice(idx, 0, item);
          return {
            next: arr,
            op: { op: 'Insert', item: item, index: (index === undefined ? null : index) },
            // Undo: remove the item we inserted at idx (by identity if still there).
            inverse: (curr) => {
              const at = curr.indexOf(item);
              if (at !== -1) curr.splice(at, 1);
              else if (idx >= 0 && idx < curr.length) curr.splice(idx, 1);
              return curr;
            },
          };
        }),
        delete: (indexOrPred) => mutate((arr) => {
          let idx = typeof indexOrPred === 'function' ? arr.findIndex(indexOrPred) : coerceIndex(indexOrPred);
          if (typeof idx !== 'number' || isNaN(idx) || idx < 0 || idx >= arr.length) {
            console.warn('[ST] collection.delete: index out of range', indexOrPred); return null;
          }
          const removed = arr[idx];
          arr.splice(idx, 1);
          return {
            next: arr,
            op: { op: 'Delete', index: idx },
            // Undo: re-insert the removed item at its original index.
            inverse: (curr) => { curr.splice(Math.min(idx, curr.length), 0, removed); return curr; },
          };
        }),
        update: (idOrPred, patch) => mutate((arr) => {
          // Find the row to patch: a function predicate, else match by `id`
          // field equality (the common case — `$cards.update(cardId, {col})`).
          const idx = (typeof idOrPred === 'function')
            ? arr.findIndex(idOrPred)
            : arr.findIndex((it) => it && it.id === idOrPred);
          if (idx < 0 || idx >= arr.length) {
            console.warn('[ST] collection.update: no row matches', idOrPred); return null;
          }
          if (!patch || typeof patch !== 'object') {
            console.warn('[ST] collection.update: patch must be an object', patch); return null;
          }
          const prev = arr[idx];
          // Snapshot ONLY the keys this patch overwrites, so the inverse restores
          // exactly those fields (and removes any the patch newly added).
          const restore = {};
          for (const k in patch) {
            if (!Object.prototype.hasOwnProperty.call(patch, k)) continue;
            restore[k] = Object.prototype.hasOwnProperty.call(prev, k) ? prev[k] : undefined;
          }
          arr[idx] = Object.assign({}, prev, patch);
          return {
            next: arr,
            op: { op: 'Update', index: idx, patch: patch },
            // Undo: re-apply the captured prior values at the row's CURRENT index
            // (found by id, robust to a concurrent reorder). undefined => delete.
            inverse: (curr) => {
              const at = (prev && prev.id != null)
                ? curr.findIndex((it) => it && it.id === prev.id)
                : idx;
              if (at < 0 || at >= curr.length) return curr;
              const merged = Object.assign({}, curr[at]);
              for (const k in restore) {
                if (restore[k] === undefined) delete merged[k];
                else merged[k] = restore[k];
              }
              curr[at] = merged;
              return curr;
            },
          };
        }),
        reorder: (from0, to0) => mutate((arr) => {
          const from = coerceIndex(from0), to = coerceIndex(to0);
          if (isNaN(from) || isNaN(to) || from < 0 || from >= arr.length || to < 0 || to >= arr.length) {
            console.warn('[ST] collection.reorder: out of range', from0, to0); return null;
          }
          const moved = arr.splice(from, 1)[0];
          arr.splice(to, 0, moved);
          return {
            next: arr,
            op: { op: 'Reorder', from_index: from, to_index: to },
            // Undo: move it back (to -> from).
            inverse: (curr) => {
              if (to < 0 || to >= curr.length) return curr;
              const m = curr.splice(to, 1)[0];
              curr.splice(Math.min(from, curr.length), 0, m);
              return curr;
            },
          };
        }),
      };
    })();
  }
}
