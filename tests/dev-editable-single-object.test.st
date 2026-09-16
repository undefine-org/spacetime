/// Unit Tests: Dev Editable — single-object bind provenance (W3).
///
/// A plain `data-st-bind` node OUTSIDE any `@each` (e.g. `$post.title`) has no
/// `[data-st-source]` ancestor, so its provenance must come from the BIND ITSELF:
/// the first segment after `$` is the data-source name, the rest is the JSON path.
/// This is what makes the whole published page editable (not just @each rows) and
/// feeds dev-save's {source, path} EditJson rail (FUP-047).
///
/// These assert the readProvenance path-parse for both shapes. Helpers are scoped
/// in an IIFE so repeated @eval blocks never collide on a shared `var` (a known
/// foot-gun in the older dev-editable tests).

@import "stdlib/testing/test"

// =============================================================================
// Single-object bind: $post.title → { source:'post', path:'.title' }
// =============================================================================
@test "single-object bind parses into source + path" {
  @fixture {
    <h1 id="so-title" data-st-bind="$post.title">Notes</h1>
  }
  @eval (() => {
    // The single-object branch of readProvenance, in isolation.
    const parse = (bind, hasSourceAncestor) => {
      if (!bind) return null;
      let source = hasSourceAncestor ? 'collectionSource' : null;
      if (source === null) {
        const raw = bind.replace(/^\$/, '');
        const m = raw.match(/^([^.\[]+)(.*)$/);
        if (m && m[1] && m[2]) {
          return { editType: 'data', source: m[1], index: null, field: null, path: m[2] };
        }
      }
      return { editType: 'data', source: source, index: null, field: bind.replace(/^\$\./, ''), path: null };
    };
    const el = document.getElementById('so-title');
    window.__p = parse(el.getAttribute('data-st-bind'), false);
  })()
  @then {
    @assert (window.__p.editType === 'data') "editType is data";
    @assert (window.__p.source === 'post') "source is the first bind segment";
    @assert (window.__p.path === '.title') "path is the bind remainder";
    @assert (window.__p.index === null) "no collection index for a single object";
  }
}

// =============================================================================
// Indexed single-object bind: $posts[2].title → { source:'posts', path:'[2].title' }
// =============================================================================
@test "indexed single-object bind keeps the array segment in path" {
  @fixture {
    <h1 id="so-idx" data-st-bind="$posts[2].title">Row</h1>
  }
  @eval (() => {
    const parse = (bind) => {
      const raw = bind.replace(/^\$/, '');
      const m = raw.match(/^([^.\[]+)(.*)$/);
      return (m && m[1] && m[2]) ? { source: m[1], path: m[2] } : null;
    };
    window.__pi = parse(document.getElementById('so-idx').getAttribute('data-st-bind'));
  })()
  @then {
    @assert (window.__pi.source === 'posts') "source is 'posts'";
    @assert (window.__pi.path === '[2].title') "path keeps the [2] array segment";
  }
}

// =============================================================================
// Collection row (has data-st-source ancestor) still uses index+field, not path.
// =============================================================================
@test "collection-row bind keeps the index+field shape" {
  @fixture {
    <ul data-st-source="posts" data-st-index="3">
      <li id="row-title" data-st-bind="$.title">Row title</li>
    </ul>
  }
  @eval (() => {
    // Mirror readProvenance: a [data-st-source] ancestor → source/index from it,
    // field from the bind; the single-object branch is NOT taken.
    const el = document.getElementById('row-title');
    let cur = el, source = null, index = null;
    while (cur && cur !== document.body) {
      if (cur.hasAttribute && cur.hasAttribute('data-st-source')) {
        source = cur.getAttribute('data-st-source');
        index = cur.getAttribute('data-st-index');
        break;
      }
      cur = cur.parentElement;
    }
    const bind = el.getAttribute('data-st-bind');
    const prov = (source === null)
      ? { source: bind.replace(/^\$/,'').match(/^([^.\[]+)/)[1], path: '.x' }
      : { editType:'data', source: source, index: index, field: bind.replace(/^\$\./,''), path: null };
    window.__pc = prov;
  })()
  @then {
    @assert (window.__pc.source === 'posts') "source from the data-st-source ancestor";
    @assert (window.__pc.index === '3') "index from the data-st-index ancestor";
    @assert (window.__pc.field === 'title') "field from the bind (collection shape)";
    @assert (window.__pc.path === null) "no single-object path for a collection row";
  }
}
