/// FEAT-159 — LiveView assign subscriptions share the existing local signal plane.
@import "stdlib/testing/test"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/host"

@host $counter : live("TestWeb.CounterLive") ;
@data subscribe $serverCount from $counter : count ;

@test "subscribe mirrors a local assign diff once after reseed" {
  @fixture { <div class="subscribe-target">x</div> }

  // This is the hook's module+assign dispatch path: only the registered setter
  // may write the subscribed local target.
  @eval (window.__stLiveSubscriptions["TestWeb.CounterLive"].count[0](7))
  @eval (window.__feat159_after_first = window.SpacetimeLocal.serverCount)
  @assert (window.__feat159_after_first === 7)
    "a declared assign diff reaches the subscribed local target"

  // Re-applying a diff uses the same registered setter; no listener is added.
  @eval (window.__stLiveSubscriptions["TestWeb.CounterLive"].count[0](9))
  @eval (window.__feat159_after_reseed = window.SpacetimeLocal.serverCount)
  @assert (window.__feat159_after_reseed === 9)

  @eval (window.__feat159_handlers = window.__stLiveSubscriptions["TestWeb.CounterLive"].count.length)
  @assert (window.__feat159_handlers === 1)
    "reseeded subscription remains a single reactive target"
}
