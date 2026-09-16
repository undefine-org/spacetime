@version 2026-06-09;

/// A page carrying every shape of comment the feature supports, so the pill,
/// `check`, and the MCP tools all have something real to show.
///
/// Run it: `cargo run -- serve tests/fixtures/comments-demo/`
/// Read it: `cargo run -- check tests/fixtures/comments-demo/`

//@: a bare marker is a note — an observation, nothing owed by anyone

//@todo(owner: ada): replace the placeholder photography before launch

//@bug(severity: high): the nav overlaps the CTA below 380px
//@> reproduced on a 375px viewport in both Safari and Chrome
//@> the CTA is still tappable, so this is ugly rather than broken

//@design: the hero grid is 7/5 rather than 6/6 on purpose
//@> the eye should reach the price before the photograph, and an even
//@> split makes the two compete

//@question(asked-of: brand): is "from $40" acceptable, or must it be exact?

//@agent-task(acceptance: "cargo run -- check tests/fixtures/comments-demo/"): tidy the CSS ordering

//@brand-review(reviewer: ada, due: "2026-08-15"): the headline claim needs sign-off

<main class="page">
  <h1 class="headline">The best coffee, from $40</h1>
  // NB this line RENDERS — inside HTML markup `//` is literal page text,
  // which is exactly why the comment scanner leaves markup blocks alone.
  <p class="lede">Roasted the day it ships.</p>
  <a class="cta" href="/order">Order</a>
</main>

.page { display: grid; gap: 16px; padding: 32px; }
.headline { font-size: 40px; margin: 0; }
.lede { color: #6b7280; margin: 0; }
.cta { justify-self: start; padding: 10px 18px; border-radius: 999px; background: #111; color: #fff; }
