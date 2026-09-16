/// `@drop-zone` — a first-class place draggables can be dropped.
///
/// @example basic
/// [data-col="doing"] {
///   @drop-zone(group: "cards") as $zone {
///     over { background: rgba(255,255,255,0.04) }   // while a card hovers
///   }
/// }
///
/// @example accept / reject
/// [data-col="done"] {
///   @drop-zone(group: "cards") as $zone {
///     accepts: $payload.subtasksOpen == 0;          // predicate over the dragged datum
///     over   { background: rgba(110,231,183,0.06) } // hovering AND accepted
///     reject { outline: 1px solid var(--bad) }      // hovering but NOT accepted
///   }
/// }
///
/// `group` namespaces zones↔draggables (default ""). `as $zone` names the zone
/// descriptor; the drag's lifecycle clauses read its dataset as `$zone.col`
/// (← `data-col`). `accepts:` is an optional predicate over `$payload` (the
/// dragged datum) evaluated by the gesture at hover. `over` / `reject` are
/// %states gated by the live `$hovering` / `$accepting` signals.

%macro drop-zone {
  %form {
    @drop-zone(group: $group:string = "") as $zone:ident {
      accepts: $accepts:balanced(';')?
      $customStates:states?
    }
  }

  %binds {
    dnd-runtime() -> { }
    drop-zone(&self, group: $group, accepts: $accepts) -> {
      $hovering,
      $accepting,
      $over,
      $reject
    }
  }

  %states {
    // `over` / `reject` gate on the zone's COMBINED signals (computed in the
    // primitive), not a compound `$hovering && $accepting` — the state machinery
    // (ST.bindState) keys on a single signal name, not an expression.
    over when $over {
    }

    reject when $reject {
    }

    // Merge the caller's over/reject (and any extra) state bodies.
    $customStates?
  }
}
