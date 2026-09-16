/// Editable surface tests (PLAN-031 / FEAT-098).
///
/// These drive the LIVE surface end-to-end: a real `input` event flows through
/// beforeinput → AST op → re-projection, and the signal-method commands
/// ($doc.apply/$doc.setBlock) mutate the doc and re-project. They assert
/// logic-rung facts (have_text / element existence), so they run on --headless
/// (LinkeDOM dispatches the events + runs the op→reproject path) AND, at higher
/// fidelity, under the real browser:
///
///   cargo run --features headless -- test stdlib/editable/tests/surface.test.st --headless
///   cargo run --features cdp      -- test stdlib/editable/tests/surface.test.st --cdp
///
/// CDP is the higher-fidelity confirmation: real Selection/Range + IME composition
/// (LinkeDOM's selection is partial). The pure AST kernel (ops, projection,
/// position-bridge round-trip, 40-seed op-fuzz) is covered headless in Rust:
/// tests/v8_runtime/editable_model_tests.rs.

@import "stdlib/editable"

// A minimal editable surface bound to a doc signal, with the default vocabulary.
@data inline $body : null;

// --------------------------------------------------------------------------
// Typing routes through beforeinput → AST op → re-projection.
// --------------------------------------------------------------------------
@test "typing inserts text into the projected document" {
  @mount {
    <main><div class="ed"></div></main>
    .ed { @editable(bind: $body, marks: ["bold","italic","link"], blocks: ["p","h2"]); }
  }
  // Place the caret in the (empty) editable, then type.
  @eval (() => {
    const host = document.querySelector('.ed');
    host.focus();
    const r = document.createRange();
    const first = host.firstChild || host;
    r.setStart(first, 0); r.collapse(true);
    const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
  })
  @when .ed input "Hello"
  @then {
    @find ".ed p" { have_text "Hello"; }
  }
}

// --------------------------------------------------------------------------
// $doc.apply("bold") over a selection toggles the bold mark (projection shows <strong>).
// --------------------------------------------------------------------------
@test "apply bold over a selection wraps it in <strong>" {
  @mount {
    <main><div class="ed2"></div></main>
    .ed2 { @editable(bind: $body, marks: ["bold","italic","link"], blocks: ["p","h2"]); }
  }
  @eval (() => {
    const host = document.querySelector('.ed2');
    host.focus();
    // Seed content by typing, then select all of it.
    const r = document.createRange();
    r.setStart(host.firstChild || host, 0); r.collapse(true);
    const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
  })
  @when .ed2 input "bold me"
  @eval (() => {
    const host = document.querySelector('.ed2');
    const p = host.querySelector('p');
    const r = document.createRange();
    r.selectNodeContents(p);
    const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
    // Command rides the signal-method rail.
    ST.get(host, 'doc').apply('bold');
  })
  @then {
    @find ".ed2 strong" { have_text "bold me"; }
  }
}

// --------------------------------------------------------------------------
// $doc.setBlock("h2") changes the block type (projection shows <h2>).
// --------------------------------------------------------------------------
@test "setBlock h2 changes the block projection tag" {
  @mount {
    <main><div class="ed3"></div></main>
    .ed3 { @editable(bind: $body, marks: ["bold"], blocks: ["p","h2"]); }
  }
  @eval (() => {
    const host = document.querySelector('.ed3');
    host.focus();
    const r = document.createRange();
    r.setStart(host.firstChild || host, 0); r.collapse(true);
    const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
  })
  @when .ed3 input "Title"
  @eval (() => {
    const host = document.querySelector('.ed3');
    ST.get(host, 'doc').setBlock('h2');
  })
  @then {
    @find ".ed3 h2" { have_text "Title"; }
  }
}

