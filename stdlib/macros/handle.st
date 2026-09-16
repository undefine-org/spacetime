/// @handle — the reactive-output CONSUMER directive (PLAN-038 W1, pillar 4).
///
/// `@handle $sig { optimistic{} receive{} final{} }` consumes a `@data signal`
/// in THIS scope: it re-opens the signal's clauses (nearest-scope cascade), so
/// one signal can have many handlers, each owning its scope's behavior. A fire
/// resolves to its nearest enclosing `@handle`; absent one, the signal def's
/// own clauses apply.
///
///   .quick-add {
///     @handle $add {
///       optimistic { $before <- $todos; $todos <- $todos.concat({ title: $draft, pending: true }); }
///       receive {
///         Created(todo) => { $todos <- $todos.replace(todo); }
///         Invalid(errs) => { $todos <- $before; $errors <- errs; }
///       }
///       final { $draft <- ""; }
///     }
///   }
///
/// All three clauses are OPTIONAL — a bare `@handle $sig {}` adopts the def
/// behavior. The clauses are capture-grammar (handle_optimistic / handle_receive
/// / handle_final), so the CST routes a `@handle` body to a RAW body (the
/// `Variant(b) => { … }` arms are not CST statements). `@handle` is selector- or
/// file-scoped and binds the `signal-handler` primitive, which subscribes to the
/// signal's reply event and runs the matching arm's effect statements.
///
/// @each : collection-in-space  ::  @handle : signal-responses-in-time.
%macro handle {
  %pipeline-consumed
  %scope file | selector
  %order 700

  %form {
    @handle $signal:binding {
      ( $optimistic:handle_optimistic )?
      ( $receive:handle_receive )?
      ( $final:handle_final )?
    }
  }

  %binds {
    signal-handler(
      &self,
      signal: $signal,
      optimistic: $optimistic,
      arms: $arms,
      sum: $sum,
      final: $final
    ) -> { $handled }
  }
}
