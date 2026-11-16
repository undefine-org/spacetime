/// Serve demos/film-grokbot at port 4561. No source writes.
@import "stdlib/testing/test";
@import "stdlib/testing/automation";
@drive "http://127.0.0.1:4561/" {
  @wait_until (window.SpacetimeLocal && SpacetimeLocal.stuScore && ST.stuFilm.transport()) timeout: 20s
  @eval (document.documentElement.classList.contains('stu-on') || document.querySelector('.st-studio__pill').click())
  @eval (ST.stuFilm.transport().pause(), ST.stuFilm.seek(10500))
  @wait 300ms
  @assert (document.querySelector('.film__canvas').getBoundingClientRect().right <= document.querySelector('.stu-inspector').getBoundingClientRect().left + 1) "the whole authored film fits before the inspector"
  @eval (document.querySelector('[data-tab="Components"]').click())
  @wait 100ms
  @assert (document.querySelectorAll('.stu-tree--components .au-definition').length === SpacetimeLocal.stuComponents.length && Array.from(document.querySelectorAll('.stu-tree--components .au-definition__name')).every(x => x.textContent.trim())) "each reflected definition renders exactly one named card without empty shells"
  @assert (Array.from(document.querySelectorAll('.au-definition')).every(x => x.closest('.stu-comp'))) "child definition cards do not leak into the page root"
  @eval (Array.from(document.querySelectorAll('.stu-comp')).find(x => x.textContent.includes('--bubble')).querySelector('.stu-comp__inspect').click())
  @wait 100ms
  @assert (document.querySelectorAll('.stu-inspector .au-origin').length === document.querySelectorAll('.stu-inspector .au-property').length && Array.from(document.querySelectorAll('.stu-inspector .au-origin__value')).every(x => x.textContent.trim())) "each property renders exactly one populated origin trail"
  @assert (Array.from(document.querySelectorAll('.au-origin')).every(x => x.closest('.au-property'))) "child origin trails do not leak into the page root"
  @eval (Array.from(document.querySelectorAll('.stu-comp')).find(x => x.textContent.includes('--bubble')).querySelector('.stu-comp__source').click())
  @wait 100ms
  @assert (document.querySelector('.stu-code').dataset.path === 'design/components.st') "a definition opens its owning module"
  @assert (document.querySelectorAll('.stu-source-file').length >= 14) "all imported source files can be opened"
  @eval (document.querySelector('[data-panel="Timeline"]').click(), document.querySelector('[data-tab="Layers"]').click())
  @eval (Array.from(document.querySelectorAll('.stu-node')).find(x => x.dataset.file === 'index.st' && x.textContent.trim() === '--voice-invitation').click())
  @wait 100ms
  @assert (!SpacetimeLocal.stuError && ST.stu.el === document.querySelector('.voice-invitation')) "a form invocation reveals its actual expanded scene"
  @assert (SpacetimeLocal.stuFilmRows.some(x => x.kind === 'audio' && x.label.includes('&soundtrack'))) "recorded audio is an editable timeline entity"
}
