# Showcase metadata schema

Each `.st` file under `stdlib/showcases/<scenario>/<style>.st` MUST begin with a
doc-comment block following this layout. These *7 fields* are read as DATA by
`@data declarations $p from template` — each becomes a key on `$p.fields`, so a
page can group, filter and label patterns without parsing anything by hand.

There is no showcase CLI and no `@showcases` registry: the metadata reaches a
page through the ordinary language surface. `demos/showcase-gallery/` renders
the whole tree from it.

## Required fields

| Field        | Format                                  | Example                                                      |
|--------------|-----------------------------------------|--------------------------------------------------------------|
| scenario     | `/// scenario: <kebab-case>`            | `/// scenario: hero`                                         |
| style        | `/// style: <kebab-case>`               | `/// style: minimal`                                         |
| school       | `/// school: <kebab-case>`              | `/// school: minimalism`                                     |
| description  | `/// description: <one-line sentence>`  | `/// description: Centered headline + subhead + single CTA.` |
| dependencies | `/// dependencies: <comma list> \| none`| `/// dependencies: stdlib, ./reveal-on-scroll/minimal.st`    |
| screenshot   | `/// screenshot: <path> \| (pending)`   | `/// screenshot: stdlib/showcases/hero/minimal.png`          |
| useWhen      | `/// useWhen: <one-line guidance>`      | `/// useWhen: Marketing pages where the offer is the page.`  |

`(pending)` is allowed for `screenshot`: the rendered gallery shows the live
pattern, so a still image is optional documentation rather than the only way to
see one.

## Allowed scenarios

`hero`, `nav`, `features-grid`, `cta`, `footer`, `gallery`, `form`, `reveal-on-scroll`.

## Allowed styles

`minimal`, `editorial`, `maximal`.

## Allowed schools

From the 5-school taxonomy referenced in
`.claude/skills/spacetime-design/references/design-styles.md`:

- `information-architecture` — grid-driven, data-forward, restrained.
- `motion-poetics` — dynamic, immersive, scroll-driven.
- `minimalism` — order, white space, precision.
- `experimental` — avant-garde, generative, visual disruption.
- `eastern-philosophy` — warm, poetic, contemplative.

## Doc-comment block

The header MUST be the first non-blank lines of the file and MUST appear
exactly once. The first regular line of the file SHOULD be:

```
/// scenario: <s>
/// style: <s>
/// school: <s>
/// description: <s>
/// dependencies: <s>
/// screenshot: <s>
/// useWhen: <s>
```

Lines may appear in any order, but each key MUST appear at most once.

## Parser regex (canonical)

The CLI and registry parse with one expression per key:

```
^/// (scenario|style|school|description|dependencies|screenshot|useWhen): (.+)$
```

A file that omits a key still appears in the gallery — its label is simply
empty, which is visible on the page rather than a silent exclusion. The gate
`every_showcase_pattern_declares_its_school` (tests/bug_343_declarations_as_data_test.rs)
fails if any pattern loses its `school`, since a mistyped key is otherwise easy
to miss.

## Token convention

Each style SHOULD declare its own token vars under
`:root { --showcase-<style>-<token> }` so the showcase demonstrates the
project-style protocol described in
`.claude/skills/spacetime-design/references/project-style-protocol.md`.

Inline hex literals are an antipattern; use vars.
