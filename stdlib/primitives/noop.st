/// `none` — the no-runtime marker primitive (BUG-268).
///
/// Some registry declarations positively state that a construct has "nothing
/// to start" — the canonical case is a driver whose `%registers` declares
/// `primitive: none` (`driver-clip` in stdlib/macros/drivers.st): a clip has no
/// clock of its own, its progress IS the window its parent score assigned, so
/// the drive expansion names `none` as the primitive. That is a RESOLUTION, not
/// a lookup failure. This empty primitive makes the resolution honest: the
/// expand layer finds it, emits nothing, and the bundle ships clean.
///
/// It deliberately has no `%emit` body and no parameters — `none` never
/// produces runtime JS or CSS.
%primitive none() {
}

/// `noop` — the no-runtime declaration marker (BUG-268).
///
/// Declaration macros that register compile-time data bind a no-op so the
/// declaration stays observable as a match without inventing emit behaviour:
/// `@form …` binds `noop(&self, decl: $name)` (stdlib/macros/form.st), keeping
/// the declaration visible to the form registry while emitting no runtime code.
///
/// Like `none`, it has no `%emit` body: a `noop` is a marker, never output.
%primitive noop() {
}
