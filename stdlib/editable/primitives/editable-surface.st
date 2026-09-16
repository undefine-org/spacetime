/// Editable Surface Primitive (PLAN-031 / FEAT-098)
///
/// The live host for a structured rich-text field. Wraps a `contenteditable`
/// element around the pure AST kernel (`ST.Editable`, runtime editable.js):
///   - holds the document AST (`$doc`), never reads DOM state back as truth;
///   - intercepts `beforeinput`, translates each intent into an AST OP, applies
///     it, then RE-PROJECTS the doc into the host (DOM is a render target only);
///   - maps the browser Selection to integer positions (`$sel = {from,to}`) and
///     restores it after re-projection;
///   - exposes editor state as signals so toolbars/placeholders/counters are
///     ordinary bindings: `$doc $sel $activeMarks $blockType $empty`.
///
/// Commands ride the signal-method rail: `$doc.apply("bold")`,
/// `$doc.setBlock("h2")`, `$doc.apply("link", { href })` — a custom toolbar is
/// plain `@on click { $doc.apply("bold"); }`.
///
/// @param &host    - the contenteditable host element
/// @param bind     - the persisted value path (AST object) this surface reads/writes
/// @param marks    - array of allowed mark names (from the @editable marks: list)
/// @param blocks   - array of allowed block names (from the @editable blocks: list)
/// @param schema   - the editable schema descriptor { marks, blocks } (from editable-schema.json)
/// @param gate     - editing gate: null = always editable (forms/admin); 'edit-mode'
///                   = contenteditable mirrors html[data-st-edit-mode] (published pages,
///                   the @richtext default — reads as final HTML, edits in place on toggle)
///
/// @yields $doc        - the document AST (+ command methods apply/setBlock/split)
/// @yields $sel        - the selection { from, to } in integer positions
/// @yields $activeMarks- mark types active across the current selection
/// @yields $blockType  - block type at the selection head
/// @yields $empty      - whether the document is effectively empty

