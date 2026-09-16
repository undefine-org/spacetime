/**
 * Spacetime Editable — structured rich-text AST model (PLAN-031 / FEAT-098).
 *
 * The editor edits a CONSTRAINED DOCUMENT AST, not contenteditable innerHTML.
 * HTML is a PROJECTION of that AST; the AST is the source of truth (and the wire
 * format). This module is the pure kernel: the document shape, integer positions
 * (ProseMirror-style), the edit operations, and AST→DOM projection. It reads NO
 * DOM state back — every change is an op on the AST followed by a re-projection.
 *
 * Deliberately framework-free and side-effect-free (except `project` which builds
 * detached DOM nodes): it is unit-tested headless in V8 with no browser. The live
 * surface (editable-surface, beforeinput interception, selection mapping) sits on
 * top of this in a separate module.
 *
 * ## Document shape
 *   doc   = { type: "doc", content: Block[] }
 *   Block = { type: <blockName>, attrs?, content: Inline[] }      e.g. p, h2, figure
 *   Inline= { type: "text", text: string, marks?: Mark[] }
 *   Mark  = { type: <markName>, attrs? }                          e.g. bold, link{href}
 *
 * ## Positions (ProseMirror model)
 * A flat integer address into the document. Position 0 is before the first block's
 * content. Entering a block costs 1 (the opening boundary); each text character
 * costs 1; leaving a block costs 1 (closing boundary). This makes every edit a
 * range [from,to) over a single integer line, and keeps the selection stable
 * across projection.
 */
