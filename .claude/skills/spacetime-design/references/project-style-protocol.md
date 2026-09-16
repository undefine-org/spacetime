# Project-style protocol

How to extract a project's brand without inventing one.

## When to run this

Before authoring ANY new section, screen, or component for a project. Always.
The cost of inventing a color and then having to refactor when the user
notices is much higher than running this checklist.

## Asset tier hierarchy

The protocol's order matters. Brand recognizability is *not* a function of
the color tokens alone. The hierarchy, ranked by how much identity each tier
carries:

```
1. Logo                — non-substitutable identifier
2. Product / hero imagery — what the brand SHOWS
3. UI screenshots      — for SaaS / digital products
4. Color tokens        — repeatable signal
5. Font tokens         — voice
6. Vibe / temperature  — descriptor (warm, quiet, loud, ...)
```

Tokens at tier 4 without anything at tier 1-3 produce a structurally clean
file that still reads as off-brand the moment it's rendered. Run Step 0
before promoting tokens.

## Step 0 · Asset-tier inventory

Before any token work:

```bash
ls projects/<name>/                                  # repo layout
ls projects/<name>/assets/ projects/<name>/images/   # asset directories
find projects/<name>/ -iname "logo*"                 # logos
find projects/<name>/ -iname "hero*" -o -iname "og*" # hero / social card
find projects/<name>/ -iname "screenshot*"           # product UI
```

Decision rules:

- **Logo missing.** Halt. Ask the user. Do NOT proceed to token freeze with
  a placeholder logo; placeholder logos quietly become real ones in shipped
  builds.
- **Product / hero imagery missing AND the site is content-driven** (a
  landing page, marketing site, portfolio). Ask the user before proceeding.
- **UI screenshot missing AND the site is a digital product** (SaaS demo,
  documentation site). Ask the user.
- **Vibe descriptor missing.** Ask. ("Quiet / professional / warm /
  playful / loud — pick one or two adjectives.")

This step ASKS the user; it never fetches assets from the web.

## The 5-step token procedure (after Step 0 inventory)

### 1. Read authoritative declarations

```bash
cat projects/<name>/AGENTS.md          # project-level rules
cat projects/<name>/README.md          # human-facing summary
read projects/<name>/docs/             # any internal style guide
```

Concrete example. `projects/undefine.org/AGENTS.md` declares `Indigo: #3B3A9E`.
You do **not** pick a "slightly nicer indigo" — you use that exact value, lift
it into a token, and reference the token everywhere.

### 2. Read existing `.st` and `.css` for tokens

```bash
ls projects/<name>/*.st projects/<name>/*.css 2>/dev/null
```

Look for:

- `:root { --x: ... }` declarations
- `var(--x)` usages
- `@apply` / `@theme` (when used)

### 3. Inventory current var usage

```bash
grep -rn "var(--" projects/<name>/
```

Output: the set of tokens already defined. Use these by name in new code.

### 4. Inventory unbound literals (debt)

```bash
grep -rEn "#[0-9a-fA-F]{3,8}\b" projects/<name>/
```

Each unbound literal is either:

- A token that should exist but doesn't (promote → add to tokens, reference
  via var), or
- A genuinely one-off value that the brand permits (rare; document why).

### 5. Freeze tokens before authoring

If the project lacks a tokens file, create one:

```st
// projects/<name>/tokens.st
.brand {
  --brand-fg: #...;
  --brand-bg: #...;
  --brand-accent: #...;
  --brand-radius: ...;
}
```

Hosted on a class so cascading + retheming work; or attach to `body { ... }`
which is a recognized scope by the parser.

Reference it from the project's `index.st`:

```st
@import "./tokens.st";
```

After this, NEW code MUST use `var(--brand-*)` and never inline hex.

## Step 6 · Compose brand-spec.md

After Step 0 inventory + Step 5 token freeze, record everything in
`projects/<name>/brand-spec.md`. This is the single source of truth that
`cargo run -- doctor` reads to emit `brand-asset-missing` warnings when
referenced files are absent.

```markdown
# Brand spec — <project-name>

## Logo
- Primary: assets/logo.svg
- Inverted (dark backgrounds): assets/logo-inverted.svg
- Favicon: assets/favicon.ico

## Hero imagery
- Above-the-fold: assets/hero/main.jpg
- Social card: assets/og-image.png

## UI screenshots
- (n/a — marketing site)

## Tokens
See [tokens.st](tokens.st). Canonical entries:
- --brand-fg: #...
- --brand-bg: #...
- --brand-accent: #...

## Forbidden colors
List of palettes that look "branded" but are not — drawn from common
corpus defaults that conflict with this brand:
- Purple gradients
- GitHub dark mode

## Vibe descriptors
quiet · considered · warm
```

`cargo run -- doctor projects/<name>/` reads this file and emits a
`brand-asset-missing` warn for any path under `## Logo`, `## Hero imagery`,
or `## UI screenshots` that doesn't exist on disk. The check is opt-in:
projects without `brand-spec.md` see no `brand-asset-missing` findings.

## When the brand isn't documented

If Step 0 inventory comes back empty, do not proceed to token freeze. Ask
the user. If the user authorizes a placeholder run, mark every borrowed or
inferred decision in `brand-spec.md` explicitly (e.g. "Logo: TBD —
placeholder") so it can be promoted later. Question 2 dovetails into
[fallback-advisor.md](fallback-advisor.md) but ONLY after Step 0 inventory
is signed off.

## Verification

After authoring:

```bash
grep -rEn "#[0-9a-fA-F]{3,8}\b" projects/<name>/<your-new-files>
# expect: zero hits, or only inside :root/--*: declarations
```

Equivalently, `Spacetime.review()` should report `brand` score = 10 on the
project's routes, and `cargo run -- doctor` should pass with no
`brand-asset-missing` findings.

## brand-spec.md template

A compile-able template lives at the bottom of this file (above). For a
worked example, see `projects/undefine.org/brand-spec.md` (when authored)
which fills the template with the project's actual indigo `#3B3A9E`
declarations, hero imagery paths, and the "quiet · considered" vibe.
