/* ============================================================================
   SSG List — first-class HTML, inline @each, and static-unroll (PLAN-023)

   A full-Spacetime page that ships REAL markup to crawlers / no-JS clients and
   hydrates to the identical DOM. No index.html, no .js — everything is Spacetime.

   Showcases the unified surface:
     1. <ul>…</ul>            first-class HTML element literals (the `<` sigil)
     2. @data inline          a STATIC source (compile-time-constant array)
     3. @each(...) { <li> }   inline-HTML list body with `…` holes
     4. @data derive          a live count derived from the source
     5. SSG unroll == hydrate  static rows are served as HTML AND hydrated
   ============================================================================ */

/* ---- Static data: a constant list, so it SSG-unrolls -------------------- */

@data inline $fruits : ["Apple", "Banana", "Cherry", "Date"];
@data derive $count number : $fruits.length;

/* ---- Page markup (served HTML) ------------------------------------------ */

<main class="page">
  <h1>Fruit Basket</h1>
  <p class="count">Items: <span class="count__n">`$count`</span></p>
  <ul class="basket"></ul>
</main>

/* ---- Reactive list: each fruit becomes a real <li> ---------------------- */

.basket {
  @each($fruits as $fruit) {
    <li class="basket__item">`$fruit`</li>
  }
}

/* ---- Styling (plain Spacetime CSS) -------------------------------------- */

.page {
  max-width: 32rem;
  margin: 4rem auto;
  font-family: system-ui, sans-serif;
}

.basket {
  list-style: none;
  padding: 0;
}

.basket__item {
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #eee;
}

.count__n {
  font-weight: 700;
}
