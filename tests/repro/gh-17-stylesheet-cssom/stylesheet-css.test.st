@import "stdlib/testing/test"

body {
  --brand-color: #ff0000;
}

@test "stylesheet custom property is visible via getComputedStyle" {
  @fixture {
    <div class="probe17"></div>
  }
  @let (val = getComputedStyle(document.body).getPropertyValue('--brand-color'))
  @assert (val.trim() === '#ff0000') "expected stylesheet custom property to be readable"
}

@test "getComputedStyle refuses stylesheet-derived reads by rung name + --cdp" {
  @fixture {
    <div class="probe17b"></div>
  }
  // A stylesheet-derived read is NOT in the logic backend's `el.style`, so the
  // honest value is unknowable headless. The stub must REFUSE (I5 / PLAN-137 W5)
  // — a tagged error naming the 'layout' rung and the `--cdp` escape hatch — not
  // return a plausible undefined and crash the caller with a bare TypeError.
  @eval (eval("try { getComputedStyle(document.body).getPropertyValue('--brand-color'); window.__r17 = 'NO-REFUSAL'; } catch (e) { window.__r17 = e.message; }"))
  @assert (window.__r17 !== 'NO-REFUSAL')
    "a stylesheet-derived getComputedStyle read must refuse, not return a fake value"
  @assert (window.__r17.indexOf('layout') !== -1)
    "the refusal must name the 'layout' rung the read requires"
  @assert (window.__r17.indexOf('--cdp') !== -1)
    "the refusal must name the `--cdp` escape hatch (defer to a browser backend)"
}

