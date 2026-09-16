/// FUP-045 S4 — live region-editing under a real browser (CDP/--cdp).
///
/// The headless `.st` surface test for region typing is blocked by BUG-095 (the
/// @eval lexer mis-parses an array-of-object literal). This `@drive` test runs the
/// SAME region-edit loop against a real Chromium, where the browser Selection is
/// real (LinkeDOM's is partial) — the highest-fidelity rung for the region surface.
/// It builds the seed doc with push() (no inline `[ { … } ]` literal) to sidestep
/// BUG-095, and asserts the full loop: a real caret in the right region → S1 bridge
/// → S2 region op → reproject → caret restored in the right region.
///
/// RUN:  cargo run --features cdp -- test stdlib/editable/tests/region-surface.cdp.test.st --cdp
/// (the demo at projects/region-edit-demo/ must be served, or @drive a built page).
///
/// NB run-pending in this environment: the demo is served on a dev port; point the
/// runner at it (or a deployed build). The same loop is PROVEN interactively via the
/// browser tool against the live page (screenshot scratch/region-edit-live.png).

@import "stdlib/testing/test"
@import "stdlib/testing/automation"

@drive "http://127.0.0.1:4444/" {
  // Wait for the surface to mount, then build a columns doc WITHOUT an inline
  // array-of-object literal (BUG-095): push the block, then attach regions.
  @eval (() => {
    const host = document.querySelector('.editor');
    // Build the schema region list with push() (no inline array-of-object — BUG-095).
    const regs = [];
    const rl = {}; rl.name = 'left';  rl.kind = 'inline'; rl.tag = 'div'; regs.push(rl);
    const rr = {}; rr.name = 'right'; rr.kind = 'inline'; rr.tag = 'div'; regs.push(rr);
    const colSpec = {}; colSpec.tag = 'div'; colSpec.regions = regs;
    const blocks = {}; blocks.p = { tag: 'p' }; blocks.columns = colSpec;
    ST.Editable.setSchema({ marks: { bold: { tag: 'strong' } }, blocks: blocks });
    const doc = { type: 'doc' };
    doc.content = [];
    const blk = { type: 'columns' };
    const left = {}; left.kind = 'inline'; left.content = []; left.content.push(ST.Editable.text('L'));
    const right = {}; right.kind = 'inline'; right.content = []; right.content.push(ST.Editable.text('R'));
    blk.regions = {}; blk.regions.left = left; blk.regions.right = right;
    doc.content.push(blk);
    host.innerHTML = '';
    host.appendChild(ST.Editable.project(doc, document, ['bold']));
    window.__doc = doc;
  })

  // Real Selection: caret at the end of the RIGHT region's text, then run the
  // surface loop (posFromDOM → insertText → project → domFromPos) on the real DOM.
  @eval (() => {
    const host = document.querySelector('.editor');
    const right = host.querySelector('[data-st-region=right]');
    let t = right; while (t.firstChild) t = t.firstChild;
    const range = document.createRange();
    range.setStart(t, t.textContent.length); range.collapse(true);
    const sel = window.getSelection(); sel.removeAllRanges(); sel.addRange(range);
    const r = sel.getRangeAt(0);
    let doc = window.__doc;
    const from = ST.Editable.posFromDOM(host, doc, r.startContainer, r.startOffset);
    doc = ST.Editable.insertText(doc, from, 'Z');
    host.innerHTML = '';
    host.appendChild(ST.Editable.project(doc, document, ['bold']));
    const dp = ST.Editable.domFromPos(host, doc, from + 1);
    window.__doc = doc;
    let n = dp ? dp.node : null, inRight = false;
    while (n && n !== host) { if (n.getAttribute && n.getAttribute('data-st-region') === 'right') { inRight = true; break; } n = n.parentNode; }
    window.__caretInRight = inRight;
  })

  // The char landed in the RIGHT region; the left is untouched; the caret restored
  // inside the right region — the live region-edit loop under a real browser.
  @assert (window.__doc.content[0].regions.right.content[0].text === 'RZ')
    "typing in the right region grows the right region (live Selection)";
  @assert (window.__doc.content[0].regions.left.content[0].text === 'L')
    "the left region is untouched";
  @assert (window.__caretInRight === true)
    "the caret is restored inside the right region after the edit";

  @capture screenshot "scratch/region-edit-cdp.png";
}
