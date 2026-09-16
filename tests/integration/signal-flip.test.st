/// FEAT-161 — subscribed assign writes move their view through a clock-owned
/// FLIP transition rather than snapping, including a write while already moving.
@import "stdlib/testing/test";
@import "stdlib/testing/timing";
@import "stdlib/macros/data-kind";
@import "stdlib/macros/host";
@import "stdlib";

@host $board : live("TestWeb.BoardLive") ;
@data subscribe $column from $board : column ;

.flip-card {
  @flip $column(duration: 300ms, easing: linear)
}

@test "subscribed write FLIPs and retargets from its current visual position" {
  @fixture { <div class="flip-card">card</div> }
  @clock install

  // LinkeDOM has no layout engine. This deterministic rect maps the subscribed
  // column to layout x and includes @flip's inline visual translation exactly
  // as a browser's getBoundingClientRect does.
  @eval (window.__flipEl = document.querySelector('.flip-card'))
  @eval (eval("window.__flipEl.getBoundingClientRect = function(){ var x = Number(window.SpacetimeLocal.column || 0) * 100; var m = (this.style.transform || '').match(/-?[0-9.]+/); return { left: x + (m ? Number(m[0]) : 0), top: 0 }; }"))
  @eval (eval("ST._selectorInitializers.filter(function(entry){ return entry.selector === '.flip-card' }).forEach(function(entry){ entry.init(window.__flipEl) })"))
  @eval (eval("document.addEventListener('signal:column:before-write', function(){ window.__flipWrites = (window.__flipWrites || 0) + 1 })"))

  // The real subscription setter is the only server-assign write path.
  @eval (window.__stLiveSubscriptions["TestWeb.BoardLive"].column[0](1))
  @assert (window.__flipWrites === 1 && window.SpacetimeLocal.column === 1)
    "the subscription emits before-write and applies its local signal write"
  @clock advance 1
  @assert (window.__flipEl.style.transform.includes('translate(-100px, 0px)'))
    "the post-write layout is inverted to the prior visual position"

  @clock advance 150
  @let (mid = window.__flipEl.style.transform)
  @assert (mid.includes('translate(') && !mid.includes('translate(-100px, 0px)') && !mid.includes('translate(0px, 0px)'))
    "a virtual-clock sample observes a non-snap intermediate position"

  // This second server write occurs while the first delta remains applied.
  // Its initial delta must be based on that rendered position, not the old
  // layout origin (-100px) and not the new layout destination (0px).
  @eval (window.__stLiveSubscriptions["TestWeb.BoardLive"].column[0](2))
  @clock advance 1
  @let (retarget = window.__flipEl.style.transform)
  @assert (retarget.includes('translate(') && !retarget.includes('translate(-100px, 0px)') && !retarget.includes('translate(0px, 0px)'))
    "a mid-animation assign write retargets from the current visual position"

  @clock run-to-end
  @assert (window.__flipEl.style.transform === '')
    "rollback or later server truth converges at the new layout position"
}
