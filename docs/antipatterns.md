# Antipatterns

## Biggest: Usage of Javascript anywhere outside of %primitive and @test

VERY VERY BAD:

onclick="window.__st_setLocale('en')"

GOOD:

whatever binding you do through Spacetime

## Visual anti-patterns

These rules cover *how the page looks once it parses*. The skill-side digest
with examples is at
`.claude/skills/spacetime-design/references/anti-patterns.md`.

### 9. Purple gradient hero without brand justification

`linear-gradient(135deg, #7c3aed, #2563eb)` is the SaaS-AI corpus default.
Use `var(--brand-bg)`. **Exception**: project AGENTS.md / tokens.st declares
purple as the brand signature (see `stdlib/showcases/hero/maximal.st` for an
authorized gradient anchored in the experimental school).

### 10. Emoji-as-icon-system

`content: "🚀"` per bullet signals "no design effort". Use real SVG icons
(Lucide / Phosphor / Heroicons) or omit icons. **Exception**: brand voice IS
casual (Notion, Discord) or audience expects playfulness.

### 11. Generic display fonts (Inter / Roboto / Arial as headlines)

Inter / Roboto / system-default at 4rem is the AI agnostic-system default.
Type carries 50%+ of brand recall. Use `var(--brand-display)`.
**Exception**: tokens.st / brand-spec.md explicitly maps `--brand-display` to
Inter / Roboto / Arial.

### 12. AI-drawn SVG imagery (people, scenes, products)

Drawn faces have wrong proportions; drawn scenes have impossible lighting.
Use real photography (project `assets/` per project-style-protocol Step 0)
or honest placeholders. **Exception**: brand IS illustrated and ships its
own illustration system.

### 13. GitHub-dark-mode aesthetic

`#0d1117` + `#58a6ff` + `#39d353` is GitHub's signature; using it elsewhere
reads as a GitHub clone. Use brand tokens. **Exception**: developer tool
whose brand-spec.md explicitly authorizes the palette.

### 14. Rounded card + colored left-border accent

`border-radius: 8px; border-left: 4px solid <accent>` is the 2020-2024
Material/Tailwind tic; corpus-saturated; the cue stops carrying signal.
**Exception**: design system explicitly declares the pattern.
