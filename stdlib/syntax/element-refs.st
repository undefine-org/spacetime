/// Element Refs Module Primitive (PLAN-133 single-consumer %uses wave)
///
/// Owns the named-element-ref registry (`window.__stRefs`: `&name` -> its
/// resolved DOM element). Lets a lowered `%derives` expression resolve
/// `$bounds` (an `&name` Element capture) to a real node via `ST.ref("name")`
/// (PLAN-057, the @drag bounds fix), and is a general seam any named ref can
/// read. Moved out of element-ref-impl — including its per-usage defensive
/// init (the drift shape: a second inline copy inside %emit js).
%primitive element-refs() {
  %emit prelude-js {
    window.__stRefs = window.__stRefs || {};
  }
}