%primitive editable-surface(
  &host,
  bind: string? = null,
  marks: array = [],
  blocks: array = [],
  schema: any = null,
  gate: string? = null
) {
  %emit js {
    (function () {
      var host = %&host;
      if (!host || typeof ST === 'undefined' || !ST.Editable) {
        console.warn('[ST] editable-surface: host or ST.Editable missing');
        return;
      }
      var E = ST.Editable;
      var markOrder = %marks || [];
      var blockNames = %blocks || [];

      // --- schema: prefer the explicit descriptor, else the global registry ---
      var schema = %schema;
      if (schema && (schema.marks || schema.blocks)) {
        E.setSchema(schema);
      } else if (ST.editable && (ST.editable.marks || ST.editable.blocks)) {
        E.setSchema({ marks: ST.editable.marks || {}, blocks: ST.editable.blocks || {} });
      }

      // --- initial document: from the bound value (AST) or an empty doc ---------
      // bind can arrive two ways:
      //   (a) an inlined AST OBJECT (admin drawer: `bind:`$f.value`` backtick hole);
      //   (b) a `$`-PATH STRING (a page: `bind: $post.body`) that names a value in
      //       the reactive local state. The admin path inlines the object; a page
      //       names a data field, so we resolve the path against SpacetimeLocal and
      //       re-seed when that data loads/changes (the field lives in @data, the
      //       editable-everything default). Returns null when unresolved.
      var bindPath = %bind;
      function resolveBoundDoc() {
        if (bindPath == null) return null;
        if (typeof bindPath === 'object') return bindPath; // (a) inlined AST
        if (typeof bindPath !== 'string' || bindPath.charAt(0) !== '$') return null;
        // (b) `$`-PATH STRING. Resolve the head signal lexically from `host`'s scope (a
        //     TEMPLATE INSTANCE param the factory seeded, or an outer scope, or global
        //     @data via SpacetimeLocal) and traverse the dotted tail — ST.resolvePath is
        //     the single scope-resolution primitive (no per-primitive ancestor walk).
        var v = (typeof ST !== 'undefined' && ST.resolvePath)
          ? ST.resolvePath(host, bindPath.slice(1))
          : undefined;
        return (v != null && typeof v === 'object') ? v : null;
      }
      var initial = null;
      var bound0 = resolveBoundDoc();
      if (bound0) { initial = E.fromJSON(bound0); }
      var doc = initial || E.emptyDoc();
      var sel = { from: 0, to: 0 };
      var dirty = false; // set true by a genuine user edit; gates on-page persist

      host.setAttribute('data-st-editable', '');

      // --- edit-mode gating (PLAN-031 / editable-everything) -------------------
      // gate=null  → always editable (admin drawer, forms, tests; unchanged).
      // gate='edit-mode' → contenteditable MIRRORS html[data-st-edit-mode], so a
      //   published page renders as clean read-only HTML (the projection IS the
      //   final markup) and upgrades to an editor in place when edit-mode toggles.
      //   The doc/projection is identical in both states; only the attribute flips.
      var gate = %gate;
      function applyGate() {
        if (gate === 'edit-mode') {
          if (document.documentElement.hasAttribute('data-st-edit-mode')) {
            host.setAttribute('contenteditable', 'true');
          } else {
            host.removeAttribute('contenteditable');
          }
        } else {
          host.setAttribute('contenteditable', 'true');
        }
      }
      applyGate();
      if (gate === 'edit-mode' && typeof MutationObserver === 'function') {
        var gateObs = new MutationObserver(applyGate);
        gateObs.observe(document.documentElement, { attributes: true, attributeFilter: ['data-st-edit-mode'] });
      }

      // --- on-page persistence (FUP-047) ---------------------------------------
      // A PUBLISHED @richtext surface (gate='edit-mode') bound to a `$`-PATH persists
      // its doc AST back to the source file when edited — reusing the dev-save rail
      // (EditJson → server validates the doc against the editable schema → disk). The
      // ADMIN drawer (gate=null) keeps its own adFieldInput path; this hook is the
      // PAGE analogue and only arms for the page case. The bind names a data field:
      //   $post.body     → source 'post',  jsonPath '.body'
      //   $posts[2].body → source 'posts', jsonPath '[2].body'
      // so $x.f and $x[i].f flow through one parser (the source is the FIRST segment,
      // the json path is the remainder, leading '.'/'[' preserved).
      var persistTarget = null; // { source, path } or null when not a persistable path
      (function () {
        if (gate !== 'edit-mode') return;
        if (typeof bindPath !== 'string' || bindPath.charAt(0) !== '$') return;
        var rest = bindPath.slice(1);
        // First segment = data-source name (up to the first '.' or '[').
        var m = rest.match(/^([^.\[]+)(.*)$/);
        if (!m || !m[1]) return;
        var jsonPath = m[2] || '';        // '', '.body', '[2].body', …
        if (jsonPath === '') return;       // binding the WHOLE source is not a field edit
        persistTarget = { source: m[1], path: jsonPath };
      })();
      var persistTimer = null;
      function persistDoc() {
        if (!persistTarget) return;
        if (typeof window === 'undefined' || !window.__stDevSave) return;
        // Only persist while edit-mode is on (a programmatic re-seed must not write).
        if (!document.documentElement.hasAttribute('data-st-edit-mode')) return;
        if (persistTimer) clearTimeout(persistTimer);
        persistTimer = setTimeout(function () {
          persistTimer = null;
          try {
            window.__stDevSave.save({
              editType: 'data',
              source: persistTarget.source,
              path: persistTarget.path,
              value: E.toJSON(doc),
              element: host
            });
          } catch (e) { /* persist is best-effort; the model stays authoritative */ }
        }, 400);
      }

      // --- projection: render the doc into the host, then restore the caret ---
      function reproject(restorePos) {
        var frag = E.project(doc, document, markOrder);
        host.innerHTML = '';
        host.appendChild(frag);
        if (restorePos != null && typeof window !== 'undefined' && window.getSelection) {
          var dp = E.domFromPos(host, doc, restorePos);
          if (dp) {
            try {
              var range = document.createRange();
              range.setStart(dp.node, dp.offset);
              range.collapse(true);
              var s = window.getSelection();
              s.removeAllRanges();
              s.addRange(range);
            } catch (e) { /* selection restore best-effort */ }
          }
        }
        publish();
      }

      // --- read the current Selection as integer positions ---
      function readSelection() {
        if (typeof window === 'undefined' || !window.getSelection) return sel;
        var s = window.getSelection();
        if (!s || s.rangeCount === 0) return sel;
        var r = s.getRangeAt(0);
        var from = E.posFromDOM(host, doc, r.startContainer, r.startOffset);
        var to = E.posFromDOM(host, doc, r.endContainer, r.endOffset);
        if (from == null) from = sel.from;
        if (to == null) to = from;
        return { from: Math.min(from, to), to: Math.max(from, to) };
      }

      // --- publish editor state into signals ---
      function publish() {
        ST.set(host, 'doc', withCommands(doc));
        ST.set(host, 'sel', sel);
        ST.set(host, 'activeMarks', E.activeMarks(doc, sel.from, sel.to));
        ST.set(host, 'blockType', E.blockTypeAt(doc, sel.from));
        ST.set(host, 'empty', E.isEmpty(doc));
        // Persist hook: store the current document AST on the host element so the
        // admin field-input path (ST.adFieldInput) reads it and sends EditJson with
        // the AST object (server validates against the editable schema → disk,
        // FEAT-099). A host that is an admin richtext field is tagged `data-widget=
        // richtext`; the drawer's @on input fires adFieldInput($, $activeIndex).
        host.__stDoc = E.toJSON(doc);
        host.setAttribute('data-st-doc', '1');
        // Fire a synthetic input event so the host's bound @on input persists the
        // change through the normal field path (no special-casing in the surface).
        if (typeof Event === 'function') {
          try { host.dispatchEvent(new Event('input', { bubbles: true })); } catch (e) {}
        }
        // On-page persist (FUP-047): page @richtext surfaces save to disk through the
        // dev-save rail. Only fires for genuine USER edits (the `dirty` flag, set by
        // the beforeinput funnel + command methods) — never on first paint or re-seed,
        // and only while edit-mode is on (persistDoc re-checks the attribute).
        if (dirty) { dirty = false; persistDoc(); }
      }

      // --- command surface: $doc.apply(...) / $doc.setBlock(...) / $doc.split() ---
      // The yielded $doc is the plain AST with non-enumerable command methods, so
      // `$doc.apply("bold")` works while JSON.stringify($doc) is still the AST.
      function withCommands(d) {
        var wrapped = E.toJSON(d);
        Object.defineProperty(wrapped, 'apply', { value: function (markType, attrs) {
          dirty = true;
          doc = E.toggleMark(doc, sel.from, sel.to, markType, attrs);
          reproject(sel.to);
        }, enumerable: false });
        Object.defineProperty(wrapped, 'setBlock', { value: function (blockType, attrs) {
          dirty = true;
          doc = E.setBlockType(doc, sel.from, blockType, attrs);
          reproject(sel.from);
        }, enumerable: false });
        Object.defineProperty(wrapped, 'split', { value: function () {
          dirty = true;
          var caret = E.splitCaret(doc, sel.from);
          doc = E.splitBlock(doc, sel.from);
          reproject(caret);
        }, enumerable: false });
        return wrapped;
      }

      // --- beforeinput: the single funnel; every intent → an AST op → re-project ---
      host.addEventListener('beforeinput', function (ev) {
        ev.preventDefault(); // DOM never mutates directly; we own the model.
        dirty = true; // a genuine user edit — arm on-page persist (FUP-047)
        sel = readSelection();
        var type = ev.inputType;
        var restore = sel.from;
        if (type === 'insertText' && ev.data != null) {
          doc = E.insertText(doc, sel.from, ev.data);
          restore = sel.from + ev.data.length;
        } else if (type === 'insertParagraph') {
          restore = E.splitCaret(doc, sel.from); // region-correct (no flat +2 assumption)
          doc = E.splitBlock(doc, sel.from);
        } else if (type === 'deleteContentBackward') {
          if (sel.from !== sel.to) { doc = E.deleteRange(doc, sel.from, sel.to); restore = sel.from; }
          else if (sel.from > 0) { doc = E.deleteRange(doc, sel.from - 1, sel.from); restore = sel.from - 1; }
        } else if (type === 'deleteContentForward') {
          if (sel.from !== sel.to) { doc = E.deleteRange(doc, sel.from, sel.to); restore = sel.from; }
          else { doc = E.deleteRange(doc, sel.from, sel.from + 1); restore = sel.from; }
        } else if (type === 'insertReplacementText' || type === 'insertFromPaste') {
          // Paste gate (FEAT-101): if the clipboard carries HTML, LIFT it through
          // the lens backward (liftHtml) so only schema-known marks/blocks survive,
          // then merge the lifted document at the cursor. Falls back to plain text.
          var htmlData = ev.dataTransfer && ev.dataTransfer.getData && ev.dataTransfer.getData('text/html');
          if (htmlData) {
            var lifted = E.liftHtml(htmlData, document);
            // Delete the selection, then merge the lifted document at the cursor
            // (E.insertDoc joins the first lifted block inline, appends the rest).
            doc = E.deleteRange(doc, sel.from, sel.to);
            var merged = E.insertDoc(doc, sel.from, lifted);
            doc = merged.doc; restore = merged.pos;
          } else {
            var txt = ev.data || (ev.dataTransfer && ev.dataTransfer.getData && ev.dataTransfer.getData('text/plain')) || '';
            if (txt) { doc = E.deleteRange(doc, sel.from, sel.to); doc = E.insertText(doc, sel.from, txt); restore = sel.from + txt.length; }
          }
        } else {
          // Unhandled intent: ignore (model stays authoritative).
          return;
        }
        sel = { from: restore, to: restore };
        reproject(restore);
      });

      // --- selection tracking: keep $sel + active states live on caret moves ---
      if (typeof document !== 'undefined') {
        document.addEventListener('selectionchange', function () {
          if (typeof window === 'undefined' || !window.getSelection) return;
          var s = window.getSelection();
          if (!s || s.rangeCount === 0) return;
          // Only react when the selection is inside our host.
          var node = s.anchorNode;
          while (node && node !== host) node = node.parentNode;
          if (node !== host) return;
          sel = readSelection();
          ST.set(host, 'sel', sel);
          ST.set(host, 'activeMarks', E.activeMarks(doc, sel.from, sel.to));
          ST.set(host, 'blockType', E.blockTypeAt(doc, sel.from));
        });
      }

      // --- re-seed from data: if the bind is a `$`-path and the value was not yet
      //     present at init (data loads after selector-init flush), re-seed once the
      //     bound value publishes. Idempotent: only adopts an external doc while the
      //     surface is still empty, so it never clobbers in-progress edits.
      //
      //     The base signal `$root` lives in one of two scopes, each reached by the SAME
      //     scope-watch: (1) global @data → SpacetimeLocal (`local:<root>:updated`); (2) a
      //     TEMPLATE INSTANCE param the factory seeds on the instance root — whose set may
      //     race AHEAD of this selector-init. ST.watchScoped watches the name across host +
      //     ancestors AND fires immediately if already present, so it covers both the race
      //     and the already-seeded case in one call (PLAN-039).
      if (typeof bindPath === 'string' && bindPath.charAt(0) === '$'
          && typeof document !== 'undefined') {
        var rootName = bindPath.slice(1).split('.')[0];
        var reseed = function () {
          if (!E.isEmpty(doc)) return; // user has content — don't overwrite
          var bound = resolveBoundDoc();
          if (bound) { doc = E.fromJSON(bound); reproject(null); }
        };
        // (1) global @data channel (SpacetimeLocal publishes a DOM event, not a signal).
        document.addEventListener('local:' + rootName + ':updated', reseed);
        // (2) scope channel: instance param / outer signal, wherever in scope it lands.
        if (typeof ST !== 'undefined' && ST.watchScoped) {
          ST.watchScoped(host, rootName, reseed);
        }
      }

      // --- first paint ---
      reproject(null);

      %yield withCommands(doc) -> $doc;
      %yield sel -> $sel;
      %yield E.activeMarks(doc, sel.from, sel.to) -> $activeMarks;
      %yield E.blockTypeAt(doc, sel.from) -> $blockType;
      %yield E.isEmpty(doc) -> $empty;
    })();
  }

  %exports {
    $doc: any
    $sel: any
    $activeMarks: array
    $blockType: string
    $empty: bool
  }
}
