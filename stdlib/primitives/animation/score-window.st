/// Score Window — one clip's progress, derived from its parent's (SIP-001 W4/R3)
///
/// A score adds NO runtime concept. Every driver already publishes exactly one
/// 0..1 progress signal, so a clip's progress is a pure FUNCTION of the score's
/// progress — an affine window with a clamp:
///
///   &lower-third at 2s for 3s        // inside @score &.time(60s)
///   child = clamp01((parent * 60000 - 2000) / 3000)
///
/// `at` is the offset, `for` is the span, and both are expressed in the
/// score's own DOMAIN (W4/R1: the driver owns it — milliseconds under
/// `.time`, a 0..1 ratio under `.scroll`). `total` carries the scale that
/// converts the parent's 0..1 into that domain, so this primitive never needs
/// to know WHICH driver is upstream. That is what lets one score body run
/// under `&.time` on a page and `&.clip` under render — the thesis, reduced to
/// an argument.
///
/// WHY A DERIVATION AND NOT A SCHEDULER
///
/// A scheduler would hold a clip list and tick it. Two consumers (live rAF and
/// deterministic render) would then need two schedulers kept in agreement,
/// which is exactly how live and rendered films diverge (SIP-001b §8.6).
/// Because this is a pure map, seeking is just evaluating it at another input:
/// the score is seekable BY CONSTRUCTION, so render determinism (pressure test
/// C4) falls out instead of being enforced.
///
/// @param &el      - the scope node the signals resolve through
/// @param name     - the published signal name for this clip's progress
/// @param parent   - the score's own progress signal name (0..1)
/// @param offset   - when this clip starts, in the score's domain
/// @param span     - how long it lasts, in the score's domain (0 = to the end)
/// @param total    - the score's full extent in its own domain (the scale)
/// @param offset_rel - when 1, `offset` is a FRACTION of `total` and is scaled
///                   by the resolved total AT SAMPLE TIME (BUG-270: only set
///                   under a runtime-total driver such as `.playback`, whose
///                   total is the media duration). Compile-time totals keep the
///                   constant folding in score.rs and emit this as 0.
/// @param span_rel - the `for` counterpart of `offset_rel`.
/// @param rate_from - progress rate at the window's start (1 = normal speed)
/// @param rate_to   - progress rate at its end; equal to rate_from = constant
///
/// @yields $<name> - this clip's progress (0-1)
%primitive score-window(
  &el,
  name: string = "",
  parent: string = "",
  offset: number = 0,
  span: number = 0,
  total: number = 1,
  offset_rel: number = 0,
  span_rel: number = 0,
  rate_from: number = 1,
  rate_to: number = 1
) {
  %emit js {
    var _swName = %name;
    var _swOffset = %offset;
    var _swSpan = %span;
    var _swTotal = %total;
    var _swOffsetRel = %offset_rel;
    var _swSpanRel = %span_rel;
    var _swRateFrom = %rate_from;
    var _swRateTo = %rate_to;

    // `total` is the SCALE that converts the parent's 0..1 into the score's
    // domain — normally a compile-time constant (`.time`: the declared ms;
    // `.scroll`/`.steps`: the normalized unit). Under `.playback` it is the
    // MEDIA element's duration, which the media-driver publishes as a signal
    // and this primitive cannot know until runtime, so `total` may arrive as
    // a SIGNAL NAME rather than a number and is resolved per sample. A span
    // of 0 means "until the end of the score" — the honest reading of a
    // clip with an `at` but no `for". Extent is therefore derived per sample
    // too: a runtime scale can change while the score runs (a media element's
    // duration is unknown until its metadata loads).

    // THE WINDOW MAP. Pure: same input, same output, no clock of its own and
    // no state carried between samples. Everything the score body can say is
    // a shape of this function — `at`/`for` set offset and extent, `during`
    // resolves them from another clip's span, and `rate:` (W4/t6) will make
    // the map non-affine by composing a curve over its output.
    var _swMap = function (parentProgress) {
      var total = typeof _swTotal === 'number' ? _swTotal : (ST.resolve(%&el, _swTotal) || 0);
      // A RELATIVE offset/span is a FRACTION of the score's total (BUG-270).
      // Under a runtime-total driver the total is the media duration, unknown
      // until the element's metadata loads, so the fraction cannot be a
      // constant — it is scaled HERE, at sample time, by the total this
      // sample resolved. Compile-time totals never set these flags (score.rs
      // folds the fraction into a constant instead), so the flags are the
      // discriminator between "scaled by the live total" and "already ms".
      var offset = _swOffsetRel ? _swOffset * total : _swOffset;
      var span = _swSpanRel ? _swSpan * total : _swSpan;
      var extent = span > 0 ? span : Math.max(0, total - offset);
      if (extent <= 0) return 0;
      var local = (parentProgress * total - offset) / extent;
      local = local < 0 ? 0 : (local > 1 ? 1 : local);

      // RATE IS THE DERIVATIVE of the window map (W4/R3). `local` is how far
      // through the window the PLAYHEAD is; rate says how fast the clip's own
      // progress advances at that point, so the clip's progress is the
      // INTEGRAL of the rate over the window, normalized.
      //
      //   d/dt = r0 + (r1 - r0) * t          (a linear ramp)
      //   integral 0..t = r0*t + (r1 - r0)*t^2/2
      //   integral 0..t IS the clip progress (deliberately NOT normalized)
      //
      // Every rate case falls out of that ONE integral rather than a branch
      // per case, which is the point of treating rate as a derivative:
      //   rate 1        identity      0  .25 .50 .75  1
      //   rate 1 -> 2   speed ramp    0  .28 .63  1    1   (finishes early)
      //   rate 2        double speed  0  .50  1    1    1
      //   rate 0.5      half speed    0  .13 .25 .38 .50   (never finishes)
      //   rate 0        freeze        0   0   0   0    0
      //   rate -1       reverse       0  .75 .50 .25   0
      //
      // NB the integral is NOT normalized by its own total. Normalizing would
      // make every constant rate identical to rate 1 — `rate 2` and `rate -1`
      // would both collapse into the identity map, silently. (They did; the
      // compile-output test could not see it, because the emitted CONSTANTS
      // differed while the resulting BEHAVIOR did not. Caught by evaluating
      // the map over sample points.)
      //
      // Not normalizing is also the honest semantics: a rate of 2 means the
      // clip advances twice as fast and therefore FINISHES EARLY, holding at 1
      // for the rest of its window. A rate of 0.5 never reaches the end. Both
      // are what an editor's speed control does.
      if (_swRateFrom !== 1 || _swRateTo !== 1) {
        var area = _swRateFrom * local
          + (_swRateTo - _swRateFrom) * local * local / 2;
        // A negative rate runs the clip BACKWARD. Its origin is selected once
        // from the initial sign below; clamping never turns reverse into freeze.
        // Origin belongs to the WINDOW, decided from the initial rate once.
        // A negative rate starts at the clip's end; its (negative) integral
        // then walks back toward zero. Never choose an origin per sample:
        // that was only accidentally correct for a total area of exactly -1.
        var origin = _swRateFrom < 0 ? 1 : 0;
        local = origin + area;
        local = local < 0 ? 0 : (local > 1 ? 1 : local);
      }
      return local;
    };

    // NB `%yield` takes a BARE IDENTIFIER, not an expression — `%yield f(x) ->`
    // is a JS parse error (E0800) that the expand stage records as a
    // diagnostic and then DROPS, so the primitive silently emits nothing.
    // Compute into a variable first; every working driver primitive does the
    // same (`progress = …; %yield progress -> $%name;`).

    // Publish once at the current parent value so a clip has a defined
    // progress BEFORE the score first advances. Without this a clip that has
    // not started yet reads `undefined` rather than 0, and anything mapping it
    // to a style would produce NaN — a silent visual break rather than a loud
    // one.
    var progress = _swMap(ST.resolve(%&el, %parent) || 0);
    %yield progress -> $%name;

    // SCOPED watch (W5b/BUG-266, FUP-094 doctrine): `parent` is the score's
    // own progress signal, which an enclosing score publishes on an ANCESTOR
    // (a nested `@score &.clip` inherits a window its PARENT assigned on a
    // node further up the chain). `ST.resolve` already walks up the parent
    // chain, so the initial read below worked while the reactive watch on our
    // OWN node never fired — the nested clip sat at its start value forever,
    // with a green build and a correct-looking bundle.
    //
    // `watchScoped` mirrors `resolve` — that mirroring IS the FUP-094 rule
    // (stdlib/enum/dispatch.st:192 states it canonically): a read path and
    // its watch path that disagree produce exactly this class of silent
    // staleness. Own-signal windows are unaffected; the walk starts at `el`.
    ST.watchScoped(%&el, %parent, function (parentProgress) {
      progress = _swMap(parentProgress || 0);
      %yield progress -> $%name;
    });
  }

  %exports {
    $progress: number
  }
}
