// GH-36: optional trailing literal in %form.
// @wait_until $condition:expr timeout: $timeout:time = 5000
// The `timeout:` literal looks optional (default 5000) but is required; the
// documented bare spelling `@wait_until cond` was claimed to match no form and
// be silently dropped.
@import "stdlib/testing/test"

// (A) FULL form: condition + `timeout:` literal with explicit value.
@test "full form with timeout binds" {
  @eval (window.__g36a = 'no')
  @eval (eval("setTimeout(function() { window.__g36a = 'yes'; }, 20)"))
  @wait_until __g36a === 'yes' timeout: 500ms
  @assert (window.__g36a === 'yes')
}

// (B) BARE form: condition only, no `timeout:` literal (relies on default 5000).
@test "bare form (no timeout literal)" {
  @eval (window.__g36b = 'no')
  @eval (eval("setTimeout(function() { window.__g36b = 'yes'; }, 20)"))
  @wait_until __g36b === 'yes'
  @assert (window.__g36b === 'yes')
}
