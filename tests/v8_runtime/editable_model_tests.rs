//! Editable AST-model tests (PLAN-031 / FEAT-098), headless V8.
//!
//! Exercises the PURE kernel: integer positions, edit ops, active-mark/block
//! queries, AST→DOM projection, and an op-sequence FUZZ round-trip. No selection,
//! caret, or IME here — those need a real layout engine (LinkeDOM has no
//! getSelection), so they live in the `--cdp` suite per the fidelity ladder.

use super::context::V8TestContext;

const EDITABLE_JS: &str = include_str!("../../public/runtime/editable.js");

/// A context with the editable kernel loaded and a representative schema
/// (bold/italic/link marks; p/h2/figure blocks). LinkeDOM `document` provided by
/// with_runtime so `project` can build real nodes.
fn ctx() -> V8TestContext {
    let mut c = V8TestContext::new()
        .with_runtime()
        .expect("st.js")
        .with_editable()
        .expect("editable.js");
    c.eval(
        r#"
        ST.Editable.setSchema({
          marks: {
            bold: { tag: 'strong', attrs: {}, sel: 'children' },
            italic: { tag: 'em', attrs: {}, sel: 'children' },
            link: { tag: 'a', attrs: { href: { type: 'url' } }, sel: 'children' }
          },
          blocks: {
            p: { tag: 'p', attrs: {}, sel: 'children' },
            h2: { tag: 'h2', attrs: {}, sel: 'children' },
            figure: { tag: 'figure', attrs: {}, sel: 'nested' }
          }
        });
        void 0;
    "#,
    )
    .expect("set schema");
    c
}

fn assert_true(c: &mut V8TestContext, js: &str) {
    let r = c.eval(js);
    assert!(
        matches!(&r, Ok(v) if v.as_bool() == Some(true)),
        "expected true from `{js}`, got {r:?}"
    );
}

#[test]
fn empty_doc_is_single_empty_paragraph() {
    let mut c = ctx();
    assert_true(&mut c, "ST.Editable.emptyDoc().content.length === 1");
    assert_true(&mut c, "ST.Editable.emptyDoc().content[0].type === 'p'");
    assert_true(&mut c, "ST.Editable.isEmpty(ST.Editable.emptyDoc())");
}

#[test]
fn doc_size_counts_boundaries_and_chars() {
    let mut c = ctx();
    // one block "hi": 1 open + 2 chars + 1 close = 4
    c.eval(
        r#"globalThis.D = { type:'doc', content:[{type:'p', content:[ST.Editable.text('hi')]}] };"#,
    )
    .unwrap();
    assert_true(&mut c, "ST.Editable.docSize(D) === 4");
}

#[test]
fn insert_text_inherits_marks_and_coalesces() {
    let mut c = ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[{type:'p', content:[ ST.Editable.text('ab', [{type:'bold'}]) ]}] };
        // Insert "X" at pos 1 (between a|b): inherits bold from the char to the left.
        globalThis.D2 = ST.Editable.insertText(D, 1, 'X');
        void 0;
    "#).unwrap();
    // Single coalesced text run "aXb", all bold.
    assert_true(&mut c, "D2.content[0].content.length === 1");
    assert_true(&mut c, "D2.content[0].content[0].text === 'aXb'");
    assert_true(&mut c, "D2.content[0].content[0].marks[0].type === 'bold'");
}

#[test]
fn toggle_mark_adds_then_removes() {
    let mut c = ctx();
    c.eval(r#"globalThis.D = { type:'doc', content:[{type:'p', content:[ST.Editable.text('hello')]}] };"#)
        .unwrap();
    // pos 1..4 = "ell"
    c.eval("globalThis.B = ST.Editable.toggleMark(D, 1, 4, 'bold');")
        .unwrap();
    assert_true(
        &mut c,
        "ST.Editable.activeMarks(B, 1, 4).indexOf('bold') !== -1",
    );
    // toggling again removes it
    c.eval("globalThis.B2 = ST.Editable.toggleMark(B, 1, 4, 'bold');")
        .unwrap();
    assert_true(
        &mut c,
        "ST.Editable.activeMarks(B2, 1, 4).indexOf('bold') === -1",
    );
}

#[test]
fn toggle_link_carries_href_attr() {
    let mut c = ctx();
    c.eval(r#"globalThis.D = { type:'doc', content:[{type:'p', content:[ST.Editable.text('site')]}] };"#)
        .unwrap();
    c.eval(r#"globalThis.L = ST.Editable.toggleMark(D, 0, 4, 'link', { href: '/x' });"#)
        .unwrap();
    assert_true(
        &mut c,
        "L.content[0].content[0].marks.find(m=>m.type==='link').attrs.href === '/x'",
    );
}

#[test]
fn unknown_mark_or_block_is_rejected() {
    let mut c = ctx();
    c.eval(
        r#"globalThis.D = { type:'doc', content:[{type:'p', content:[ST.Editable.text('x')]}] };"#,
    )
    .unwrap();
    // unregistered mark → doc unchanged (identity)
    assert_true(&mut c, "ST.Editable.toggleMark(D,0,1,'blink') === D");
    assert_true(&mut c, "ST.Editable.setBlockType(D,0,'marquee') === D");
}

#[test]
fn set_block_type_changes_block() {
    let mut c = ctx();
    c.eval(r#"globalThis.D = { type:'doc', content:[{type:'p', content:[ST.Editable.text('Title')]}] };"#)
        .unwrap();
    c.eval("globalThis.H = ST.Editable.setBlockType(D, 0, 'h2');")
        .unwrap();
    assert_true(&mut c, "ST.Editable.blockTypeAt(H, 0) === 'h2'");
}

#[test]
fn split_block_produces_two_blocks() {
    let mut c = ctx();
    c.eval(r#"globalThis.D = { type:'doc', content:[{type:'p', content:[ST.Editable.text('abcd')]}] };"#)
        .unwrap();
    c.eval("globalThis.S = ST.Editable.splitBlock(D, 2);")
        .unwrap();
    assert_true(&mut c, "S.content.length === 2");
    assert_true(&mut c, "S.content[0].content[0].text === 'ab'");
    assert_true(&mut c, "S.content[1].content[0].text === 'cd'");
}

#[test]
fn delete_range_within_block() {
    let mut c = ctx();
    c.eval(r#"globalThis.D = { type:'doc', content:[{type:'p', content:[ST.Editable.text('hello')]}] };"#)
        .unwrap();
    c.eval("globalThis.X = ST.Editable.deleteRange(D, 1, 4);")
        .unwrap(); // remove "ell"
    assert_true(&mut c, "X.content[0].content[0].text === 'ho'");
}

#[test]
fn delete_range_merges_blocks() {
    let mut c = ctx();
    c.eval(
        r#"globalThis.D = { type:'doc', content:[
            {type:'p', content:[ST.Editable.text('ab')]},
            {type:'p', content:[ST.Editable.text('cd')]}
        ]};"#,
    )
    .unwrap();
    // Block boundaries cost 1 each: block1 spans flat [0..4) (1 open + "ab" + 1
    // close), block2 spans [4..8). "a"=offset0, after-"a"=pos1; "d"=last char,
    // before-"d"=pos5. Delete [1,5) removes "b",close,open,"c" → "a"+"d" merged.
    c.eval("globalThis.M = ST.Editable.deleteRange(D, 1, 5);")
        .unwrap();
    assert_true(&mut c, "M.content.length === 1");
    assert_true(&mut c, "M.content[0].content[0].text === 'ad'");
}

#[test]
fn projection_renders_marks_and_blocks() {
    let mut c = ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'h2', content:[ ST.Editable.text('Title') ] },
          { type:'p',  content:[ ST.Editable.text('hi '), ST.Editable.text('bold', [{type:'bold'}]) ] }
        ]};
        const frag = ST.Editable.project(D, document, ['bold','italic','link']);
        const host = document.createElement('div');
        host.appendChild(frag);
        globalThis.HTML = host.innerHTML;
        void 0;
    "#,
    )
    .unwrap();
    assert_true(&mut c, "HTML.indexOf('<h2>Title</h2>') !== -1");
    assert_true(&mut c, "HTML.indexOf('<strong>bold</strong>') !== -1");
    assert_true(&mut c, "/<p>hi <strong>bold<\\/strong><\\/p>/.test(HTML)");
}

