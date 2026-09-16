/// Unit Tests: Animation Progress Signals
///
/// Named progress signals live on their owning element. Undefined means absent.

@import "stdlib/testing/test"

@test "ST.resolve returns undefined for an absent progress signal" {
  @fixture { <div class="absent-progress">Absent</div> }

  @let (el = document.querySelector('.absent-progress'))
  @assert (ST.resolve(el, 'intro') === undefined)
    "Absent progress signal should resolve to undefined"
}

@test "ST.set publishes progress resolved from its owner" {
  @fixture { <div class="published-progress">Published</div> }

  @let (el = document.querySelector('.published-progress'))
  @eval (ST.set(el, 'intro', 0.5))

  @assert (ST.resolve(el, 'intro') === 0.5)
    "Published progress should resolve from the same element"
}

@test "progress at or above one means complete" {
  @fixture { <div class="complete-progress">Complete</div> }

  @let (el = document.querySelector('.complete-progress'))
  @eval (ST.set(el, 'intro', 1))

  @assert (ST.resolve(el, 'intro') >= 1)
    "Progress at one should be complete"
}

@test "progress between zero and one means running" {
  @fixture { <div class="running-progress">Running</div> }

  @let (el = document.querySelector('.running-progress'))
  @eval (ST.set(el, 'intro', 0.5))

  @assert (ST.resolve(el, 'intro') > 0 && ST.resolve(el, 'intro') < 1)
    "Intermediate progress should be running"
}

@test "ST.watch receives direct progress publications" {
  @fixture { <div class="watched-progress">Watched</div> }

  @let (el = document.querySelector('.watched-progress'))
  @eval (eval("window.__receivedProgress = undefined; ST.watch(el, 'intro', function(progress) { window.__receivedProgress = progress })"))
  @eval (ST.set(el, 'intro', 0.75))
  @flush

  @assert (window.__receivedProgress === 0.75)
    "Watch callback should receive the published progress"
}
