/// When-Guard Primitive (BUG-088)
///
/// Runs a mutation body whenever a watched element-local signal becomes truthy.
/// The declarative bridge between a primitive that *yields* an element-scoped
/// signal (e.g. `@scroll-spy` → `$isActive`) and the state mutations a section
/// wants to perform while that signal holds:
///
///   .work-section {
///     @scroll-spy
///     @when &self.scrollSpy.isActive { $navTheme <- "dark"; }
///   }
///
/// Before this, `@when &self.<ns>.<sig>` was a documented-but-unimplemented form:
/// the guard body was silently dropped (no listener, no mutation). Now it lowers
/// to an `ST.watch(el, sig)` whose callback runs the SAME mutation grammar as an
/// `@on` handler, via the shared `ST.runMutations` helper.
///
/// The `scrollSpy.` / `<ns>.` prefix is purely cosmetic documentation of which
/// primitive owns the signal — only the FINAL path segment is the real signal
/// name (`isActive`), matching how the yielding primitive stores it with
/// `ST.set(el, "isActive", …)`.
///
/// @param &el    - the host element whose signal is watched
/// @param signal - the dotted signal path; the last segment is the signal name
/// @param body   - captured mutation_actions { js_statements, macro_calls }

%primitive when-guard(
  &el,
  signal: string = "",
  body: expr
) {
  %emit js {
    (() => {
      const host = %&el;
      const path = %signal;
      const body = %body;
      if (!host || !path) return;

      // Only the final segment is the real signal name: `scrollSpy.isActive`
      // watches the `isActive` signal the owning primitive sets on this element.
      const sig = String(path).split('.').pop();
      if (!sig) return;

      if (typeof ST === 'undefined' || !ST.watch) return;

      // Run the mutation body each time the signal becomes truthy. ST.watch fires
      // immediately with the current value (so a section already active at mount
      // themes correctly) and on every subsequent change.
      const unwatch = ST.watch(host, sig, (v) => {
        if (!v) return;
        try { ST.runMutations(host, body); }
        catch (e) { console.error('[ST] @when guard body error:', e); }
      });

      if (ST.onCleanup) {
        ST.onCleanup(host, () => { if (unwatch) unwatch(); });
      }
    })();
  }
}
