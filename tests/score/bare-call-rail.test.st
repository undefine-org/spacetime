/// BUG-271: a bare signal call `$ping();` under `@on $sig.change` fires on the
/// signal rail (change-driver + ST.runMutations' bare-call dispatch) — before
/// the fix the whole directive vanished from a green page.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"

@host $testApi : http("https://bare-call-rail-test.invalid")

@data signal $bcPing() to $testApi {
  send GET "/ping"
  receive to BcResult {
    200 => Ok($.body);
    _   => Failed($.statusText);
  }
}

@data inline $bcSrc number : 0;

.bc {
  @on $bcSrc.change {
    $bcPing()
  }
}

@test "a bare signal call under a change head FIRES when the signal changes" needs timing {
  @fixture { <div class="bc">x</div> }

  @data inline $bcFired number : 0;
  @eval (document.addEventListener('signal:bcPing:fire', () => { window.SpacetimeLocal.bcFired = (window.SpacetimeLocal.bcFired || 0) + 1; }))
  @assert (Number(window.SpacetimeLocal.bcFired || 0) === 0)
    "NEGATIVE: nothing has fired before the source changes"
  @eval (window.SpacetimeLocal.bcSrc = 1)
  @wait_for_signal $bcFired on .bc to_equal 1 timeout 2500ms
}
