// A GraphQL feed, declared. See README.md for the walkthrough.
//
// NB this demo ships as `.st` rather than the literate `.st.md` it was drafted
// as, because every literate document currently fails to compile (BUG-338 —
// `@doc` resolves to a nonexistent primitive, breaking even the shipped
// starter-pack tutorial). When that lands, this becomes one `index.st.md` and
// the README's prose moves back inline where it belongs.
@import "stdlib"

// --- 1 · The query is a data source -----------------------------------------
//
// `@data graphql` gives four bindings whose names you already know if you have
// used `@data fetch`: $repos, $repos_loading, $repos_error, $repos_refetch.
// It is an ARM of the data-kind family, not a new concept to learn.
@data graphql $repos : "https://api.github.com/graphql", query: "{ viewer { repositories(first: 5) { nodes { name } } } }";

// The same family, two more arms. Same head, same binding shape.
@data inline $items : ["ready", "set", "go"];

<section class="feed">
  <h2>Repositories</h2>
  <ul class="repo-list"></ul>
</section>

// --- 2 · Rendering ----------------------------------------------------------
//
// Driven by $items so the page renders with no network call or token. Point
// @each at $repos and the same markup renders live data — nothing else changes.
// That substitutability IS the family working.
.repo-list {
  @each($items as $item) {
    <li class="repo">`$item`</li>
  }
}

// --- 3 · Style and motion are declared too ----------------------------------
//
// No custom JavaScript anywhere: animation is a declaration, not an imperative
// timeline you start and stop.
.feed {
  max-width: 34rem;
  margin: 3rem auto;
  padding: 1.5rem 1.75rem;
  border-radius: 14px;
  background: #10131a;
  border: 1px solid #1e2430;
  font-family: ui-sans-serif, system-ui, sans-serif;
  color: #e6edf3;
}

.feed h2 {
  margin: 0 0 1rem;
  font-size: 1.05rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #7dd3fc;
}

.repo {
  padding: 0.7rem 0.9rem;
  margin-bottom: 0.5rem;
  border-radius: 9px;
  background: #161b25;
  border-left: 2px solid #5eead4;
  list-style: none;

  // The driver names WHAT drives progress; the body names what changes. No
  // observer to construct, no cleanup to remember, no ratio to thread through
  // a callback.
  @on &.visible(420ms) {
    opacity: 0 -> 1;
  }
}
