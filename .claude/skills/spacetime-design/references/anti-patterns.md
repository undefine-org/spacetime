# Spacetime anti-patterns

> Canonical short-form list. The repo-canonical version is `docs/antipatterns.md`;
> this file is the skill-side digest with examples. Keep them in sync — if you
> add an entry here, add or update it there too.

## 1. JavaScript outside `%primitive` and `@test`

**Bad**

```html
<button onclick="window.__st_setLocale('en')">EN</button>
<button onclick="alert('hi')">Click</button>
<script>
  document.querySelector('.cta').addEventListener('click', go);
</script>
```

**Good**

```st
.cta {
  @on click {
    $locale <- "en";
  }
}
```

Inline event handlers, hand-rolled `<script>` tags, and ad-hoc DOM event wiring
are all the same defect: they bypass the metasystem. Express behavior through
macros (`@on`, `@bind`, `@fade-in`, ...). Detected by `Spacetime.review()`
under the `idiomatic` dimension.

## 2. Hex literals outside CSS variables

**Bad**

```css
.hero { color: #2563eb; background: #f8fafc; }
```

**Good**

```css
.hero {
  --hero-fg: #2563eb;
  --hero-bg: #f8fafc;
  color: var(--hero-fg);
  background: var(--hero-bg);
}
```

Or, better, lift the tokens into the project's `tokens.st` and import them.
Detected by `Spacetime.review()` under the `brand` dimension.

## 3. Rust-based parsers / fallbacks for stdlib constructs

**Bad**

> "The stdlib pattern for `%capture_type` isn't loading, so I'll add a hardcoded
> Rust shim in `src/parser/...` that handles it."

**Good**

> "The stdlib pattern isn't loading. Diagnose `cargo run -- inspect --layer registry`,
> fix the loading mechanism, never bypass it."

The metasystem is self-describing; reimplementing stdlib in Rust forks the
language. Recorded as a hard rule in `AGENTS.md`.

## 4. Hardcoded data not flowing through `@data`

**Bad**

```st
@template &card($title) {
  <div class="card">
    <h3>$title</h3>
    <p>Price: $19.99</p>           // hardcoded
    <p>SKU: ABC-123</p>            // hardcoded
  </div>
}
```

**Good**

```st
@data products: Product[] { src: "/data/products.json"; }

@each($products as $p) {
  &card($p);
}
```

Hardcoded values turn templates into copy-paste parents.

## 5. JSON.stringify of large objects in `%emit js`

Performance trap. Seen in dataiku hangs (memory note). Coalesce/batch instead;
see `memory://root/MEMORY.md` for the canonical fix patterns.

## 6. `@each` nested inside `@template` body

Parser limitation. Lift the iteration outside the template; pass each item in
as a parameter.

## 7. HTML comments and Unicode/emoji in `.st` files

Parser limitation. Use `///` doc comments and ASCII or escaped Unicode (`\u2192`)
in JS string literals.

## 8. Inventing brand colors

If `projects/<name>/AGENTS.md` says "Indigo: #3B3A9E", you do NOT pick "a slightly
nicer indigo." Read the project; freeze the tokens; reference them. See
[project-style-protocol.md](project-style-protocol.md).

## Visual anti-patterns

Once your code parses, the next failure mode is *visual slop*: layouts that
compile + animate + token-resolve cleanly yet scream "AI default." Each rule
below names a corpus-mean signature, the brand-recognizability cost of
shipping it, and the explicit exception under which it IS the right call.
These are about brand identity, not taste; an AI default is the *opposite*
of a brand voice.

### 9. Purple gradient hero without brand justification

**Bad**

```st
&hero {
    background: linear-gradient(135deg, #7c3aed 0%, #2563eb 100%);
    color: white;
}
```

**Good**

```st
&hero {
    background: var(--brand-bg);
    color: var(--brand-fg);
}
```

Rationale: violet → indigo gradient is the SaaS-AI corpus default. Shipping it dilutes brand recognizability the same way generic Lorem ipsum body copy would. Exception: project AGENTS.md or tokens.st declares purple as the brand signature — e.g. the experimental-school anchor `stdlib/showcases/hero/maximal.st` (FEAT-030) ships an authorized gradient because it represents that school deliberately.

### 10. Emoji-as-icon-system

**Bad**

```st
.feature-card::before { content: "🚀 "; }
.feature-card--secure::before { content: "🔒 "; }
```

**Good**

```st
/* Real SVG icons (Lucide / Phosphor / Heroicons) imported via @data,
   or omit icons and let typography do the work. */
.feature-card__icon { background-image: var(--icon-rocket); }
```

Rationale: emoji glyphs render differently across OS font stacks; using them as primary iconography signals "no design effort." Exception: brand voice IS casual (Notion, Discord), or the audience explicitly expects playfulness.

### 11. Generic display fonts (Inter / Roboto / Arial as headlines)

**Bad**

```st
h1, h2 {
    font-family: "Inter", sans-serif;
    font-size: 4rem;
}
```

**Good**

```st
h1, h2 {
    font-family: var(--brand-display);
    font-size: var(--font-display-xl);
}
```

Rationale: Inter / Roboto / system-default at 4rem is the AI "agnostic-system" default. Type carries 50%+ of brand recall on text-driven pages. Exception: the project's `tokens.st` (or `brand-spec.md`) explicitly maps `--brand-display` to Inter / Roboto / Arial.

### 12. AI-drawn SVG imagery (people, scenes, products)

**Bad**

```st
.testimonial__avatar {
    background: url("data:image/svg+xml;utf8,<svg>...drawn-face...</svg>");
}
```

**Good**

```st
.testimonial__avatar {
    background: url("/assets/people/jordan-h.jpg");
    background-size: cover;
}
```

Rationale: AI-drawn faces have wrong proportions; AI-drawn scenes have impossible lighting. Use real photography (project `assets/` per [project-style-protocol.md](project-style-protocol.md) Step 0) or honest placeholders that survive review. Exception: brand IS illustrated and ships its own illustration system (Mailchimp, Stripe Press).

### 13. GitHub-dark-mode aesthetic

**Bad**

```st
:root {
    --bg: #0d1117;
    --accent: #58a6ff;
    --neon: #39d353;
}
```

**Good**

```st
:root {
    --bg: var(--brand-surface);
    --accent: var(--brand-accent);
}
```

Rationale: `#0d1117` + `#58a6ff` + `#39d353` is GitHub's signature; using it elsewhere makes the product read as a GitHub clone. Exception: developer tool whose `brand-spec.md` explicitly authorizes the palette (and even then, differentiate the accent).

### 14. Rounded card + colored left-border accent

**Bad**

```st
.alert {
    border-radius: 8px;
    border-left: 4px solid #2563eb;
    background: #f1f5f9;
    padding: 16px;
}
```

**Good**

```st
.alert {
    border: var(--brand-border);
    background: var(--brand-surface-quiet);
    padding: var(--space-3);
}
```

Rationale: 2020-2024 Material/Tailwind tic; corpus-saturated. The visual cue stops carrying signal. Exception: design system explicitly declares the pattern as part of its alert vocabulary.

## See also

- `docs/antipatterns.md` — canonical repo doc.
- `Spacetime.review()` — runtime detector for items 1, 2, and adjacent (and `hierarchy` + `detail` per FEAT-032).
- `references/project-style-protocol.md` — Step 0 asset-tier inventory feeds visual rule 12.
- `references/design-styles.md` — 5-school taxonomy that names when a corpus default IS the deliberate direction (experimental anchors gradients; eastern-philosophy anchors warm minimalism).