// --------------------------------------------------------------------------
// Active-mark state is a signal: $activeMarks reflects the caret context.
// --------------------------------------------------------------------------
@test "activeMarks signal reflects bold over the selection" {
  @mount {
    <main><div class="ed4"></div></main>
    .ed4 { @editable(bind: $body, marks: ["bold","italic"], blocks: ["p"]); }
  }
  @eval (() => {
    const host = document.querySelector('.ed4');
    host.focus();
    const r = document.createRange();
    r.setStart(host.firstChild || host, 0); r.collapse(true);
    const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
  })
  @when .ed4 input "x"
  @eval (() => {
    const host = document.querySelector('.ed4');
    const p = host.querySelector('p');
    const r = document.createRange(); r.selectNodeContents(p);
    const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
    ST.get(host, 'doc').apply('bold');
    window.__am = ST.get(host, 'activeMarks');
  })
  @then {
    @assert (window.__am && window.__am.indexOf('bold') !== -1)
      "activeMarks should contain 'bold' after applying it over the selection";
  }
}

// --------------------------------------------------------------------------
// REGION editing (FUP-045): typing into a region host lands in THAT region.
// A columns block has two parallel inline regions; a caret in the right column's
// host routes through the PATH-aware bridge (S1) + region-aware ops (S2), so the
// text grows the right region's run — not a flattened one. The live-funnel proof
// of the region surface on the headless logic rung (CDP confirms real Selection).
// (Unblocked by the BUG-095 @eval arrow-fn parse fix.)
// --------------------------------------------------------------------------
@test "typing into a column region lands in that region" {
  @mount {
    <main><div class="edr"></div></main>
    .edr { @editable(bind: $body, marks: ["bold"], blocks: ["columns","p"]); }
  }
  // Seed a columns doc into the surface, place the caret at the end of the RIGHT
  // region's text, type, then read the surface's own doc back.
  @eval (() => {
    const host = document.querySelector('.edr');
    const doc = { type:'doc', content:[
      { type:'columns', regions:{
        left:  { kind:'inline', content:[ ST.Editable.text('L') ] },
        right: { kind:'inline', content:[ ST.Editable.text('R') ] } } }
    ]};
    ST.set(host, 'doc', doc);
    host.innerHTML = '';
    host.appendChild(ST.Editable.project(doc, document, ['bold']));
    const rightHost = host.querySelector('[data-st-region=right]');
    let t = rightHost; while (t.firstChild) t = t.firstChild;
    const r = document.createRange();
    r.setStart(t, t.textContent ? t.textContent.length : 0); r.collapse(true);
    const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
  })
  @when .edr input "Z"
  @eval (() => {
    const host = document.querySelector('.edr');
    const d = ST.get(host, 'doc');
    const rc = d.content[0].regions.right.content;
    const lc = d.content[0].regions.left.content;
    window.__rt = rc.length ? rc[0].text : '?';
    window.__lt = lc.length ? lc[0].text : '?';
  })
  @then {
    @assert (window.__rt === 'RZ') "typing in the right column grows the right region";
    @assert (window.__lt === 'L') "the left column is untouched";
  }
}

// --------------------------------------------------------------------------
// @richtext (editable-everything): a PUBLISHED surface is EDIT-MODE-GATED and
// seeds its initial doc from a `$`-PATH bind resolved against local state.
//   - read-only by default (no contenteditable until html[data-st-edit-mode]);
//   - reads the doc AST from $page.body (a member path), not an inlined object;
//   - toggling edit-mode upgrades it in place; typing routes through the funnel.
// --------------------------------------------------------------------------
@data inline $page : { "body": { "type": "doc", "content": [ { "type": "p", "content": [ { "type": "text", "text": "Hello" } ] } ] } };

@test "@richtext is edit-mode-gated and seeds from a data path" {
  @mount {
    <main><div class="rt"></div></main>
    .rt { @richtext(bind: $page.body, marks: ["bold","italic"], blocks: ["p","h2"]); }
  }
  // The surface seeded its doc from $page.body (a `$`-path, resolved against local
  // state) and is READ-ONLY until edit-mode is on.
  @eval (() => {
    const host = document.querySelector('.rt');
    window.__ceBefore = host.getAttribute('contenteditable');   // null = read-only
    window.__seeded = (host.querySelector('p') || {}).textContent || '';
    // Turn on edit-mode; the gate observer flips contenteditable.
    document.documentElement.setAttribute('data-st-edit-mode', 'true');
  })
  @then {
    @find ".rt p" { have_text "Hello"; }
    @assert (window.__ceBefore == null) "published @richtext is read-only until edit-mode";
    @assert (window.__seeded === 'Hello') "@richtext seeds its doc from the $page.body data path";
  }
}
