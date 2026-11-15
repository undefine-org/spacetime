/// Serve tests/fixtures/studio-authoring at port 4564. No film is present.
@import "stdlib/testing/test";
@import "stdlib/testing/automation";
@drive "http://127.0.0.1:4564/" {
  @wait_until (window.SpacetimeLocal && SpacetimeLocal.stuReflect && SpacetimeLocal.stuScore) timeout: 20s
  @eval (document.querySelector('.st-studio__pill').click(), document.querySelector('[data-tab="Components"]').click())
  @wait 150ms
  @eval (document.querySelector('.stu-comp__inspect').click())
  @wait 100ms
  @assert (document.querySelector('.stu-inspector').dataset.stState === 'Definition') "a normal page can inspect its shared definition"
  @assert (document.querySelector('.stu-definition-properties .w-swatch').value === '#70864c') "Studio uses the same typed property editor as the catalog"
  @eval (document.querySelector('.stu-comp__source').click())
  @wait 100ms
  @assert (document.querySelector('.stu-code').dataset.path === 'components.st') "source navigation works without a film"
  @assert (document.querySelector('.stu-film__body--code').getBoundingClientRect().height > 50) "the code workspace is visible on an ordinary page"
  @assert (!ST.stuFilm.transport()) "source editing does not invent a film transport"
}
