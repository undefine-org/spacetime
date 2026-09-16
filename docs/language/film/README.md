# Film surface — working docs (per wave)

The **golden reference** is [`../film.st.md`](../film.st.md): it describes the
film surface in its FINISHED state, and by design it does **not** build today —
every section whose feature has not landed fails loud (that is the point; see
the doc gate).

This directory holds one **working doc per wave** of `PLAN-150`. Each `wN-*.st.md`
uses **only syntax that has landed up to and including that wave**, so it BUILDS
GREEN and doubles as:

- the wave's **acceptance target** (`cargo test --test film_doc_gate` builds it), and
- a **living example** a reader can run (`cargo run -- serve` / `render`).

As a wave lands, its working doc appears here and the matching `— status —`
block in the golden flips to "shipped". When the last wave lands, every working
doc's syntax is a subset of the golden, and the golden itself builds green — at
which point these per-wave files are folded back and deleted.

| wave | working doc | lands |
|---|---|---|
| W0 | `w0-doc-gate.st.md` | the gate + honest diagnostics for unlanded words (this wave) |
| W1 | `w1-positioned-stops.st.md` | `at` stops, per-step easing, `--steps` |
| W2 | `w2-post.st.md` | `@post` on any element, keyframeable |
| … | … | see `!tasks/plans/PLAN-150` |

## The doc gate

`tests/film_doc_gate.rs` is the enforcement:

1. **Landed builds green.** Every `wN-*.st.md` in this directory compiles.
2. **Unlanded fails loud.** A canary snippet for each not-yet-landed feature is
   asserted to FAIL with a specific diagnostic code — so no film word can sit in
   the golden and silently compile to nothing. When a wave lands, its canary
   moves from the "must fail" list to a green fence in the wave's working doc.
