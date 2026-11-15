/// Serve demos/film-robot-day at port 4562. No source writes.
@import "stdlib/testing/test";
@import "stdlib/testing/automation";
@drive "http://127.0.0.1:4562/" {
  @wait_until (window.SpacetimeLocal && SpacetimeLocal.stuReflect && ST.stuFilm.transport() && ST.stuFilm.stage()) timeout: 20s
  @eval (document.querySelector('.st-studio__pill').click(), ST.stuFilm.transport().pause(), ST.stuFilm.seek(8000))
  @eval (ST.stuFilm.select('part:' + SpacetimeLocal.stuScore.parts.findIndex(p => p.name === 'robot')))
  @wait 250ms
  @assert (document.querySelector('.stu-part-fields .w-field[data-label="clip"] .w-choice[data-on="true"]').dataset.word === 'Walking') "a driven field displays the actual evaluated clip at the playhead"
  @assert (document.querySelector('.stu-part-fields .au-property[data-property="clip"] .au-property__authored').textContent.includes('Idle')) "the authored base remains separately visible"
  @eval (document.querySelector('.stu-part-fields .au-property[data-property="clip"] [data-scope="base"]').click())
  @wait 100ms
  @assert (document.querySelector('.stu-part-fields .w-field[data-label="clip"]').dataset.drive === '') "base scope targets the declaration rather than an animation stop"
  @assert (document.querySelector('.stu-part-fields .w-field[data-label="clip"] .w-choice[data-on="true"]').dataset.word === 'Idle') "base scope edits the authored value"
  @eval (ST.stuFilm.seek(12500))
  @wait 100ms
  @assert (document.querySelector('.stu-part-fields .au-property[data-property="clip"] .au-property__evaluated').textContent.includes('Wave')) "the evaluated readout follows a seek while base editing stays explicit"
  @eval (document.querySelector('.stu-part-fields .au-property[data-property="clip"] [data-scope="keyframe"]').click())
  @wait 100ms
  @assert (document.querySelector('.stu-part-fields .w-field[data-label="clip"]').dataset.drive !== '') "keyframe scope targets the current score channel"
  @assert (document.querySelector('.stu-part-fields .w-field[data-label="clip"] .w-choice[data-on="true"]').dataset.word === 'Wave') "the control returns to the actual playhead value"
}