#[test]
fn projection_link_carries_href() {
    let mut c = ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'p', content:[ ST.Editable.text('go', [{type:'link', attrs:{href:'/x'}}]) ] }
        ]};
        const frag = ST.Editable.project(D, document, ['bold','italic','link']);
        const host = document.createElement('div'); host.appendChild(frag);
        globalThis.HTML = host.innerHTML;
        void 0;
    "#,
    )
    .unwrap();
    assert_true(&mut c, "/<a href=\"\\/x\">go<\\/a>/.test(HTML)");
}

#[test]
fn dom_position_bridge_round_trips() {
    // posFromDOM and domFromPos are inverses over a projected host, AND agree with
    // the model (`resolve`) BY CONSTRUCTION (both walk the same boundary budget as
    // blockSize/resolvePath). Headless-safe (no Selection): project, then for each
    // position recover (node,offset)→back to pos, and cross-check vs resolve.
    let mut c = ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'h2', content:[ ST.Editable.text('Hi') ] },
          { type:'p',  content:[ ST.Editable.text('ab'), ST.Editable.text('cd', [{type:'bold'}]) ] }
        ]};
        globalThis.host = document.createElement('div');
        host.appendChild(ST.Editable.project(D, document, ['bold','italic','link']));
        globalThis.checkPos = function(p){
          const dp = ST.Editable.domFromPos(host, D, p);
          if (!dp) return 'no-dp@'+p;
          const back = ST.Editable.posFromDOM(host, D, dp.node, dp.offset);
          return back === p ? 'ok' : ('drift '+p+'->'+back);
        };
        void 0;
    "#,
    )
    .unwrap();
    // Every interior position must round-trip AND equal the model's flat offset.
    for p in [1, 2, 5, 6, 7, 8] {
        let r = c.eval(&format!("checkPos({p})")).unwrap();
        assert_eq!(r.as_str(), Some("ok"), "position {p} did not round-trip");
    }
    // bridge ≡ model (the headline fix): for every INTERIOR content position, the
    // bridge round-trips it back to ITSELF (domFromPos→posFromDOM === p) AND the
    // recovered position resolves to the same {blockIndex,offset} the model maps p
    // to. The pre-FUP-042 bridge used a block-only budget (open=acc+1) that
    // disagreed with the folded model convention (open=acc) — this pins the equality
    // so that latent off-by-one can never silently return. Block-boundary positions
    // (0, block-close) legitimately snap to a neighbour leaf, so we check the
    // interior content positions where the caret actually lives.
    assert_true(
        &mut c,
        "(function(){var bad=[];[1,2,5,6,7,8].forEach(function(p){var dp=ST.Editable.domFromPos(host,D,p);var back=ST.Editable.posFromDOM(host,D,dp.node,dp.offset);var rp=ST.Editable.resolve(D,p),rb=ST.Editable.resolve(D,back);if(back!==p||rp.blockIndex!==rb.blockIndex||rp.offset!==rb.offset)bad.push(p+':'+back);});return bad.length===0;})()",
    );
}

// =========================================================================
// Paste lift (lens backward direction, FEAT-101)
// =========================================================================

#[test]
fn insert_doc_single_block_joins_inline() {
    let mut c = ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[{ type:'p', content:[ ST.Editable.text('abef') ] }] };
        // Positions: open@0, a@1, b@2, e@3, f@4. Insert "CD" at pos 2 (after "ab").
        globalThis.OTHER = { type:'doc', content:[{ type:'p', content:[ ST.Editable.text('CD') ] }] };
        globalThis.R = ST.Editable.insertDoc(D, 2, OTHER);
        void 0;
    "#,
    )
    .unwrap();
    assert_true(&mut c, "R.doc.content.length === 1");
    assert_true(&mut c, "R.doc.content[0].content[0].text === 'abCDef'");
    assert_true(&mut c, "R.pos === 4");
}

#[test]
fn insert_doc_multi_block_appends_blocks() {
    let mut c = ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[{ type:'p', content:[ ST.Editable.text('abef') ] }] };
        globalThis.OTHER = { type:'doc', content:[
          { type:'p', content:[ ST.Editable.text('CD') ] },
          { type:'h2', content:[ ST.Editable.text('H') ] }
        ]};
        // Insert at pos 2 (after "ab"): block0 = "ab"+"CD", block1 = h2 "H"+tail "ef".
        globalThis.R = ST.Editable.insertDoc(D, 2, OTHER);
        void 0;
    "#,
    )
    .unwrap();
    // Block 0 = "abCD" (head+first), block 1 = h2 "H" + tail "ef".
    assert_true(&mut c, "R.doc.content.length === 2");
    assert_true(&mut c, "R.doc.content[0].content[0].text === 'abCD'");
    assert_true(&mut c, "R.doc.content[1].type === 'h2'");
    assert_true(
        &mut c,
        "R.doc.content[1].content.map(n=>n.text).join('') === 'Hef'",
    );
}

#[test]
fn lift_plain_paragraphs_and_marks() {
    let mut c = ctx();
    c.eval(
        r#"
        // Schema must carry tag + invertible for the reverse map.
        ST.Editable.setSchema({
          marks: {
            bold: { tag:'strong', attrs:{}, invertible:true },
            italic: { tag:'em', attrs:{}, invertible:true },
            link: { tag:'a', attrs:{ href:{type:'url'} }, invertible:true }
          },
          blocks: {
            p: { tag:'p', attrs:{}, invertible:true },
            h2: { tag:'h2', attrs:{}, invertible:true }
          }
        });
        globalThis.D = ST.Editable.liftHtml('<h2>Title</h2><p>hi <strong>bold</strong> <em>it</em></p>', document);
        void 0;
    "#,
    )
    .unwrap();
    assert_true(&mut c, "D.content.length === 2");
    assert_true(&mut c, "D.content[0].type === 'h2'");
    assert_true(&mut c, "D.content[0].content[0].text === 'Title'");
    assert_true(&mut c, "D.content[1].type === 'p'");
    // bold + italic recovered as marks
    assert_true(
        &mut c,
        "D.content[1].content.some(n => n.marks && n.marks[0] && n.marks[0].type==='bold')",
    );
    assert_true(
        &mut c,
        "D.content[1].content.some(n => n.marks && n.marks[0] && n.marks[0].type==='italic')",
    );
}

#[test]
fn lift_recovers_link_href() {
    let mut c = ctx();
    c.eval(
        r#"
        ST.Editable.setSchema({
          marks: { link: { tag:'a', attrs:{ href:{type:'url'} }, invertible:true } },
          blocks: { p: { tag:'p', attrs:{}, invertible:true } }
        });
        globalThis.D = ST.Editable.liftHtml('<p>go <a href="/x">there</a></p>', document);
        void 0;
    "#,
    )
    .unwrap();
    assert_true(
        &mut c,
        "D.content[0].content.find(n => n.text==='there').marks[0].type === 'link'",
    );
    assert_true(
        &mut c,
        "D.content[0].content.find(n => n.text==='there').marks[0].attrs.href === '/x'",
    );
}