(function (global) {
  'use strict';

  var ST = global.ST = global.ST || {};
  var E = ST.Editable = ST.Editable || {};

  // ---- Schema registry (populated from ST.editable.* / editable-schema.json) ----
  // The set of allowed marks/blocks. Each entry: { tag, attrs:{name:{type}}, sel }.
  // The model validates every op against this — an unknown mark/block is rejected,
  // so the AST can never hold a node the schema doesn't describe.
  E.schema = E.schema || { marks: {}, blocks: {} };

  E.setSchema = function (schema) {
    E.schema = {
      marks: (schema && schema.marks) || {},
      blocks: (schema && schema.blocks) || {},
    };
  };

  E.hasMark = function (name) { return !!E.schema.marks[name]; };
  E.hasBlock = function (name) { return !!E.schema.blocks[name]; };

  // =========================================================================
  // Construction
  // =========================================================================

  /** An empty document: a single empty paragraph (the minimal valid doc). */
  E.emptyDoc = function () {
    return { type: 'doc', content: [{ type: 'p', content: [] }] };
  };

  /** A text inline node with optional marks (marks array is normalized/sorted). */
  E.text = function (str, marks) {
    var node = { type: 'text', text: String(str == null ? '' : str) };
    if (marks && marks.length) node.marks = normalizeMarks(marks);
    return node;
  };

  // Marks are identity-compared by (type + JSON attrs); normalize sorts by type
  // and de-dupes so two equal mark-sets compare structurally and render in a
  // stable nesting order (registration order is applied at projection time).
  function normalizeMarks(marks) {
    var seen = {};
    var out = [];
    marks.forEach(function (m) {
      var key = m.type + ':' + JSON.stringify(m.attrs || null);
      if (!seen[key]) { seen[key] = 1; out.push({ type: m.type, attrs: m.attrs || undefined }); }
    });
    out.sort(function (a, b) { return a.type < b.type ? -1 : a.type > b.type ? 1 : 0; });
    return out;
  }
  E.normalizeMarks = normalizeMarks;

  function markEq(a, b) {
    return a.type === b.type && JSON.stringify(a.attrs || null) === JSON.stringify(b.attrs || null);
  }
  function hasMark(node, type) {
    return !!(node.marks && node.marks.some(function (m) { return m.type === type; }));
  }

  // =========================================================================
  // Positions + sizes (ProseMirror integer model)
  // =========================================================================

  /** Size of an inline node = its character length (text) or 1 (atom). */
  function inlineSize(node) {
    return node.type === 'text' ? node.text.length : 1;
  }

  /** Sum of inline sizes over a content array. */
  function inlineContentSize(content) {
    return (content || []).reduce(function (n, c) { return n + inlineSize(c); }, 0);
  }

  /** Inline content size of a (flat) block. Kept for flat-only call sites. */
  function blockContentSize(block) {
    return inlineContentSize(block.content);
  }

  // =========================================================================
  // Regions (FUP-042 R1) — the recursive document primitive
  //
  // A *region* is a named, ordered editable sub-space:
  //     Region = { kind: 'inline' | 'block', content: Node[] }
  // The flat block is the DEGENERATE one-region case: a block with no `.regions`
  // key has a single implicit inline region named 'content' whose boundaries
  // COINCIDE with the block's own open/close — so its size stays `content + 2`
  // and every flat-doc position is byte-identical to the pre-region model.
  //
  // A block that opts in carries `.regions = { name: { kind, content } }`. Each
  // explicit region pays its own open/close boundary pair (so positions can
  // distinguish e.g. end-of-left-column from start-of-right-column, and a
  // block-region's child blocks nest with their own boundaries). Lists ride a
  // block-region (`ul.regions.items` holds `li` blocks); columns ride parallel
  // inline regions (`columns.regions = { left, right }`).
  // =========================================================================

  // The SCHEMA is the single source of truth for whether a block has regions, so
  // project (forward), lift (backward), the server validator, and the position
  // math all agree on one block shape. A block is a region block iff its schema
  // declares regions; the AST node then SUPPLIES the per-region content (a
  // declared region the AST omits is an empty region). When the schema is unknown
  // (e.g. headless tests with no setSchema), fall back to the AST `.regions` key.
  function blockRegionSpecs(block) {
    var spec = E.schema.blocks[block.type];
    if (spec && spec.regions && spec.regions.length) return spec.regions;
    // Schema-less fallback: trust the AST shape (keeps pure-kernel tests working).
    if (!spec && block.regions) {
      return Object.keys(block.regions).map(function (name) {
        return { name: name, kind: (block.regions[name] || {}).kind || 'inline' };
      });
    }
    return null;
  }

  /** True for a flat block (the schema declares no regions → degenerate case). */
  function isFlatBlock(block) { return !blockRegionSpecs(block); }

  /**
   * Normalized region list for a block, in document order, driven by the SCHEMA's
   * declared regions (names + kinds authoritative); the AST node supplies each
   * region's content, an omitted region yielding empty content. A flat block
   * yields its single implicit inline region over `block.content`.
   */
  function regionList(block) {
    var specs = blockRegionSpecs(block);
    if (specs) {
      var astRegions = block.regions || {};
      return specs.map(function (s) {
        var r = astRegions[s.name] || {};
        return { name: s.name, kind: s.kind || r.kind || 'inline', content: r.content || [] };
      });
    }
    return [{ name: 'content', kind: 'inline', content: block.content || [], implicit: true }];
  }
  E.regionsOf = regionList;

  /** Addressable inner size of a region's content (NOT counting the region's own
   *  boundary pair): inline → Σ char sizes; block → Σ child block sizes. */
  function regionInnerSize(region) {
    if (region.kind === 'block') {
      return (region.content || []).reduce(function (n, b) { return n + blockSize(b); }, 0);
    }
    return inlineContentSize(region.content);
  }

  /**
   * Total size of a block = open boundary + its regions + close boundary.
   * Flat (degenerate) block: the single implicit region's boundaries fold into
   * the block's own, so size = inlineContentSize + 2 (UNCHANGED from the flat
   * model). Explicit-regioned block: 2 (own) + Σ over regions of (1 + inner + 1).
   */
  function blockSize(block) {
    if (isFlatBlock(block)) return inlineContentSize(block.content) + 2;
    var inner = 0;
    regionList(block).forEach(function (r) { inner += regionInnerSize(r) + 2; });
    return inner + 2;
  }
  E.blockSize = blockSize;

  /**
   * Document size = sum over top-level blocks of blockSize. For a flat doc this
   * equals the old `Σ (contentSize + 2)` exactly; region-bearing blocks add the
   * per-region boundary costs.
   */
  E.docSize = function (doc) {
    return doc.content.reduce(function (n, b) { return n + blockSize(b); }, 0);
  };

  /**
   * Resolve a flat position to { blockIndex, offset } — the FLAT (degenerate)
   * view, preserved byte-for-byte for flat docs (callers that predate regions).
   * Steps the accumulator by blockSize so region-bearing blocks are skipped
   * correctly; `offset` is clamped into the block's addressable inner span.
   * Positions inside a region-bearing block should use `resolvePath` (R2+).
   */
  E.resolve = function (doc, pos) {
    var p = Math.max(0, Math.min(pos, E.docSize(doc)));
    var acc = 0;
    for (var i = 0; i < doc.content.length; i++) {
      var innerSpan = blockSize(doc.content[i]) - 2; // addressable span inside the block
      var open = acc;            // position just inside this block's start
      var close = acc + innerSpan;
      if (p <= close + 1) {
        return { blockIndex: i, offset: Math.max(0, Math.min(p - open, innerSpan)) };
      }
      acc = close + 2;           // == open + blockSize
    }
    var last = doc.content.length - 1;
    return { blockIndex: last < 0 ? 0 : last, offset: last < 0 ? 0 : blockSize(doc.content[last]) - 2 };
  };

  /**
   * Resolve a flat position to a recursive PATH down the region tree. The path
   * is an array: a leading `{ blockIndex }`, then alternating `{ region }` /
   * `{ blockIndex }` descents, terminating in `{ region, offset }` at the inline
   * leaf. A flat doc yields a length-2 path `[{blockIndex}, {region:'content',
   * offset}]` — the strict generalization of `resolve`'s `{blockIndex, offset}`.
   */
  E.resolvePath = function (doc, pos) {
    var p = Math.max(0, Math.min(pos, E.docSize(doc)));
    return resolveBlockSeq(doc.content, p);
  };

  // Resolve `local` (relative to the sequence start) over a Block[] sequence.
  // `local-acc` is the position measured from the block's start (its open
  // boundary), passed UNIFORMLY for flat and region blocks. The flat/region
  // distinction lives in resolveWithinBlock: a flat block folds its boundary
  // budget so offset 0 coincides with the block start (legacy `resolve`
  // convention, byte-identical); a region block spends the block-open boundary
  // before its first region.
  function resolveBlockSeq(blocks, local) {
    var acc = 0;
    for (var i = 0; i < blocks.length; i++) {
      var b = blocks[i];
      var size = blockSize(b);
      if (local <= acc + size - 1) {
        return [{ blockIndex: i }].concat(resolveWithinBlock(b, local - acc));
      }
      acc += size;
    }
    var last = blocks.length - 1;
    if (last < 0) return [{ blockIndex: 0 }, { region: 'content', offset: 0 }];
    var lb = blocks[last];
    return [{ blockIndex: last }].concat(resolveWithinBlock(lb, blockSize(lb) - 2));
  }

  // Resolve `rel` (0 = the block's open boundary) within one block.
  function resolveWithinBlock(block, rel) {
    if (isFlatBlock(block)) {
      // Legacy convention: offset 0 == block start; close boundary clamps to end.
      var inner = inlineContentSize(block.content);
      return [{ region: 'content', offset: Math.max(0, Math.min(rel, inner)) }];
    }
    // Region block: rel 0 is the block-open boundary; regions follow it.
    var off = rel - 1; // measure from just inside the block open
    var regs = regionList(block);
    for (var k = 0; k < regs.length; k++) {
      var r = regs[k];
      var rsize = regionInnerSize(r) + 2; // region open + inner + close
      if (off <= rsize - 1) {
        var innerLocal = off - 1; // 0 = region content start
        if (r.kind === 'inline') {
          var ri = regionInnerSize(r);
          return [{ region: r.name, offset: Math.max(0, Math.min(innerLocal, ri)) }];
        }
        return [{ region: r.name }].concat(resolveBlockSeq(r.content, innerLocal));
      }
      off -= rsize;
    }
    var lastR = regs[regs.length - 1];
    if (lastR.kind === 'inline') {
      return [{ region: lastR.name, offset: regionInnerSize(lastR) }];
    }
    return [{ region: lastR.name }].concat(
      resolveBlockSeq(lastR.content, regionInnerSize(lastR))
    );
  }
  // =========================================================================
  // Inline helpers: split / slice / merge a block's content at char offsets
  // =========================================================================

  /** Flatten a block's content to a per-character array of {ch, marks}. */
  function explode(block) {
    var cells = [];
    (block.content || []).forEach(function (n) {
      if (n.type === 'text') {
        for (var i = 0; i < n.text.length; i++) cells.push({ ch: n.text[i], marks: n.marks || [] });
      } else {
        cells.push({ atom: n, marks: n.marks || [] });
      }
    });
    return cells;
  }

  /** Rebuild a block's content array from per-character cells, coalescing runs. */
  function implode(cells) {
    var out = [];
    cells.forEach(function (cell) {
      if (cell.atom) { out.push(cell.atom); return; }
      var last = out[out.length - 1];
      var marks = normalizeMarks(cell.marks);
      if (last && last.type === 'text' && sameMarks(last.marks || [], marks)) {
        last.text += cell.ch;
      } else {
        out.push(E.text(cell.ch, marks));
      }
    });
    return out;
  }

  function sameMarks(a, b) {
    if (a.length !== b.length) return false;
    for (var i = 0; i < a.length; i++) if (!markEq(a[i], b[i])) return false;
    return true;
  }

  // =========================================================================
  // Operations — each returns a NEW doc (immutable; never mutates input)
  // =========================================================================

  function clone(doc) { return JSON.parse(JSON.stringify(doc)); }

  /**
   * Resolve a position to the INLINE LEAF it lives in, generalizing `resolve`'s
   * {blockIndex,offset} down the region tree. Returns the mutable container whose
   * `.content` is the inline run, plus the sibling Block[] sequence the leaf block
   * lives in (for split/merge), all referencing nodes INSIDE `d` (so callers
   * mutate in place). A flat block yields { owner:block, block, sequence:d.content,
   * seqIndex:blockIndex, offset } — byte-identical to `resolve`, so flat ops are
   * unchanged. An inline region (columns) yields owner = the region object
   * (sequence:null → not structurally splittable). A block region (list) descends
   * into its child-block sequence. Materializes an omitted-but-declared region's
   * container so an edit into an empty region persists.
   */
  function resolveLeaf(d, pos) {
    var path = E.resolvePath(d, pos);
    var sequence = d.content;
    var seqIndex = path[0].blockIndex;
    var block = sequence[seqIndex];
    for (var i = 1; i < path.length; i++) {
      var seg = path[i];
      if (seg.offset !== undefined) {
        if (isFlatBlock(block)) {
          return { owner: block, block: block, sequence: sequence, seqIndex: seqIndex, offset: seg.offset };
        }
        // Inline region leaf: owner is the region object (materialize if omitted).
        if (!block.regions) block.regions = {};
        if (!block.regions[seg.region]) block.regions[seg.region] = { kind: 'inline', content: [] };
        return { owner: block.regions[seg.region], block: block, sequence: null, seqIndex: -1, offset: seg.offset };
      }
      // Block-region descent: {region} then {blockIndex} into its child blocks.
      if (!block.regions) block.regions = {};
      if (!block.regions[seg.region]) block.regions[seg.region] = { kind: 'block', content: [] };
      sequence = block.regions[seg.region].content;
      i++;
      seqIndex = path[i].blockIndex;
      block = sequence[seqIndex];
      if (!block) {
        // EMPTY block-region (a declared list with no items yet, e.g. after
        // deleting every li): resolvePath yields a phantom child index with no
        // backing block. Materialize a first empty child block so an edit into the
        // empty region creates content (on the cloned doc — never the input). The
        // child type is the region host's natural item: derived from the region
        // spec's child tag if present, else 'p' (a safe paragraph). The next path
        // segment is the child's inline-region offset, handled below.
        block = { type: emptyRegionChildType(seg.region), content: [] };
        sequence[seqIndex] = block;
      }
    }
    // Degenerate guard (empty doc / out of range): treat as flat block start.
    return { owner: block, block: block, sequence: sequence, seqIndex: seqIndex, offset: 0 };
  }

  // The block type to synthesize for the first child of an empty block-region.
  // Prefer a schema hint (region.childType / the region host's li-like tag), else
  // a paragraph. Kept conservative so the synthesized block validates server-side.
  function emptyRegionChildType(regionName) {
    var blocks = E.schema && E.schema.blocks;
    if (blocks) {
      // A block whose schema declares this region tells us nothing about the child
      // type directly; common case is a list whose item block is 'li'. Use 'li' if
      // the schema registers it, else the first registered non-region block, else p.
      if (blocks.li) return 'li';
      for (var k in blocks) {
        if (blocks.hasOwnProperty(k) && !(blocks[k].regions && blocks[k].regions.length)) return k;
      }
    }
    return 'p';
  }

  /** Insert plain text (inheriting marks at the insertion point) at a position. */
  E.insertText = function (doc, pos, str) {
    if (!str) return doc;
    var d = clone(doc);
    var r = resolveLeaf(d, pos);
    var cells = explode(r.owner);
    var inherit = r.offset > 0 ? cells[r.offset - 1].marks : (cells[0] ? cells[0].marks : []);
    var ins = [];
    for (var i = 0; i < str.length; i++) ins.push({ ch: str[i], marks: inherit });
    cells.splice.apply(cells, [r.offset, 0].concat(ins));
    r.owner.content = implode(cells);
    return d;
  };

  /** Delete the range [from,to) (collapses if equal). Same-leaf deletes splice the
   *  inline run; a cross-block delete WITHIN one sequence (flat doc, or two child
   *  blocks of the same block-region) merges head+tail and drops the between
   *  blocks. A delete whose endpoints cross a REGION boundary (different leaf
   *  containers / sequences) clamps to the anchor leaf — cross-region ranges are a
   *  deferred S-arc item (S0 doc); clamping is safe (no cross-region data loss). */
  E.deleteRange = function (doc, from, to) {
    if (from === to) return doc;
    var lo = Math.min(from, to), hi = Math.max(from, to);
    var d = clone(doc);
    var a = resolveLeaf(d, lo), b = resolveLeaf(d, hi);
    if (a.owner === b.owner) {
      var cells = explode(a.owner);
      cells.splice(a.offset, b.offset - a.offset);
      a.owner.content = implode(cells);
      return d;
    }
    // Different leaves but the SAME block sequence (flat doc, or sibling child
    // blocks within one block-region): merge head of first + tail of last, drop
    // the blocks between. `sequence` is null for an inline-region leaf, so this
    // only fires for structurally-mergeable siblings.
    if (a.sequence && a.sequence === b.sequence && a.seqIndex !== b.seqIndex) {
      var first = a.sequence[a.seqIndex], last = b.sequence[b.seqIndex];
      if (isFlatBlock(first) && isFlatBlock(last)) {
        var headCells = explode(first).slice(0, a.offset);
        var tailCells = explode(last).slice(b.offset);
        first.content = implode(headCells.concat(tailCells));
        a.sequence.splice(a.seqIndex + 1, b.seqIndex - a.seqIndex);
        return d;
      }
    }
    // Cross-region (or otherwise non-mergeable): clamp to the anchor leaf, deleting
    // from `a.offset` to the end of its run. Safe, deterministic, no data loss
    // outside the anchor region. True cross-region delete is a deferred follow-up.
    var ac = explode(a.owner);
    ac.splice(a.offset);
    a.owner.content = implode(ac);
    return d;
  };

  /**
   * Toggle a mark over the range [from,to). If EVERY character in range already
   * carries the mark (same type), remove it; otherwise add it to all. `attrs`
   * attaches to the added mark (e.g. link href). Validates against the schema.
   */
  E.toggleMark = function (doc, from, to, type, attrs) {
    if (!E.hasMark(type)) return doc;
    if (from === to) return doc;
    var lo = Math.min(from, to), hi = Math.max(from, to);
    var d = clone(doc);
    var a = resolveLeaf(d, lo), b = resolveLeaf(d, hi);
    // Flat multi-block selection (both leaves flat, same top-level sequence): the
    // original cross-block toggle, byte-identical — a mark can span paragraphs.
    if (a.sequence && a.sequence === d.content && b.sequence === d.content &&
        isFlatBlock(a.block) && isFlatBlock(b.block)) {
      var ra = { blockIndex: a.seqIndex, offset: a.offset };
      var rb = { blockIndex: b.seqIndex, offset: b.offset };
      var allHaveF = true;
      forEachCellInRange(d, ra, rb, function (cell) {
        if (!cell.marks.some(function (m) { return m.type === type; })) allHaveF = false;
      });
      forEachCellInRange(d, ra, rb, function (cell) {
        var marks = cell.marks.filter(function (m) { return m.type !== type; });
        if (!allHaveF) marks = marks.concat([{ type: type, attrs: attrs || undefined }]);
        cell.set(marks);
      });
      return d;
    }
    // Region leaf (or cross-leaf within a region): toggle within ONE inline run.
    // A cross-leaf selection clamps to the anchor leaf (cross-region marking is the
    // deferred cross-region-selection follow-up; clamping is safe).
    var owner = a.owner;
    var cells = explode(owner);
    var start = a.offset;
    var end = (a.owner === b.owner) ? b.offset : cells.length;
    var allHave = true;
    for (var i = start; i < end; i++) {
      if (!cells[i] || !cells[i].marks.some(function (m) { return m.type === type; })) { allHave = false; break; }
    }
    for (var j = start; j < end; j++) {
      if (!cells[j]) continue;
      var marks = cells[j].marks.filter(function (m) { return m.type !== type; });
      if (!allHave) marks = marks.concat([{ type: type, attrs: attrs || undefined }]);
      cells[j].marks = marks;
    }
    owner.content = implode(cells);
    return d;
  };

  // Walk every per-character cell in [a,b) across blocks, allowing in-place set.
  function forEachCellInRange(d, a, b, fn) {
    for (var bi = a.blockIndex; bi <= b.blockIndex; bi++) {
      var block = d.content[bi];
      var cells = explode(block);
      var start = bi === a.blockIndex ? a.offset : 0;
      var end = bi === b.blockIndex ? b.offset : cells.length;
      var changed = false;
      for (var i = start; i < end; i++) {
        (function (cell) {
          cell.set = function (m) { cell.marks = m; changed = true; };
          fn(cell);
        })(cells[i]);
      }
      if (changed) block.content = implode(cells);
    }
  }

  /** Set a block's type (e.g. p → h2). Targets the LEAF block at `pos` (a list
   *  item inside a block-region, or a top-level flat block). An inline-region leaf
   *  has no own block to retype (its container is the region), so it is a no-op.
   *  Validates against the schema. */
  E.setBlockType = function (doc, pos, type, attrs) {
    if (!E.hasBlock(type)) return doc;
    var d = clone(doc);
    var r = resolveLeaf(d, pos);
    if (!r.sequence) return d; // inline-region leaf: no own block
    var blk = r.sequence[r.seqIndex];
    blk.type = type;
    if (attrs) blk.attrs = attrs;
    else delete blk.attrs;
    return d;
  };

  /** Split the block at `pos` into two (Enter key). In a flat block (or a child
   *  block of a block-region) the split inserts a new sibling INTO THE SAME
   *  sequence — so Enter inside a list item makes a new list item (the block region
   *  grows), and Enter in a flat paragraph makes a new paragraph (unchanged).
   *  Inside an INLINE region (columns), there is no block sequence to grow, so the
   *  run is split in place into two text inlines (a soft break) rather than a new
   *  block — an inline region holds text, not blocks. */
  E.splitBlock = function (doc, pos) {
    var d = clone(doc);
    var r = resolveLeaf(d, pos);
    var cells = explode(r.owner);
    var head = cells.slice(0, r.offset), tail = cells.slice(r.offset);
    if (r.sequence) {
      var block = r.sequence[r.seqIndex];
      block.content = implode(head);
      var next = { type: block.type, content: implode(tail) };
      if (block.attrs) next.attrs = JSON.parse(JSON.stringify(block.attrs));
      r.sequence.splice(r.seqIndex + 1, 0, next);
      return d;
    }
    // Inline region: no block sequence — keep all content in the run (the head and
    // tail are already contiguous inlines; implode coalesces). A split here is a
    // no-op on structure (an inline region cannot hold a second block).
    r.owner.content = implode(head.concat(tail));
    return d;
  };

  /**
   * The caret position AFTER splitBlock(doc,pos): the start of the new tail
   * content. A structural split (flat block or a block-region child) advances the
   * caret past the head block's close boundary and the new block's open boundary
   * (+2). An inline-region split is a structural no-op, so the caret stays at
   * `pos`. The surface uses this instead of a hardcoded `+2` so the caret lands
   * correctly inside a region (a flat `+2` would over-shoot an inline-region split).
   */
  E.splitCaret = function (doc, pos) {
    var r = resolveLeaf(clone(doc), pos);
    return r.sequence ? pos + 2 : pos;
  };

  /**
   * Insert (merge) a whole document `other` into `doc` at position `pos`. The
   * first block of `other` is spliced INLINE into the block at the cursor; any
   * further blocks of `other` are inserted as new blocks after it. Returns
   * `{ doc, pos }` where pos is the caret position after the inserted content.
   * This is the merge used by paste (lift → insertDoc).
   */
  E.insertDoc = function (doc, pos, other) {
    if (!other || !other.content || !other.content.length) return { doc: doc, pos: pos };
    var d = clone(doc);
    var r = resolveLeaf(d, pos); // region-aware: the run the caret actually sits in
    var cells = explode(r.owner);
    var head = cells.slice(0, r.offset);
    var tail = cells.slice(r.offset);

    var srcBlocks = clone(other).content;
    // First source block: its inline content joins the current run inline.
    var firstCells = blockToCells(srcBlocks[0]);
    var consumed = firstCells.length;

    // Single source block, OR a leaf with no own block sequence (an inline region
    // — a column — cannot grow blocks): flatten all source inline into THIS run.
    // For an inline-region leaf this is the safe degradation for a multi-block
    // paste (no structural loss; the text survives, joined into the column).
    if (srcBlocks.length === 1 || !r.sequence) {
      var allCells = firstCells;
      for (var s = 1; s < srcBlocks.length; s++) allCells = allCells.concat(blockToCells(srcBlocks[s]));
      r.owner.content = implode(head.concat(allCells).concat(tail));
      return { doc: d, pos: pos + allCells.length };
    }

    // Multi-block paste into a block sequence (flat doc, or a list's child blocks):
    // head + first source inline stays in the current block; middle source blocks
    // insert verbatim as siblings IN THE SAME SEQUENCE; the LAST source block gets
    // the tail appended. So a multi-paragraph paste inside a list inserts new list
    // siblings, not top-level blocks.
    var leafBlock = r.sequence[r.seqIndex];
    leafBlock.content = implode(head.concat(firstCells));
    var insertAt = r.seqIndex + 1;
    var newBlocks = [];
    for (var i = 1; i < srcBlocks.length; i++) {
      var sb = srcBlocks[i];
      if (i === srcBlocks.length - 1) {
        var lastCells = blockToCells(sb);
        sb.content = implode(lastCells.concat(tail));
        newBlocks.push(sb);
      } else {
        newBlocks.push(sb);
      }
    }
    r.sequence.splice.apply(r.sequence, [insertAt, 0].concat(newBlocks));
    // Caret: the end of the last inserted block's own (pre-tail) inline. For a flat
    // top-level paste this matches the legacy absolute position; for a nested paste
    // we approximate with pos + total inserted inline length (monotone, in-range).
    var insertedInline = 0;
    for (var j = 0; j < srcBlocks.length; j++) insertedInline += blockToCells(srcBlocks[j]).length;
    var caret = (r.sequence === d.content)
      ? positionAtBlockOffset(d, insertAt + newBlocks.length - 1, blockToCells(srcBlocks[srcBlocks.length - 1]).length)
      : pos + insertedInline;
    return { doc: d, pos: caret };
  };

  // Per-character cells for a block's inline content (reuses explode).
  function blockToCells(block) {
    return explode({ content: block.content || [] });
  }

  // Flat position of `offset` characters into the block at `blockIndex`.
  function positionAtBlockOffset(doc, blockIndex, offset) {
    var acc = 0;
    for (var i = 0; i < blockIndex && i < doc.content.length; i++) {
      acc += blockContentSize(doc.content[i]) + 2;
    }
    return acc + 1 + offset; // +1 for this block's open boundary
  }

  // =========================================================================
  // Active-mark / block queries (drive toolbar state signals)
  // =========================================================================

  /** Marks active across the WHOLE range (intersection); for a collapsed
   *  selection, the marks of the character to the left. */
  E.activeMarks = function (doc, from, to) {
    var lo = Math.min(from, to), hi = Math.max(from, to);
    var dd = clone(doc);
    if (lo === hi) {
      var r = resolveLeaf(dd, Math.max(0, lo));
      var cells = explode(r.owner);
      var c = cells[r.offset - 1];
      return c ? c.marks.map(function (m) { return m.type; }) : [];
    }
    var a = resolveLeaf(dd, lo), b = resolveLeaf(dd, hi);
    // Flat multi-block selection: intersect marks across every block a..b (a mark
    // is active only if EVERY char carries it). Byte-identical to the pre-region
    // path — the same floor toggleMark preserves, so toolbar state stays correct
    // when a selection spans paragraphs.
    if (a.sequence && a.sequence === dd.content && b.sequence === dd.content &&
        isFlatBlock(a.block) && isFlatBlock(b.block)) {
      var ra = { blockIndex: a.seqIndex, offset: a.offset };
      var rb = { blockIndex: b.seqIndex, offset: b.offset };
      var interF = null;
      forEachCellInRange(dd, ra, rb, function (cell) {
        var types = cell.marks.map(function (m) { return m.type; });
        if (interF === null) interF = types;
        else interF = interF.filter(function (t) { return types.indexOf(t) !== -1; });
      });
      return interF || [];
    }
    // Region leaf (or single-leaf): intersection over the anchor leaf's run
    // (clamped end for a cross-leaf range; cross-region active-marks is deferred).
    var rcells = explode(a.owner);
    var start = a.offset, end = (a.owner === b.owner) ? b.offset : rcells.length;
    var inter = null;
    for (var i = start; i < end; i++) {
      if (!rcells[i]) continue;
      var types = rcells[i].marks.map(function (m) { return m.type; });
      if (inter === null) inter = types;
      else inter = inter.filter(function (t) { return types.indexOf(t) !== -1; });
    }
    return inter || [];
  };

  /** The block type at a position — the LEAF block (list item or flat block). An
   *  inline-region leaf reports its containing block's type. */
  E.blockTypeAt = function (doc, pos) {
    var r = resolveLeaf(clone(doc), pos);
    return (r.sequence ? r.sequence[r.seqIndex] : r.block).type;
  };

  /** Whether the doc is effectively empty (one empty block). */
  E.isEmpty = function (doc) {
    return doc.content.length === 1 && blockContentSize(doc.content[0]) === 0;
  };

  // =========================================================================
  // Projection: AST → DOM (the forward direction of the lens)
  // =========================================================================

  /**
   * Project the doc into a detached DOM fragment. Each block becomes its schema
   * tag; inline runs are wrapped in their marks' tags in REGISTRATION order
   * (outer mark = earlier-registered), so nesting is deterministic. `doc` argument
   * supplies the element factory (document) so this is testable with a fake.
   */
  E.project = function (doc, document, markOrder) {
    var frag = document.createDocumentFragment();
    doc.content.forEach(function (block) {
      frag.appendChild(projectBlock(block, document, markOrder));
    });
    return frag;
  };

  // Project one block to its DOM element. A flat block renders its inline content
  // directly into the block element (unchanged). A region block renders each
  // region into a child host element (the region's `tag`, default <div>): an
  // inline region gets its inline runs; a block region gets its child blocks
  // projected recursively. This is the forward half of the regions lens; lift
  // (backward) descends the same region hosts.
  function projectBlock(block, document, markOrder) {
    var spec = E.schema.blocks[block.type];
    var tag = (spec && spec.tag) || block.type;
    var el = document.createElement(tag);
    if (block.attrs) Object.keys(block.attrs).forEach(function (k) { el.setAttribute(k, block.attrs[k]); });
    if (isFlatBlock(block)) {
      projectInlines(block.content || [], el, document, markOrder);
      return el;
    }
    // Region block: one host element per region, in declared order.
    var regSpecs = (spec && spec.regions) || [];
    regionList(block).forEach(function (r) {
      var rs = regSpecs.filter(function (x) { return x.name === r.name; })[0];
      var host = document.createElement((rs && rs.tag) || 'div');
      host.setAttribute('data-st-region', r.name);
      if (r.kind === 'block') {
        (r.content || []).forEach(function (child) {
          host.appendChild(projectBlock(child, document, markOrder));
        });
      } else {
        projectInlines(r.content || [], host, document, markOrder);
      }
      el.appendChild(host);
    });
    return el;
  }

  function projectInlines(inlines, parent, document, markOrder) {
    inlines.forEach(function (node) {
      if (node.type !== 'text') return; // atoms: W3
      var marks = (node.marks || []).slice();
      // Order marks by registration priority (markOrder), else alphabetical.
      marks.sort(function (a, b) {
        var ia = markOrder ? markOrder.indexOf(a.type) : -1;
        var ib = markOrder ? markOrder.indexOf(b.type) : -1;
        if (ia === -1) ia = 1e9; if (ib === -1) ib = 1e9;
        return ia - ib;
      });
      var leaf = document.createTextNode(node.text);
      var wrapped = leaf;
      // Wrap inside-out: last mark in order is innermost.
      for (var i = marks.length - 1; i >= 0; i--) {
        var m = marks[i];
        var spec = E.schema.marks[m.type];
        var tag = (spec && spec.tag) || m.type;
        var wrapper = document.createElement(tag);
        if (m.attrs) Object.keys(m.attrs).forEach(function (k) { wrapper.setAttribute(k, m.attrs[k]); });
        wrapper.appendChild(wrapped);
        wrapped = wrapper;
      }
      parent.appendChild(wrapped);
    });
  }

  // =========================================================================
  // Serialization (wire format = the AST itself; this is just stable JSON)
  // =========================================================================

  // =========================================================================
  // Selection ↔ position bridge (links integer positions to the projected DOM)
  //
  // The surface projects the doc into a host element, then must map the browser
  // Selection (anchor/focus = DOM node + offset) to integer positions, and back.
  // The projection is deterministic, so we walk the host's text nodes in document
  // order and accumulate the SAME costs docSize uses (1 per block boundary, 1 per
  // char). LinkeDOM headless has no Selection, so these run only under a real
  // browser (CDP) — kept in the kernel so they are projection-consistent.
  // =========================================================================

  // Anchor map: one deterministic walk of the DOC structure spending the IDENTICAL
  // boundary budget as blockSize/resolvePath (block open/close + per-region
  // open/close), associating each addressable model position with its projected
  // DOM (text node, offset). Because the budget matches the model exactly,
  // posFromDOM and domFromPos agree with `resolve`/`resolvePath` BY CONSTRUCTION
  // — for flat AND region docs (the pre-region bridge used a block-only budget,
  // which silently disagreed with the post-FUP-042 folded flat convention and
  // collapsed region hosts into one run). `doc` is the structural guide; the DOM
  // supplies the concrete leaf nodes the browser Selection points at.
  function buildAnchors(host, doc) {
    var anchors = [];
    walkBlockSeqAnchors((doc && doc.content) || [], host, 0, anchors);
    return anchors;
  }

  // Element children in projection order (the projection emits no stray text
  // between block elements, so element children are the blocks 1:1).
  function childBlockEls(parentEl) {
    var out = [];
    for (var i = 0; i < parentEl.children.length; i++) out.push(parentEl.children[i]);
    return out;
  }

  // The region host elements of a region block, in declared order (project tags
  // each with data-st-region; the same marker lift reads back).
  function regionHostEls(blockEl) {
    var out = [];
    for (var i = 0; i < blockEl.children.length; i++) {
      var c = blockEl.children[i];
      if (c.getAttribute && c.getAttribute('data-st-region') != null) out.push(c);
    }
    return out;
  }

  function walkBlockSeqAnchors(blocks, parentEl, base, anchors) {
    var els = childBlockEls(parentEl);
    var acc = base;
    for (var i = 0; i < blocks.length; i++) {
      walkBlockAnchors(blocks[i], els[i], acc, anchors);
      acc += blockSize(blocks[i]);
    }
  }

  function walkBlockAnchors(block, el, base, anchors) {
    if (!el) return;
    if (isFlatBlock(block)) {
      // Flat (degenerate) block: content positions start at `base` (the folded
      // convention — offset 0 coincides with the block start, matching resolve).
      emitInlineAnchors(block.content || [], el, base, anchors);
      return;
    }
    // Region block: `base` is the block-open boundary; each region spends its
    // own open boundary (+1) before its content, mirroring resolveWithinBlock.
    var hosts = regionHostEls(el);
    var regs = regionList(block);
    var p = base + 1;
    for (var k = 0; k < regs.length; k++) {
      var r = regs[k];
      var hostEl = hosts[k];
      if (hostEl) {
        // Anchor the region-OPEN boundary (model pos `p`) to the host element.
        // The model's resolveWithinBlock maps this boundary into THIS region
        // (innerLocal clamps to content offset 0). Anchoring it (a) gives an
        // EMPTY region a caret home (an empty block-region emits no content
        // anchor) and (b) makes a caret AT the boundary snap into this region's
        // host, not the previous region's trailing content. Content starts at
        // p+1 (one position past the open boundary), matching resolveWithinBlock.
        anchors.push({ pos: p, node: hostEl, offset: 0 });
        if (r.kind === 'block') walkBlockSeqAnchors(r.content || [], hostEl, p + 1, anchors);
        else emitInlineAnchors(r.content || [], hostEl, p + 1, anchors);
      }
      p += regionInnerSize(r) + 2;
    }
  }

  // Anchor every char boundary of an inline run sequence. The DOM text nodes of
  // `containerEl` are in 1:1 document order with the run's text inlines (project
  // emits one Text leaf per text inline, wrapped by marks); an atom occupies one
  // position with no text caret. An empty run gets a single container anchor so
  // an empty block/region still has a caret home.
  function emitInlineAnchors(inlines, containerEl, startPos, anchors) {
    var texts = collectTextNodes(containerEl);
    var ti = 0, pos = startPos, emitted = false;
    for (var i = 0; i < inlines.length; i++) {
      var node = inlines[i];
      if (node.type === 'text') {
        var dom = texts[ti++];
        var len = node.text.length;
        for (var o = 0; o <= len; o++) anchors.push({ pos: pos + o, node: dom, offset: o });
        pos += len; emitted = true;
      } else {
        pos += 1; // atom: one position, no text caret
      }
    }
    if (!emitted) anchors.push({ pos: startPos, node: containerEl, offset: 0 });
  }

  /**
   * Map a (DOM node, offset) inside a projected host to an integer model
   * position. Walks the doc structure to anchor every position, then returns the
   * anchor on `node` nearest `offset`. Region-aware and model-consistent by
   * construction. Returns null if the node is not an anchored leaf.
   */
  E.posFromDOM = function (host, doc, node, offset) {
    if (!doc) return null;
    var anchors = buildAnchors(host, doc);
    var best = null, bestD = Infinity;
    for (var i = 0; i < anchors.length; i++) {
      var a = anchors[i];
      if (a.node !== node) continue;
      var d = Math.abs(a.offset - offset);
      if (d < bestD) { bestD = d; best = a; }
    }
    return best ? best.pos : null;
  };

  function collectTextNodes(el) {
    var out = [];
    (function rec(n) {
      for (var i = 0; i < n.childNodes.length; i++) {
        var c = n.childNodes[i];
        if (c.nodeType === 3) out.push(c);
        else if (c.nodeType === 1) rec(c);
      }
    })(el);
    return out;
  }

  /**
   * Map an integer model position back to a (DOM node, offset) inside the host,
   * for restoring a selection after re-projection. The exact inverse of
   * posFromDOM: same anchor map, picks the anchor at `pos` (clamped into range).
   * Region-aware by construction. Returns null only for an empty host.
   */
  E.domFromPos = function (host, doc, pos) {
    if (!doc) return null;
    var anchors = buildAnchors(host, doc);
    if (!anchors.length) return null;
    var lo = anchors[0].pos, hi = anchors[anchors.length - 1].pos;
    var want = Math.max(lo, Math.min(pos, hi));
    var best = anchors[0], bestD = Infinity;
    for (var i = 0; i < anchors.length; i++) {
      var d = Math.abs(anchors[i].pos - want);
      if (d < bestD) { bestD = d; best = anchors[i]; }
    }
    return { node: best.node, offset: best.offset };
  };

  E.toJSON = function (doc) { return clone(doc); };
  E.fromJSON = function (obj) {
    if (!obj || obj.type !== 'doc' || !Array.isArray(obj.content)) return E.emptyDoc();
    return clone(obj);
  };

  // =========================================================================
  // Paste lift: HTML → doc AST (the BACKWARD direction of the projection lens)
  //
  // A mark/block is a bidirectional lens over a selection: forward = render
  // (project), backward = lift (recover the AST node from pasted HTML by matching
  // it against the registered, INVERTIBLE mark/block bodies). The schema tells us,
  // per node, which projected `tag` maps back to which mark/block + how its attrs
  // map to params. So lifting is: parse the pasted HTML, walk it, and for each
  // element ask "is there an invertible block whose tag == this tag?" (→ block) or
  // "…an invertible mark?" (→ wrap the text in that mark). Tags with no inverse are
  // UNWRAPPED (children kept); everything reduces to schema-known nodes only — the
  // exact security property the server then re-checks (FEAT-099).
  //
  // Pure + DOM-light: it needs a `document` to parse HTML (a <template>), then
  // walks the resulting node tree. Headless-testable with LinkeDOM.
  // =========================================================================

  // Rawtext / embedded tags whose CONTENT must be dropped entirely on lift (not
  // unwrapped to text): a pasted `<script>alert(1)</script>` must not survive even
  // as the literal text "alert(1)" (mirrors richtext.rs rawtext policy). These are
  // never registrable as marks/blocks, so they are always dropped here.
  var RAWTEXT_DROP = {
    script: 1, style: 1, iframe: 1, object: 1, embed: 1, noscript: 1, template: 1,
    svg: 1, math: 1, xmp: 1, noembed: 1, noframes: 1, title: 1, textarea: 1, plaintext: 1
  };

  // Build reverse lookup tables tag→markName / tag→blockName from the schema,
  // restricted to invertible entries (only those have a sound backward map).
  function buildInverse() {
    var markByTag = {}, blockByTag = {};
    Object.keys(E.schema.marks).forEach(function (name) {
      var m = E.schema.marks[name];
      if (m && m.tag && m.invertible !== false) markByTag[m.tag.toLowerCase()] = { name: name, spec: m };
    });
    Object.keys(E.schema.blocks).forEach(function (name) {
      var b = E.schema.blocks[name];
      if (b && b.tag && b.invertible !== false) blockByTag[b.tag.toLowerCase()] = { name: name, spec: b };
    });
    return { markByTag: markByTag, blockByTag: blockByTag };
  }

  /**
   * Lift an HTML string into a doc AST, keeping only schema-known nodes. `document`
   * is the element factory used to parse. Returns a valid doc (≥ one block).
   */
  E.liftHtml = function (html, document) {
    var inv = buildInverse();
    var tpl = document.createElement('template');
    tpl.innerHTML = String(html == null ? '' : html);
    var root = tpl.content || tpl;

    var blocks = [];
    // Top-level: each block-tag element becomes a block; loose inline content is
    // gathered into an implicit paragraph (if `p` is a registered block).
    var pending = []; // inline nodes accumulating outside any block
    function flushPending() {
      if (!pending.length) return;
      var blockName = inv.blockByTag['p'] ? inv.blockByTag['p'].name : firstBlockName(inv);
      if (blockName) blocks.push({ type: blockName, content: pending });
      pending = [];
    }
    eachChild(root, function (node) {
      if (node.nodeType === 1) {
        var tag = node.tagName.toLowerCase();
        if (RAWTEXT_DROP[tag]) return; // drop script/style/iframe/… content entirely
        var hit = inv.blockByTag[tag];
        if (hit) {
          flushPending();
          blocks.push(liftBlock(node, hit, inv));
          return;
        }
        // Not a block tag: treat its content as inline (it may carry marks).
        pending = pending.concat(liftInlines(node, inv, []));
      } else if (node.nodeType === 3) {
        if (node.textContent) pending.push(E.text(node.textContent));
      }
    });
    flushPending();
    if (!blocks.length) blocks.push({ type: firstBlockName(inv) || 'p', content: [] });
    return { type: 'doc', content: blocks };
  };

  // Lift one block-tag element into a block AST node. A flat block (no `regions`
  // in its schema) recovers its inline `content` (unchanged). A region block
  // recovers each declared region from its host element (matched by the host's
  // `data-st-region` attr, or positionally by declared order as a fallback): an
  // inline region lifts inline runs, a block region lifts child block elements
  // recursively. This is the backward half of the regions lens — the same region
  // hosts that `projectBlock` emitted.
  function liftBlock(node, hit, inv) {
    var regSpecs = (hit.spec && hit.spec.regions) || [];
    if (!regSpecs.length) {
      var b = { type: hit.name, content: liftInlines(node, inv, []) };
      var attrs = liftAttrs(node, hit.spec);
      if (attrs) b.attrs = attrs;
      return b;
    }
    // Collect candidate host elements (direct element children).
    var hosts = [];
    eachChild(node, function (c) { if (c.nodeType === 1) hosts.push(c); });
    var regions = {};
    regSpecs.forEach(function (rs, i) {
      // Prefer a host tagged data-st-region=name; else fall back to position.
      var host = hosts.filter(function (h) {
        return h.getAttribute && h.getAttribute('data-st-region') === rs.name;
      })[0] || hosts[i];
      var content;
      if (rs.kind === 'block') {
        content = [];
        if (host) eachChild(host, function (c) {
          if (c.nodeType === 1) {
            var t = c.tagName.toLowerCase();
            if (RAWTEXT_DROP[t]) return;
            var ch = inv.blockByTag[t];
            if (ch) content.push(liftBlock(c, ch, inv));
          }
        });
      } else {
        content = host ? liftInlines(host, inv, []) : [];
      }
      regions[rs.name] = { kind: rs.kind || 'inline', content: content };
    });
    var rb = { type: hit.name, regions: regions };
    var rattrs = liftAttrs(node, hit.spec);
    if (rattrs) rb.attrs = rattrs;
    return rb;
  }

  function firstBlockName(inv) {
    var keys = Object.keys(inv.blockByTag);
    return keys.length ? inv.blockByTag[keys[0]].name : null;
  }

  // Lift an element's descendants into a flat list of text inline nodes, carrying
  // the marks accumulated from enclosing mark-tags. A mark-tag wraps its children
  // in that mark; an unknown inline tag is unwrapped (children kept, no mark).
  function liftInlines(el, inv, marks) {
    var out = [];
    eachChild(el, function (node) {
      if (node.nodeType === 3) {
        if (node.textContent) out.push(E.text(node.textContent, marks));
      } else if (node.nodeType === 1) {
        var tag = node.tagName.toLowerCase();
        if (RAWTEXT_DROP[tag]) return; // drop rawtext content in inline position too
        var hit = inv.markByTag[tag];
        if (hit) {
          var attrs = liftAttrs(node, hit.spec);
          var nextMarks = marks.concat([{ type: hit.name, attrs: attrs || undefined }]);
          out = out.concat(liftInlines(node, inv, nextMarks));
        } else {
          // Unknown inline tag (or a block tag nested in inline position): unwrap.
          out = out.concat(liftInlines(node, inv, marks));
        }
      }
    });
    return out;
  }

  // Map an element's attributes back to the node's schema attrs, keeping only the
  // declared attr names. (Dangerous-URL rejection is the server's job, FEAT-099;
  // the lift just recovers structure.)
  function liftAttrs(el, spec) {
    if (!spec || !spec.attrs) return null;
    var names = Object.keys(spec.attrs);
    if (!names.length) return null;
    var out = {};
    var any = false;
    names.forEach(function (n) {
      if (el.hasAttribute && el.hasAttribute(n)) { out[n] = el.getAttribute(n); any = true; }
    });
    return any ? out : null;
  }

  function eachChild(parent, fn) {
    var kids = parent.childNodes;
    for (var i = 0; i < kids.length; i++) fn(kids[i]);
  }

})(typeof globalThis !== 'undefined' ? globalThis : this);
