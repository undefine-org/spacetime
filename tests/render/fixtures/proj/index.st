// tests/render/fixtures/proj/index.st — the DIRECTORY-form render gate (PLAN-132
// W5 FUP). `spacetime render proj/` documents "Project directory or page": a
// directory must resolve to its root page. The documented convention (mirroring
// how `test`/`build` find a project's entry) is `index.st` first, then
// `index.st.md`. This file is that root page. The gate renders the PARENT
// DIRECTORY and asserts it lands here (1s score @ 10fps -> exactly 10 frames).

@version 2026-06-09;

@form motion --clip-rise { opacity: 0 -> 1; }

@form score --story {
  &head at 0s for 0.5s;
  &tail at 0.5s for 0.5s;
}

<div class="film">
  <div class="head">head</div>
  <div class="tail">tail</div>
</div>

.film {
  @score &.time(1s) {
    --story;
  }
}

.head { opacity: 0; @on &.clip: --clip-rise; }
.tail { opacity: 0; @on &.clip: --clip-rise; }