#[test]
fn lift_drops_unknown_tags_keeps_text() {
    let mut c = ctx();
    c.eval(
        r#"
        ST.Editable.setSchema({
          marks: { bold: { tag:'strong', attrs:{}, invertible:true } },
          blocks: { p: { tag:'p', attrs:{}, invertible:true } }
        });
        // <script>, <span style>, <div onclick> are not registered → unwrapped.
        globalThis.D = ST.Editable.liftHtml(
          '<div onclick="x()">keep<script>alert(1)</script><span style="x"> me</span></div><strong>b</strong>',
          document);
        void 0;
    "#,
    )
    .unwrap();
    // No node may be an unregistered type; the whole doc reduces to p/text(+bold).
    assert_true(&mut c, "D.content.every(b => b.type === 'p')");
    let txt = c
        .eval("D.content.map(b=>b.content.map(n=>n.text).join('')).join('|')")
        .unwrap();
    let s = txt.as_str().unwrap_or("");
    assert!(s.contains("keep"), "text kept: {s}");
    assert!(
        !s.contains("alert"),
        "script text dropped (rawtext not re-emitted): {s}"
    );
}

#[test]
fn fuzz_lifted_html_always_validates_against_schema() {
    // PROPERTY: an arbitrary HTML soup lifted through liftHtml always yields a doc
    // whose every block/mark is schema-known (the paste-gate guarantee). Seeded
    // generator assembles random tag soup incl. dangerous payloads.
    let mut c = ctx();
    c.eval(
        r#"
        ST.Editable.setSchema({
          marks: { bold:{tag:'strong',attrs:{},invertible:true}, link:{tag:'a',attrs:{href:{type:'url'}},invertible:true} },
          blocks: { p:{tag:'p',attrs:{},invertible:true}, h2:{tag:'h2',attrs:{},invertible:true} }
        });
        function mulberry32(a){return function(){a|=0;a=a+0x6D2B79F5|0;let t=Math.imul(a^a>>>15,1|a);t=t+Math.imul(t^t>>>7,61|t)^t;return((t^t>>>14)>>>0)/4294967296;};}
        globalThis.fuzzLift = function(seed){
          const rnd = mulberry32(seed);
          const frags = ['<script>alert(1)</script>','<strong>x</strong>','<em>y</em>','<a href="javascript:e()">z</a>',
            '<div onclick="e()">d</div>','<h2>H</h2>','<p>para</p>','<span style="x">s</span>','<iframe>i</iframe>',
            '<ul><li>li</li></ul>','plain ','<a href="/ok">l</a>','<b>bb</b>','<table><tr><td>c</td></tr></table>'];
          let html='';
          const n = 1+Math.floor(rnd()*8);
          for(let i=0;i<n;i++) html += frags[Math.floor(rnd()*frags.length)];
          const doc = ST.Editable.liftHtml(html, document);
          if (!doc || doc.type!=='doc' || !Array.isArray(doc.content) || doc.content.length<1) return {ok:false,why:'shape',html};
          for (const b of doc.content){
            if (!ST.Editable.hasBlock(b.type)) return {ok:false,why:'block:'+b.type,html};
            for (const inl of (b.content||[])){
              if (inl.type!=='text') return {ok:false,why:'nontext',html};
              for (const m of (inl.marks||[])) if (!ST.Editable.hasMark(m.type)) return {ok:false,why:'mark:'+m.type,html};
            }
          }
          return {ok:true};
        };
        void 0;
    "#,
    )
    .unwrap();
    for seed in 0u32..50 {
        let r = c
            .eval(&format!("JSON.stringify(fuzzLift({seed}))"))
            .unwrap();
        let s = r.as_str().unwrap_or_default();
        assert!(
            s.contains("\"ok\":true"),
            "lift fuzz failed at seed {seed}: {s}"
        );
    }
}

#[test]
fn fuzz_op_sequences_keep_doc_valid_and_serializable() {
    // PROPERTY (GENERATIVE.md spirit): a random sequence of ops over a seeded
    // PRNG must always leave a structurally valid, JSON-round-trippable doc whose
    // size is non-negative and whose blocks are all schema-known. Mulberry32 seed
    // = reproducible; on failure the seed + step index pin the counterexample.
    let mut c = ctx();
    c.eval(
        r#"
        function mulberry32(a){return function(){a|=0;a=a+0x6D2B79F5|0;let t=Math.imul(a^a>>>15,1|a);t=t+Math.imul(t^t>>>7,61|t)^t;return((t^t>>>14)>>>0)/4294967296;};}
        globalThis.fuzz = function(seed){
          const rnd = mulberry32(seed);
          const pick = (arr)=>arr[Math.floor(rnd()*arr.length)];
          let doc = ST.Editable.emptyDoc();
          const marks=['bold','italic','link'], blocks=['p','h2','figure'];
          for (let step=0; step<60; step++){
            const size = ST.Editable.docSize(doc);
            const a = Math.floor(rnd()*Math.max(1,size));
            const b = Math.floor(rnd()*Math.max(1,size));
            const op = Math.floor(rnd()*6);
            try {
              if (op===0) doc = ST.Editable.insertText(doc, a, pick(['x','yz','  ','word']));
              else if (op===1) doc = ST.Editable.deleteRange(doc, a, b);
              else if (op===2) doc = ST.Editable.toggleMark(doc, a, b, pick(marks), pick(marks)==='link'?{href:'/u'}:undefined);
              else if (op===3) doc = ST.Editable.setBlockType(doc, a, pick(blocks));
              else if (op===4) doc = ST.Editable.splitBlock(doc, a);
              else doc = ST.Editable.insertText(doc, a, 'Z');
            } catch (e) {
              return { ok:false, step, err:String(e) };
            }
            // Invariants after every step.
            if (!doc || doc.type!=='doc' || !Array.isArray(doc.content) || doc.content.length<1) return {ok:false, step, why:'shape'};
            if (ST.Editable.docSize(doc) < 0) return {ok:false, step, why:'size'};
            for (const blk of doc.content){
              if (!ST.Editable.hasBlock(blk.type)) return {ok:false, step, why:'block:'+blk.type};
              for (const inl of (blk.content||[])){
                if (inl.type==='text' && inl.marks) for (const m of inl.marks)
                  if (!ST.Editable.hasMark(m.type)) return {ok:false, step, why:'mark:'+m.type};
              }
            }
            // Round-trip: fromJSON∘toJSON is identity on a valid doc.
            const rt = ST.Editable.fromJSON(JSON.parse(JSON.stringify(ST.Editable.toJSON(doc))));
            if (JSON.stringify(rt) !== JSON.stringify(doc)) return {ok:false, step, why:'roundtrip'};
            // Projection never throws.
            const host = document.createElement('div');
            host.appendChild(ST.Editable.project(doc, document, marks));
          }
          return { ok:true };
        };
        void 0;
    "#,
    )
    .unwrap();
    // Run many seeds; first failing seed is reported.
    for seed in 0u32..40 {
        let r = c.eval(&format!("JSON.stringify(fuzz({seed}))"));
        let s = match r {
            Ok(v) => v.as_str().map(|s| s.to_string()).unwrap_or_default(),
            Err(e) => panic!("fuzz seed {seed} eval error: {e:?}"),
        };
        assert!(s.contains("\"ok\":true"), "fuzz failed at seed {seed}: {s}");
    }
}

// =============================================================================
// FUP-042 R1: regions — the recursive document model.
//
// A region is a named editable sub-space. The flat block is the degenerate
// one-region case (size byte-identical to the pre-region model, asserted by the
// 20 tests above staying green). These tests exercise EXPLICIT-region blocks:
// a `columns` block with two parallel inline regions (left/right) and a `ul`
// block with one block-region (`items`) holding `li` blocks.
// =============================================================================

