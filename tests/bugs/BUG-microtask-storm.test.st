/// Microtask Storm Regression Tests
///
/// Reproduce the dataiku hang: many scroll timeline inits each call
/// queueMicrotask(() => update()), and each update() calls getBoundingClientRect()
/// forcing synchronous layout reflow. With 18+ scroll timelines, the microtask
/// queue becomes 18+ forced reflows before the browser can paint.
///
/// Fix: scroll timeline initial update() should use requestAnimationFrame (or
/// setTimeout(0)) instead of queueMicrotask, allowing the browser to batch layout.
///
/// Also: ST.set should batch dependent notifications into a single microtask
/// instead of queueing one microtask per dependent.

// =============================================================================
// Signal system correctness tests
// =============================================================================

@test "ST.set with many dependents completes quickly" {
  @fixture {
    <div class="set-storm-test">Set Storm</div>
  }

  @let (el = document.querySelector('.set-storm-test'))

  @eval (eval("var s = ST.store(el); s['stormSig'] = { v: 0, d: new Set() }; for (var i = 0; i < 50; i++) { s['stormSig'].d.add(function(v) {}); } window.__start = Date.now()"))
  @eval (eval("for (var i = 1; i <= 20; i++) { ST.set(el, 'stormSig', i); }"))
  @eval (eval("window.__elapsed = Date.now() - window.__start"))

  @assert (window.__elapsed < 500)
    "ST.set with 50 dependents x 20 calls should complete quickly"
}

@test "ST.set dependent that triggers another ST.set does not infinite loop" {
  @fixture {
    <div class="chain-test">Chain</div>
  }

  @let (el = document.querySelector('.chain-test'))

  @eval (eval("ST.set(el, 'sigA', 0); ST.set(el, 'sigB', 0); var sA = ST.store(el); sA['sigB'].d.add(function(v) { ST.set(el, 'sigA', v * 2); }); window.__chainStart = Date.now()"))
  @eval (eval("for (var i = 1; i <= 10; i++) { ST.set(el, 'sigB', i); }"))
  @eval (eval("window.__chainElapsed = Date.now() - window.__chainStart"))
  // ST.set batches dependent notifications into a microtask; drain it before
  // asserting on the dependent-driven $sigA value.
  @flush
  @flush

  @assert (window.__chainElapsed < 500)
    "Dependent chain should complete without infinite loop"
  @then $sigA on .chain-test should equal 20
  @then $sigB on .chain-test should equal 10
}

@test "large DOM subtree insertion completes quickly" {
  @fixture {
    <div class="flush-test-root"></div>
  }

  @eval (eval("window.__start = Date.now(); var root = document.querySelector('.flush-test-root'); var frag = document.createDocumentFragment(); for (var i = 0; i < 50; i++) { var d = document.createElement('div'); d.className = 'flush-child'; d.textContent = 'Item ' + i; frag.appendChild(d); } root.appendChild(frag); window.__elapsed = Date.now() - window.__start"))

  @assert (window.__elapsed < 500)
    "50-element subtree insertion plus init should complete quickly"
  @then .flush-child should have_count 50
}

@test "many signals updated simultaneously do not block" {
  @fixture {
    <div class="multi-sig-test">Multi</div>
  }

  @let (el = document.querySelector('.multi-sig-test'))

  @eval (eval("window.__multiStart = Date.now(); var s = ST.store(el); for (var i = 0; i < 30; i++) { var name = 'sig' + i; s[name] = { v: 0, d: new Set() }; for (var j = 0; j < 3; j++) { s[name].d.add(function(v) {}); } }"))
  @eval (eval("for (var i = 0; i < 30; i++) { ST.set(el, 'sig' + i, 1); }"))
  @eval (eval("window.__multiElapsed = Date.now() - window.__multiStart"))

  @assert (window.__multiElapsed < 500)
    "30 simultaneous signal updates should complete quickly"
}

@test "rapid signal updates simulating scroll timelines do not block" {
  @fixture {
    <div class="scroll-sim-test">Scroll Sim</div>
  }

  @let (el = document.querySelector('.scroll-sim-test'))

  @eval (eval("window.__simStart = Date.now(); var s = ST.store(el); for (var i = 0; i < 20; i++) { var name = 'progress' + i; s[name] = { v: 0, d: new Set() }; s[name].d.add(function(v) { el.style.setProperty('--p' + i, v); }); s[name].d.add(function(v) { el.style.opacity = v; }); }"))
  @eval (eval("for (var i = 0; i < 20; i++) { ST.set(el, 'progress' + i, 0); }"))
  @eval (eval("window.__simElapsed = Date.now() - window.__simStart"))

  @assert (window.__simElapsed < 500)
    "20 simulated scroll timeline inits should complete quickly"
}

// =============================================================================
// Batching correctness: ST.set dependents receive the final value
// =============================================================================

@test "ST.set rapid updates deliver final value to dependents" {
  @fixture {
    <div class="batch-test">Batch</div>
  }

  @let (el = document.querySelector('.batch-test'))

  // Track what values the dependent actually receives
  @eval (eval("window.__receivedValues = []; ST.set(el, 'batchSig', 0); var s = ST.store(el); s['batchSig'].d.add(function(v) { window.__receivedValues.push(v); })"))

  // Set the signal 5 times rapidly
  @eval (eval("ST.set(el, 'batchSig', 1); ST.set(el, 'batchSig', 2); ST.set(el, 'batchSig', 3); ST.set(el, 'batchSig', 4); ST.set(el, 'batchSig', 5)"))

  // The final stored value must be 5
  @then $batchSig on .batch-test should equal 5

  // With current per-set microtask dispatch, dependent sees [1,2,3,4,5].
  // With batched dispatch, dependent should see at least the final value (5).
  // Either way, the last received value must be 5.
  // Dependent notifications fire on a microtask; drain it before reading.
  @flush
  @flush
  @eval (eval("window.__lastReceived = window.__receivedValues[window.__receivedValues.length - 1]"))
  @assert (window.__lastReceived === 5)
    "Dependent must receive final value 5"
}