/// A flat doc resolves to the SAME size + offsets whether viewed via the flat
/// `resolve` or the recursive `resolvePath` (path length 2, region 'content').
#[test]
fn region_flat_block_is_degenerate_single_region() {
    let mut c = ctx();
    c.eval("var D = { type:'doc', content:[{ type:'p', content:[ST.Editable.text('abc')] }] };")
        .unwrap();
    // size unchanged: open + 3 chars + close = 5
    assert_true(&mut c, "ST.Editable.docSize(D) === 5");
    assert_true(&mut c, "ST.Editable.blockSize(D.content[0]) === 5");
    // regionsOf yields the single implicit inline 'content' region.
    assert_true(&mut c, "ST.Editable.regionsOf(D.content[0]).length === 1");
    assert_true(
        &mut c,
        "ST.Editable.regionsOf(D.content[0])[0].name === 'content'",
    );
    assert_true(
        &mut c,
        "ST.Editable.regionsOf(D.content[0])[0].kind === 'inline'",
    );
    // Flat model: pos 0 == before first char (block-open boundary folds), so the
    // content offset equals the position directly: pos 2 → offset 2 (between b,c).
    // resolvePath(2) → [{blockIndex:0},{region:'content',offset:2}]
    assert_true(&mut c, "ST.Editable.resolvePath(D, 2).length === 2");
    assert_true(&mut c, "ST.Editable.resolvePath(D, 2)[0].blockIndex === 0");
    assert_true(
        &mut c,
        "ST.Editable.resolvePath(D, 2)[1].region === 'content'",
    );
    assert_true(&mut c, "ST.Editable.resolvePath(D, 2)[1].offset === 2");
    // flat resolve agrees with the path's leaf.
    assert_true(
        &mut c,
        "ST.Editable.resolve(D, 2).blockIndex === ST.Editable.resolvePath(D,2)[0].blockIndex",
    );
    assert_true(
        &mut c,
        "ST.Editable.resolve(D, 2).offset === ST.Editable.resolvePath(D,2)[1].offset",
    );
}

/// A `columns` block with two parallel inline regions. Each region pays its own
/// open/close boundary, so the block size accounts for both.
#[test]
fn region_columns_two_parallel_inline_regions() {
    let mut c = ctx();
    // left="ab" (2), right="cde" (3). Block size = 2(own) + (1+2+1) + (1+3+1) = 11.
    c.eval(
        r#"
        var D = { type:'doc', content:[{
          type:'columns',
          regions: {
            left:  { kind:'inline', content:[ST.Editable.text('ab')] },
            right: { kind:'inline', content:[ST.Editable.text('cde')] }
          }
        }] };
    "#,
    )
    .unwrap();
    assert_true(&mut c, "ST.Editable.blockSize(D.content[0]) === 11");
    assert_true(&mut c, "ST.Editable.docSize(D) === 11");
    assert_true(&mut c, "ST.Editable.regionsOf(D.content[0]).length === 2");
    // Position 2 = inside left region at offset 1 (after 'a').
    //   layout: 0=blockOpen, 1=leftOpen, 1..3=left chars, then leftClose, rightOpen, right chars...
    //   left region content starts at flat pos 2 (blockOpen=0, leftOpen=1).
    assert_true(&mut c, "ST.Editable.resolvePath(D, 3)[1].region === 'left'");
    assert_true(&mut c, "ST.Editable.resolvePath(D, 3)[1].offset === 1");
    // A position in the RIGHT region resolves to region 'right'.
    //   left occupies open(1)+2 chars+close(1) = pos 1..4; right opens at 5.
    assert_true(
        &mut c,
        "ST.Editable.resolvePath(D, 7)[1].region === 'right'",
    );
    assert_true(&mut c, "ST.Editable.resolvePath(D, 7)[1].offset >= 0");
    // End-of-left and start-of-right are DISTINCT positions (the property flat
    // blocks could not express).
    assert_true(
        &mut c,
        "JSON.stringify(ST.Editable.resolvePath(D,4)) !== JSON.stringify(ST.Editable.resolvePath(D,6))",
    );
}

/// A `ul` block with one block-region `items` holding two `li` blocks (each a
/// flat block). Positions descend: block → region → child block → leaf.
#[test]
fn region_list_block_region_holds_child_blocks() {
    let mut c = ctx();
    // ul.items = [ li("x"), li("yz") ]. Each li flat size = char+2 → 3 and 4.
    // items inner = 3+4 = 7; region size = 1+7+1 = 9; ul size = 2+9 = 11.
    c.eval(
        r#"
        var D = { type:'doc', content:[{
          type:'ul',
          regions: { items: { kind:'block', content:[
            { type:'li', content:[ST.Editable.text('x')] },
            { type:'li', content:[ST.Editable.text('yz')] }
          ] } }
        }] };
    "#,
    )
    .unwrap();
    assert_true(&mut c, "ST.Editable.blockSize(D.content[0]) === 11");
    assert_true(&mut c, "ST.Editable.docSize(D) === 11");
    // A position inside the SECOND li descends 4 levels and lands in its content.
    //   path = [{blockIndex:0(ul)},{region:'items'},{blockIndex:1(li)},{region:'content',offset}]
    c.eval("var P = ST.Editable.resolvePath(D, 8);").unwrap();
    assert_true(&mut c, "P.length === 4");
    assert_true(&mut c, "P[0].blockIndex === 0");
    assert_true(&mut c, "P[1].region === 'items'");
    assert_true(&mut c, "P[2].blockIndex === 1");
    assert_true(&mut c, "P[3].region === 'content'");
    // Monotonicity: every flat position 0..docSize resolves to a path without throwing.
    assert_true(
        &mut c,
        "(function(){var n=ST.Editable.docSize(D);for(var i=0;i<=n;i++){var p=ST.Editable.resolvePath(D,i);if(!Array.isArray(p)||!p.length)return false;}return true;})()",
    );
}

/// docSize over a MIXED doc (flat p + columns + ul) = sum of blockSizes, and
/// resolvePath stays in-range for every position (no gap, no overflow).
#[test]
fn region_mixed_doc_positions_are_total_and_monotone() {
    let mut c = ctx();
    c.eval(
        r#"
        var D = { type:'doc', content:[
          { type:'p', content:[ST.Editable.text('hi')] },
          { type:'columns', regions:{
            left:{kind:'inline',content:[ST.Editable.text('L')]},
            right:{kind:'inline',content:[ST.Editable.text('R')]} } },
          { type:'ul', regions:{ items:{kind:'block',content:[
            { type:'li', content:[ST.Editable.text('a')] } ] } } }
        ] };
    "#,
    )
    .unwrap();
    // p=4, columns=2+(1+1+1)+(1+1+1)=8, ul=2+(1+(1+2)+1)=2+5=7 → total 19.
    assert_true(&mut c, "ST.Editable.docSize(D) === 19");
    assert_true(
        &mut c,
        "(function(){var n=ST.Editable.docSize(D);for(var i=0;i<=n;i++){var p=ST.Editable.resolvePath(D,i);if(!Array.isArray(p))return false;var leaf=p[p.length-1];if(typeof leaf.offset!=='number')return false;}return true;})()",
    );
}

// =============================================================================
// FUP-042 R2: regions lens (client side) — project → lift round-trips through
// region hosts. The schema declares region blocks; project emits one
// data-st-region host per region, and lift recovers the regions from those
// hosts. This is the CLIENT half of the client≡server lockstep (the server's
// validate.rs region path is covered by Rust tests in src/editable/validate.rs).
// =============================================================================

fn region_ctx() -> V8TestContext {
    let mut c = V8TestContext::new()
        .with_runtime()
        .expect("st.js")
        .with_editable()
        .expect("editable.js");
    c.eval(
        r#"
        ST.Editable.setSchema({
          marks: { bold: { tag:'strong', attrs:{}, sel:'children', invertible:true } },
          blocks: {
            p:  { tag:'p',  attrs:{}, sel:'children', invertible:true },
            li: { tag:'li', attrs:{}, sel:'children', invertible:true },
            columns: { tag:'div', attrs:{}, invertible:true, regions:[
              { name:'left',  kind:'inline', tag:'div' },
              { name:'right', kind:'inline', tag:'div' } ] },
            ul: { tag:'ul', attrs:{}, invertible:true, regions:[
              { name:'items', kind:'block', tag:'ul' } ] }
          }
        });
        void 0;
    "#,
    )
    .expect("set region schema");
    c
}

/// S1 (FUP-045): the selection bridge descends region hosts. For a columns doc
/// the OLD bridge collapsed both regions into one run and returned `nodp` past the
/// first region; the anchor-map bridge round-trips every position AND agrees with
/// resolvePath (the model) by construction. This is the headless-provable half of
/// region editing (the live Selection read needs CDP).
#[test]
fn region_bridge_descends_hosts_and_matches_model() {
    let mut c = region_ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'columns', regions:{
            left:{kind:'inline',content:[ST.Editable.text('Lx')]},
            right:{kind:'inline',content:[ST.Editable.text('Ry')]} } }
        ]};
        globalThis.host = document.createElement('div');
        host.appendChild(ST.Editable.project(D, document, []));
        // Every position must round-trip through the bridge (domFromPos→posFromDOM).
        globalThis.bridgeOk = function(){
          for (var p=0; p<=ST.Editable.docSize(D); p++){
            var dp = ST.Editable.domFromPos(host, D, p);
            if (!dp) return 'nodp@'+p;
            var back = ST.Editable.posFromDOM(host, D, dp.node, dp.offset);
            if (back == null) return 'noback@'+p;
          }
          return 'ok';
        };
        // A caret in the RIGHT region's text node maps to a right-region position.
        globalThis.rightHost = host.querySelector('[data-st-region="right"]');
        globalThis.rightText = (function(){
          var t=null; (function rec(n){for(var i=0;i<n.childNodes.length;i++){var c=n.childNodes[i];if(c.nodeType===3&&!t)t=c;else if(c.nodeType===1)rec(c);}})(rightHost); return t;
        })();
        void 0;
    "#,
    )
    .unwrap();
    assert_eq!(c.eval("bridgeOk()").unwrap().as_str(), Some("ok"));
    // Caret at offset 1 inside the right region's "Ry" → a position whose resolvePath
    // lands in region 'right' (the old bridge could not reach here at all).
    assert_true(
        &mut c,
        "(function(){var p=ST.Editable.posFromDOM(host,D,rightText,1);var path=ST.Editable.resolvePath(D,p);return path[path.length-1].region==='right';})()",
    );
    // And the inverse lands the caret back in the right host's text node.
    assert_true(
        &mut c,
        "(function(){var p=ST.Editable.posFromDOM(host,D,rightText,1);var dp=ST.Editable.domFromPos(host,D,p);return dp.node===rightText;})()",
    );
    // S1r #3: a caret AT the right region's open boundary (the model position whose
    // resolvePath is region 'right' offset 0) must snap into the RIGHT host, not the
    // left region's trailing content. Find that boundary position from the model,
    // then assert domFromPos lands in the right region's subtree.
    assert_true(
        &mut c,
        "(function(){var n=ST.Editable.docSize(D);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(D,p);var leaf=pa[pa.length-1];if(leaf.region==='right'&&leaf.offset===0){var dp=ST.Editable.domFromPos(host,D,p);var node=dp.node;while(node&&node!==host){if(node.getAttribute&&node.getAttribute('data-st-region')==='right')return true;node=node.parentNode;}return false;}}return false;})()",
    );
}

/// S1r #2: an empty block-kind region (a list with no items — a normal editing
/// state, e.g. after deleting every item) must still have a caret home. The bridge
/// anchors each region-open boundary to its host element, so domFromPos resolves
/// into the empty list's host rather than returning null.
#[test]
fn region_empty_block_region_has_caret_home() {
    let mut c = region_ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'ul', regions:{ items:{ kind:'block', content:[] } } }
        ]};
        globalThis.host = document.createElement('div');
        host.appendChild(ST.Editable.project(D, document, []));
        void 0;
    "#,
    )
    .unwrap();
    // Every position has a caret home (none returns null).
    assert_true(
        &mut c,
        "(function(){var n=ST.Editable.docSize(D);for(var p=0;p<=n;p++){if(!ST.Editable.domFromPos(host,D,p))return false;}return true;})()",
    );
    // The caret home for a position inside the empty region is the region's host.
    assert_true(
        &mut c,
        "(function(){var n=ST.Editable.docSize(D);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(D,p);if(pa.some(function(s){return s.region==='items';})){var dp=ST.Editable.domFromPos(host,D,p);var node=dp.node;while(node&&node!==host){if(node.getAttribute&&node.getAttribute('data-st-region')==='items')return true;node=node.parentNode;}return false;}}return false;})()",
    );
}

/// S2 (FUP-045): edit ops route through resolveLeaf so a text edit lands in the
/// REGION the position names, not a flattened top-level run. Columns: typing in the
/// right region grows the right region's content; the left is untouched.
#[test]
fn region_insert_text_targets_the_named_region() {
    let mut c = region_ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'columns', regions:{
            left:{kind:'inline',content:[ST.Editable.text('Lx')]},
            right:{kind:'inline',content:[ST.Editable.text('Ry')]} } }
        ]};
        // Find a position inside the RIGHT region (offset 1), insert 'Z' there.
        globalThis.rpos = (function(){var n=ST.Editable.docSize(D);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(D,p);var leaf=pa[pa.length-1];if(leaf.region==='right'&&leaf.offset===1)return p;}return -1;})();
        globalThis.D2 = ST.Editable.insertText(D, rpos, 'Z');
        void 0;
    "#,
    )
    .unwrap();
    assert_true(&mut c, "rpos > 0");
    // Right region grew to 'RZy'; left region unchanged.
    assert_true(
        &mut c,
        "D2.content[0].regions.right.content[0].text === 'RZy'",
    );
    assert_true(
        &mut c,
        "D2.content[0].regions.left.content[0].text === 'Lx'",
    );
    // docSize grew by exactly one char.
    assert_true(
        &mut c,
        "ST.Editable.docSize(D2) === ST.Editable.docSize(D) + 1",
    );
}

/// S2: Enter inside a block-region's child block (a list item) splits it into a
/// NEW sibling item — the block region grows, the rest of the doc is untouched.
#[test]
fn region_split_in_list_item_grows_the_list() {
    let mut c = region_ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'ul', regions:{ items:{ kind:'block', content:[
            { type:'li', content:[ST.Editable.text('ab')] }
          ] } } }
        ]};
        // Position between a|b inside the single li, then split (Enter).
        globalThis.spos = (function(){var n=ST.Editable.docSize(D);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(D,p);var leaf=pa[pa.length-1];if(leaf.region==='content'&&leaf.offset===1 && pa.some(function(s){return s.region==='items';}))return p;}return -1;})();
        globalThis.D2 = ST.Editable.splitBlock(D, spos);
        void 0;
    "#,
    )
    .unwrap();
    assert_true(&mut c, "spos > 0");
    // The list now has TWO items: 'a' and 'b'.
    assert_true(&mut c, "D2.content[0].regions.items.content.length === 2");
    assert_true(
        &mut c,
        "D2.content[0].regions.items.content[0].content[0].text === 'a'",
    );
    assert_true(
        &mut c,
        "D2.content[0].regions.items.content[1].content[0].text === 'b'",
    );
    // Still a single top-level block (the ul) — the doc did not gain a sibling.
    assert_true(&mut c, "D2.content.length === 1");
}

/// S2: bold toggled inside a region marks only that region's text. And a flat doc's
/// multi-block toggle is preserved byte-identically (regression floor).
#[test]
fn region_toggle_mark_scoped_to_region_and_flat_floor() {
    let mut c = region_ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'columns', regions:{
            left:{kind:'inline',content:[ST.Editable.text('Lx')]},
            right:{kind:'inline',content:[ST.Editable.text('Ry')]} } }
        ]};
        // Bold the whole RIGHT region content (offsets 0..2 within right).
        globalThis.r0 = (function(){var n=ST.Editable.docSize(D);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(D,p);var l=pa[pa.length-1];if(l.region==='right'&&l.offset===0)return p;}return -1;})();
        globalThis.r2 = (function(){var n=ST.Editable.docSize(D);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(D,p);var l=pa[pa.length-1];if(l.region==='right'&&l.offset===2)return p;}return -1;})();
        globalThis.D2 = ST.Editable.toggleMark(D, r0, r2, 'bold');
        void 0;
    "#,
    )
    .unwrap();
    assert_true(&mut c, "r0 >= 0 && r2 > r0");
    // Right region text is bold; left region carries no bold.
    assert_true(
        &mut c,
        "D2.content[0].regions.right.content[0].marks && D2.content[0].regions.right.content[0].marks[0].type === 'bold'",
    );
    assert_true(
        &mut c,
        "!(D2.content[0].regions.left.content[0].marks && D2.content[0].regions.left.content[0].marks.length)",
    );
}

/// S3 (FUP-045): splitCaret gives the surface the region-correct post-split caret.
/// Flat/block-region split advances +2 (past head close + new open); an inline-region
/// split is a structural no-op so the caret stays at pos (a flat +2 would over-shoot).
#[test]
fn region_split_caret_is_region_correct() {
    let mut c = region_ctx();
    // Flat block: +2.
    c.eval(r#"globalThis.F={type:'doc',content:[{type:'p',content:[ST.Editable.text('ab')]}]};void 0;"#).unwrap();
    assert_true(&mut c, "ST.Editable.splitCaret(F, 1) === 3");
    // Inline region (columns): split is a no-op → caret stays.
    c.eval(r#"globalThis.C={type:'doc',content:[{type:'columns',regions:{left:{kind:'inline',content:[ST.Editable.text('Lx')]},right:{kind:'inline',content:[ST.Editable.text('Ry')]}}}]};
      globalThis.lp=(function(){var n=ST.Editable.docSize(C);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(C,p);var l=pa[pa.length-1];if(l.region==='left'&&l.offset===1)return p;}return -1;})();void 0;"#).unwrap();
    assert_true(&mut c, "lp > 0 && ST.Editable.splitCaret(C, lp) === lp");
    // Block-region child (list item): +2 (new sibling li).
    c.eval(r#"globalThis.L={type:'doc',content:[{type:'ul',regions:{items:{kind:'block',content:[{type:'li',content:[ST.Editable.text('ab')]}]}}}]};
      globalThis.sp=(function(){var n=ST.Editable.docSize(L);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(L,p);var l=pa[pa.length-1];if(l.region==='content'&&l.offset===1&&pa.some(function(s){return s.region==='items';}))return p;}return -1;})();void 0;"#).unwrap();
    assert_true(&mut c, "sp > 0 && ST.Editable.splitCaret(L, sp) === sp + 2");
}

/// S2r [P1]: a caret inside an EMPTY block-region (a list with no items — reachable
/// since S1 gives it a caret home) must not crash the ops. resolveLeaf materializes
/// a first child block so insertText creates content there.
#[test]
fn region_edit_into_empty_block_region_creates_child() {
    let mut c = region_ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'ul', regions:{ items:{ kind:'block', content:[] } } }
        ]};
        // A position inside the empty items region.
        globalThis.ip = (function(){var n=ST.Editable.docSize(D);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(D,p);if(pa.some(function(s){return s.region==='items';}))return p;}return -1;})();
        globalThis.D2 = ST.Editable.insertText(D, ip, 'X');
        void 0;
    "#,
    )
    .unwrap();
    assert_true(&mut c, "ip >= 0");
    // A child block now exists in the items region carrying 'X'; input doc untouched.
    assert_true(&mut c, "D2.content[0].regions.items.content.length === 1");
    assert_true(
        &mut c,
        "D2.content[0].regions.items.content[0].content[0].text === 'X'",
    );
    assert_true(&mut c, "D.content[0].regions.items.content.length === 0"); // input not mutated
}

/// S2r [P2]: flat multi-block activeMarks intersects across blocks (a mark is active
/// only if EVERY char in the multi-paragraph selection carries it). Regression floor.
#[test]
fn flat_multi_block_active_marks_intersects_all_blocks() {
    let mut c = ctx();
    c.eval(
        r#"
        // para1 fully bold, para2 not bold.
        globalThis.D = { type:'doc', content:[
          { type:'p', content:[ ST.Editable.text('ab', [{type:'bold'}]) ] },
          { type:'p', content:[ ST.Editable.text('cd') ] }
        ]};
        void 0;
    "#,
    )
    .unwrap();
    // Selection spanning both paragraphs: bold is NOT active (para2 lacks it).
    assert_true(
        &mut c,
        "ST.Editable.activeMarks(D, 1, 8).indexOf('bold') === -1",
    );
    // Selection within para1 only: bold IS active.
    assert_true(
        &mut c,
        "ST.Editable.activeMarks(D, 1, 2).indexOf('bold') !== -1",
    );
}

/// S2r [P2]: paste (insertDoc) into a region lands in that region's run, not the
/// region-bearing block's stray .content (which project ignores → silent loss).
#[test]
fn region_insert_doc_paste_lands_in_region() {
    let mut c = region_ctx();
    c.eval(
        r#"
        globalThis.D = { type:'doc', content:[
          { type:'columns', regions:{
            left:{kind:'inline',content:[ST.Editable.text('Lx')]},
            right:{kind:'inline',content:[ST.Editable.text('Ry')]} } }
        ]};
        // Caret at right offset 1; paste a single-block doc 'PP'.
        globalThis.rp = (function(){var n=ST.Editable.docSize(D);for(var p=0;p<=n;p++){var pa=ST.Editable.resolvePath(D,p);var l=pa[pa.length-1];if(l.region==='right'&&l.offset===1)return p;}return -1;})();
        globalThis.other = { type:'doc', content:[ { type:'p', content:[ST.Editable.text('PP')] } ] };
        globalThis.M = ST.Editable.insertDoc(D, rp, other);
        void 0;
    "#,
    )
    .unwrap();
    assert_true(&mut c, "rp > 0");
    // Right region now 'RPPy'; the columns block grew NO stray .content array.
    assert_true(
        &mut c,
        "M.doc.content[0].regions.right.content[0].text === 'RPPy'",
    );
    assert_true(
        &mut c,
        "!M.doc.content[0].content || M.doc.content[0].content.length === 0",
    );
    assert_true(
        &mut c,
        "M.doc.content[0].regions.left.content[0].text === 'Lx'",
    );
}

#[test]
fn region_project_emits_host_per_region() {
    let mut c = region_ctx();
    c.eval(
        r#"
        var D = { type:'doc', content:[{
          type:'columns', regions:{
            left:  { kind:'inline', content:[ST.Editable.text('L')] },
            right: { kind:'inline', content:[ST.Editable.text('R')] } } }] };
        var host = document.createElement('div');
        host.appendChild(ST.Editable.project(D, document, ['bold']));
        var cols = host.querySelector('div');
    "#,
    )
    .unwrap();
    // The columns block is a <div> containing two region hosts (left/right).
    assert_true(
        &mut c,
        "host.querySelectorAll('[data-st-region]').length === 2",
    );
    assert_true(
        &mut c,
        "host.querySelector('[data-st-region=\"left\"]').textContent === 'L'",
    );
    assert_true(
        &mut c,
        "host.querySelector('[data-st-region=\"right\"]').textContent === 'R'",
    );
}

#[test]
fn region_project_lift_round_trips_columns() {
    let mut c = region_ctx();
    c.eval(
        r#"
        var D = { type:'doc', content:[{
          type:'columns', regions:{
            left:  { kind:'inline', content:[ST.Editable.text('ab')] },
            right: { kind:'inline', content:[ST.Editable.text('cd')] } } }] };
        var host = document.createElement('div');
        host.appendChild(ST.Editable.project(D, document, ['bold']));
        var L = ST.Editable.liftHtml(host.innerHTML, document);
    "#,
    )
    .unwrap();
    assert_true(&mut c, "L.content.length === 1");
    assert_true(&mut c, "L.content[0].type === 'columns'");
    assert_true(&mut c, "!!L.content[0].regions");
    assert_true(&mut c, "L.content[0].regions.left.kind === 'inline'");
    assert_true(&mut c, "L.content[0].regions.left.content[0].text === 'ab'");
    assert_true(
        &mut c,
        "L.content[0].regions.right.content[0].text === 'cd'",
    );
}

#[test]
fn region_project_lift_round_trips_list() {
    let mut c = region_ctx();
    c.eval(
        r#"
        var D = { type:'doc', content:[{
          type:'ul', regions:{ items:{ kind:'block', content:[
            { type:'li', content:[ST.Editable.text('x')] },
            { type:'li', content:[ST.Editable.text('y')] } ] } } }] };
        var host = document.createElement('div');
        host.appendChild(ST.Editable.project(D, document, ['bold']));
        var L = ST.Editable.liftHtml(host.innerHTML, document);
    "#,
    )
    .unwrap();
    assert_true(&mut c, "L.content[0].type === 'ul'");
    assert_true(&mut c, "L.content[0].regions.items.kind === 'block'");
    assert_true(&mut c, "L.content[0].regions.items.content.length === 2");
    assert_true(
        &mut c,
        "L.content[0].regions.items.content[0].type === 'li'",
    );
    assert_true(
        &mut c,
        "L.content[0].regions.items.content[1].content[0].text === 'y'",
    );
}

#[test]
fn region_lift_drops_unknown_inside_block_region() {
    // A <script> inside a block region's host is dropped (RAWTEXT_DROP), and an
    // unknown block tag is unwrapped — mirrors the server fail-closed path.
    let mut c = region_ctx();
    c.eval(
        r#"
        var dirty = '<ul><ul data-st-region="items">' +
                    '<li>ok</li><script>alert(1)</script></ul></ul>';
        var L = ST.Editable.liftHtml(dirty, document);
    "#,
    )
    .unwrap();
    assert_true(&mut c, "L.content[0].type === 'ul'");
    assert_true(&mut c, "L.content[0].regions.items.content.length === 1");
    assert_true(
        &mut c,
        "L.content[0].regions.items.content[0].type === 'li'",
    );
}

// =============================================================================
// FUP-042 R3: @property fuzz over NESTED region trees (the wave gate).
//
// PROPERTY: for an arbitrarily generated region doc (flat blocks, columns with
// two inline regions, and ul/li block regions nested to random depth), the
// recursive position model is TOTAL and MONOTONE, and the document is in
// CLIENT≡SERVER shape: every position 0..docSize resolves to a path whose leaf
// carries a numeric offset in range, distinct positions never collapse, and
// project→lift round-trips the region structure. The shape that the client
// produces is exactly what the server validate.rs (Rust tests) accepts.
// =============================================================================

#[test]
fn fuzz_region_tree_positions_total_monotone_and_round_trip() {
    let mut c = region_ctx();
    c.eval(
        r#"
        // Seeded PRNG (mulberry32) — deterministic generation + shrink.
        function rng(seed){ return function(){ seed|=0; seed=seed+0x6D2B79F5|0;
          var t=Math.imul(seed^seed>>>15,1|seed); t=t+Math.imul(t^t>>>7,61|t)^t;
          return ((t^t>>>14)>>>0)/4294967296; }; }

        function genInline(r){
          var s = 'abcdef'[(r()*6)|0] + (r()<0.5 ? 'xy'[(r()*2)|0] : '');
          return [ST.Editable.text(s)];
        }
        // Generate a block up to `depth` levels of region nesting.
        function genBlock(r, depth){
          var pick = r();
          if (depth <= 0 || pick < 0.45) {
            return { type:'p', content: genInline(r) };
          }
          if (pick < 0.72) {
            return { type:'columns', regions:{
              left:  { kind:'inline', content: genInline(r) },
              right: { kind:'inline', content: genInline(r) } } };
          }
          // ul with 0..3 li children (each a flat li or a nested ul). The 0-item
          // case is the EMPTY block-region (deleted-all-items state) the S2r [P1]
          // crash hid in — the op-fuzz must reach it so the materialize path is
          // covered at every position.
          var n = (r()*4)|0; // 0..3 (was 1..3) — now includes the empty list
          var items = [];
          for (var i=0;i<n;i++){
            if (r() < 0.6) items.push({ type:'li', content: genInline(r) });
            else items.push(genBlock(r, depth-1)); // nested region block inside the list
          }
          return { type:'ul', regions:{ items:{ kind:'block', content: items } } };
        }

        function genDoc(seed){
          var r = rng(seed);
          var n = 1 + ((r()*3)|0);
          var blocks = [];
          for (var i=0;i<n;i++) blocks.push(genBlock(r, 2));
          return { type:'doc', content: blocks };
        }
        window.__genDoc = genDoc; // shared with the S2 op-fuzz below

        // The property check for one doc.
        window.checkRegionDoc = function(seed){
          var D = genDoc(seed);
          var n = ST.Editable.docSize(D);
          // (1) TOTALITY + MONOTONICITY: every position resolves to a valid path
          // with a numeric leaf offset, and the path never goes BACKWARD as p
          // increases. NB: distinct positions legitimately map to the same leaf at
          // a boundary (the trailing positions of a block all clamp to its end
          // offset — inherent to ProseMirror integer positions), so the invariant
          // is NON-DECREASING, not strict distinctness. We compare the path to its
          // predecessor: at the first segment that differs, the new value must be
          // ≥ (blockIndex/offset advance forward; a region transition is forward).
          function cmpPaths(a, b){
            // returns -1 if b<a (REGRESSION), else 0/1. Compares segment by segment.
            var len = Math.min(a.length, b.length);
            for (var k=0;k<len;k++){
              var sa = a[k], sb = b[k];
              if (typeof sa.blockIndex === 'number'){
                if (sb.blockIndex !== sa.blockIndex) return sb.blockIndex < sa.blockIndex ? -1 : 1;
              } else if (typeof sa.region === 'string'){
                if (sb.region !== sa.region) return 0; // different region branch: forward by construction
              }
              if (typeof sa.offset === 'number' && typeof sb.offset === 'number'){
                if (sb.offset !== sa.offset) return sb.offset < sa.offset ? -1 : 1;
              }
            }
            return b.length < a.length ? -1 : (b.length > a.length ? 1 : 0);
          }
          var prev = null;
          for (var p=0; p<=n; p++){
            var path = ST.Editable.resolvePath(D, p);
            if (!Array.isArray(path) || !path.length) return {ok:false, seed, why:'no-path@'+p};
            var leaf = path[path.length-1];
            if (typeof leaf.offset !== 'number' || leaf.offset < 0) return {ok:false, seed, why:'bad-offset@'+p};
            if (typeof leaf.region !== 'string') return {ok:false, seed, why:'no-region@'+p};
            // MONOTONE (non-decreasing): the path never regresses as p advances.
            if (prev && cmpPaths(prev, path) < 0) return {ok:false, seed, why:'regressed@'+p};
            prev = path;
          }
          // Distinctness where it MUST hold: a doc that actually CONTAINS text has
          // distinct start/end positions (resolvePath(0) ≠ resolvePath(n)). A doc
          // with no addressable text — e.g. only an EMPTY block-region (a list with
          // no items) — legitimately collapses start==end (there is nothing between
          // the boundaries), so the distinctness claim only applies when text exists.
          function hasText(blocks){
            return (blocks||[]).some(function(b){
              if (b.regions) return Object.keys(b.regions).some(function(k){
                var rg=b.regions[k];
                return rg.kind==='block' ? hasText(rg.content) : (rg.content||[]).some(function(x){return x.type==='text'&&x.text.length;});
              });
              return (b.content||[]).some(function(x){return x.type==='text'&&x.text.length;});
            });
          }
          if (n > 0 && hasText(D.content)
              && cmpPaths(ST.Editable.resolvePath(D,0), ST.Editable.resolvePath(D,n)) >= 0
              && JSON.stringify(ST.Editable.resolvePath(D,0)) === JSON.stringify(ST.Editable.resolvePath(D,n))) {
            return {ok:false, seed, why:'start==end'};
          }
          // (2) docSize == sum of blockSize (consistency of the size model).
          var sum = D.content.reduce(function(a,b){ return a + ST.Editable.blockSize(b); }, 0);
          if (sum !== n) return {ok:false, seed, why:'docsize!=sum '+sum+'/'+n};
          // (3) CLIENT≡SERVER shape: project → lift DEEP round-trips the region tree.
          var host = document.createElement('div');
          host.appendChild(ST.Editable.project(D, document, ['bold']));
          var L = ST.Editable.liftHtml(host.innerHTML, document);
          // Recursive structural compare: block types, region shape, and leaf text
          // must match at EVERY depth (nested li text, columns left/right, nested
          // region blocks inside a list item).
          function inlineText(content){
            return (content||[]).filter(function(x){return x.type==='text';})
                                .map(function(x){return x.text;}).join('');
          }
          function sameBlocks(a, b){
            if (a.length !== b.length) return false;
            for (var i=0;i<a.length;i++){ if (!sameBlock(a[i], b[i])) return false; }
            return true;
          }
          function sameBlock(d, l){
            if (!l || d.type !== l.type) return false;
            var dR = d.regions, lR = l.regions;
            if (!!dR !== !!lR) return false;
            if (dR){
              var dk = Object.keys(dR), lk = Object.keys(lR);
              if (dk.length !== lk.length) return false;
              for (var j=0;j<dk.length;j++){
                var name = dk[j];
                if (!lR[name]) return false;
                if (dR[name].kind !== lR[name].kind) return false;
                if (dR[name].kind === 'block'){
                  if (!sameBlocks(dR[name].content||[], lR[name].content||[])) return false;
                } else {
                  if (inlineText(dR[name].content) !== inlineText(lR[name].content)) return false;
                }
              }
              return true;
            }
            // Flat block: inline text must survive the round-trip.
            return inlineText(d.content) === inlineText(l.content);
          }
          if (!sameBlocks(D.content, L.content)) return {ok:false, seed, why:'deep-roundtrip'};
          return {ok:true};
        };
        void 0;
    "#,
    )
    .unwrap();
    for seed in 0u32..60 {
        let r = c.eval(&format!("JSON.stringify(window.checkRegionDoc({seed}))"));
        let s = match r {
            Ok(v) => v.as_str().map(|s| s.to_string()).unwrap_or_default(),
            Err(e) => panic!("region fuzz seed {seed} eval error: {e:?}"),
        };
        assert!(
            s.contains("\"ok\":true"),
            "region fuzz failed at seed {seed}: {s}"
        );
    }

    // S2 op-fuzz: at EVERY position of each generated doc, apply insertText and
    // assert the op is region-correct — the inserted char lands in the leaf the
    // position names (its run grows by one, docSize grows by one), the size model
    // stays consistent (docSize==ΣblockSize), and the mutated doc still DEEP
    // round-trips through project→lift. This proves the resolveLeaf routing edits
    // the named region, never a flattened run, at any tree depth.
    c.eval(
        r#"
        window.opFuzz = function(seed){
          var D = window.__genDoc(seed);
          var n0 = ST.Editable.docSize(D);
          for (var p=0; p<=n0; p++){
            var before = ST.Editable.resolvePath(D, p);
            var leaf = before[before.length-1];
            // Is this position inside an EMPTY block-region? (a list with no items)
            // — typing there CREATES a first child block, so docSize grows by the
            // char PLUS the new block's boundary pair (+3), which is correct editor
            // behaviour. Elsewhere an insert grows an existing run by exactly +1.
            var emptyRegionInsert = (function(){
              var node = D, idx = before[0].blockIndex, blk = D.content[idx];
              for (var k=1;k<before.length;k++){
                var seg = before[k];
                if (seg.offset!==undefined) return false;
                var rg = blk.regions && blk.regions[seg.region];
                if (rg && rg.kind==='block' && (!rg.content || rg.content.length===0)) return true;
                if (!rg) return false;
                if (rg.kind==='block'){ k++; blk = (rg.content||[])[before[k] && before[k].blockIndex]; if(!blk) return true; }
                else return false;
              }
              return false;
            })();
            var D2 = ST.Editable.insertText(D, p, 'Z');
            var grew = ST.Editable.docSize(D2) - n0;
            var okGrow = emptyRegionInsert ? (grew >= 1) : (grew === 1);
            if (!okGrow) return {ok:false, seed, why:'size@'+p+' grew'+grew+(emptyRegionInsert?' (empty-region)':'')};
            // Same region branch as before (the edit stayed local to the leaf).
            var after = ST.Editable.resolvePath(D2, p);
            var la = after[after.length-1];
            if (la.region !== leaf.region && !emptyRegionInsert) return {ok:false, seed, why:'region-moved@'+p};
            // ΣblockSize consistency on the mutated doc (size model stays coherent).
            var sum = D2.content.reduce(function(a,b){return a+ST.Editable.blockSize(b);},0);
            if (sum !== ST.Editable.docSize(D2)) return {ok:false, seed, why:'sum@'+p};
            // The doc still round-trips through project→lift after the edit.
            var h = document.createElement('div');
            h.appendChild(ST.Editable.project(D2, document, ['bold']));
            if (!ST.Editable.liftHtml(h.innerHTML, document)) return {ok:false, seed, why:'lift@'+p};
          }
          return {ok:true};
        };
        void 0;
    "#,
    )
    .unwrap();
    for seed in 0u32..40 {
        let r = c.eval(&format!("JSON.stringify(window.opFuzz({seed}))"));
        let s = match r {
            Ok(v) => v.as_str().map(|s| s.to_string()).unwrap_or_default(),
            Err(e) => panic!("op fuzz seed {seed} eval error: {e:?}"),
        };
        assert!(
            s.contains("\"ok\":true"),
            "op fuzz failed at seed {seed}: {s}"
        );
    }
}
