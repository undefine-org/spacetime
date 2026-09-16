# Comments

A comment is a conversation pinned to a place in a project.

Someone writes one — a person in the dev-dock pill, or an agent over MCP.
Someone else picks it up, works it, and resolves it. The conversation lives
next to the thing it is about, survives in git, and is readable by every
surface: the pill, `spacetime check`, and an agent's tools.

This is not a TODO scraper. A `//@todo` in a file is only the *content* of a
comment; who claimed it, what they replied, and whether it is settled are
recorded beside it and travel with the repository.

---

## Writing one

A comment is a real comment — `//` followed by `@`. It has no compile effect,
so it is valid anywhere `//` is a comment and it can never break a build.

```spacetime
//@: a bare marker is a note — no follow-up implied
//@todo: replace the placeholder photography
//@bug(severity: high): nav overlaps the CTA below 380px
//@agent-task(acceptance: "cargo test --lib comments"): swap the hero image
```

The shape is always the same:

```
//@<type>(<field>: <value>, …): <text>
```

- **`<type>`** is a declared comment type. Omit it (`//@:`) and you get `note`.
- **`(…)`** carries the type's declared fields. Values may be quoted; a quoted
  value can contain commas, colons and parentheses.
- **everything after the first `:`** is literal text — URLs, colons and `//`
  included, because prose contains all three.

### More than one line

A continuation line carries an explicit marker:

```spacetime
//@design: the grid is asymmetric on purpose
//@> the eye should land on the price before the photograph,
//@> so the column split is 7/5 rather than 6/6
//@>
//@> (a bare marker is a paragraph break)
```

**An unmarked `//` line is never absorbed into a comment**, no matter how
adjacent:

```spacetime
//@todo: the actual work
// an unrelated aside — stays unrelated
```

That rule costs one marker per line and buys something worth more: a `//@>`
whose header was deleted becomes *detectable*. `spacetime check` reports it as
a continuation without a header, instead of the text silently attaching itself
to whatever comment happens to sit above. Ambiguity here would be invisible and
unfixable — a stray thought quietly becoming part of a work item nobody wrote.

### Where `//@` does *not* work

Inside an HTML markup block, `//` is **literal text that renders to visitors**:

```spacetime
<div class="stage">
  // this line appears on the page
</div>
```

That is pre-existing language behavior — the same rule that makes a bare `$x`
literal in markup. Comments are therefore harvested from file scope and CSS
scope only. A marker inside markup is left alone, because filing visible page
copy as a private work item (and offering to "resolve" it) would be worse than
not supporting the case at all.

---

## Comment types are data

A type is declared, not hardcoded. Six ship in the stdlib:

| type | what it means | fields |
|---|---|---|
| `note` | an observation; no follow-up implied | — |
| `todo` | a unit of work anyone can pick up | `owner?` |
| `question` | needs an answer before work proceeds | `asked-of?` |
| `bug` | something behaves wrongly | `severity?` |
| `design` | rationale; usually stays open as a record | — |
| `agent-task` | work for an agent, with an observable check | `acceptance`, `priority?` |

A project declares its own in **`<project>/_prelude.st`** — the project
overlay, auto-loaded into the registry before any page compiles and never
served as a page:

```spacetime
%comment_type brand-review {
  %label "Brand review"
  %docs  "Needs sign-off from brand before shipping."
  %field reviewer string
  %field due string?
  %color "#b06ee8"
  %agent_hint "Check the copy against docs/brand-spec.md, then reply with
               what you changed. If the decision needs a human, leave it open."
}
```

Declaring it is the entire act of adding it. The type appears in the pill's
picker, in MCP validation, and in `check`'s roster with no Rust change, because
all three read one registry query. A `%comment_type` in a page file compiles
but never reaches the roster, so the compiler warns (**W0717**) at the
declaration and names the file it belongs in.

Field kinds are `string`, `number`, `bool`, `ident`; a trailing `?` makes the
field optional. A space introduces the type (`%field reviewer string`) — never
`name: type`, because `:` introduces a *value* everywhere else in the language.

**`%agent_hint` is the type's follow-up contract.** It travels inline with every
comment an agent lists, so an agent picking work up receives the instructions
*with* the work instead of inferring them.

---

## Status, and where state lives

Every comment has one of four statuses:

`open` · `in-progress` · `resolved` · `wontfix`

Storage is split by role, deliberately:

- **Content** lives inline in `.st` source. It travels with the code, shows up
  in review, and merges like code.
- **Workflow state** — status, claim, thread — lives in
  `<project>/.comments/<id>.json`, one file per comment. State changes far more
  often than content, and it must be writable for things that cannot host a
  comment at all (a JSON file, an asset, a route, an element).

Both are committed to git. Comment state is team data, not scratch.

An inline comment's id is a **content hash of its header** (file + type + fields
+ the first line of text). Editing the body — fixing a typo, adding a
continuation — never detaches its thread. Editing the *header* deliberately
mints a new identity, because that is a change to what is being asked.

---

## Anchors

| kind | pins to | created by |
|---|---|---|
| `inline` | a source line | writing `//@…` in the file |
| `file` | a file, optionally a path within it | the pill or an agent |
| `page` | a route | the pill or an agent |
| `element` | a route + an element's content identity | the pill's element picker, an agent, or a re-anchor |

Inline anchors cannot be *minted* through the pill or MCP: an inline comment's
identity comes from its source line, so a record created with a fresh id could
never match the header it claims — it would be born orphaned. You create an
inline comment by writing one.

### How an element anchor survives an edit

An element anchor is identified by its **content** — tag, class, id, and a
digest of its text — not by its position. That distinction is the whole design.

Spacetime's structural ids (`data-st-node`) are *positional*: `0.1.2` means
"third child of the second child of the root". An anchor keyed on that would not
orphan when you edit the page — it would resolve cleanly to a **different**
element, sliding a note like "this claim is unsupported" onto the next
paragraph with no warning. A lost comment is visible; a relocated one reads like
data while being wrong.

So the selector is kept only as a hint, and edits resolve like this:

| you | the comment |
|---|---|
| insert markup above it | follows its element |
| re-indent or reformat | stays put (whitespace normalizes) |
| append to a long paragraph | stays put (only a text prefix is hashed) |
| rewrite what it says | **orphans** — the subject changed |
| delete it | **orphans** — never inherits a neighbour |
| duplicate it identically | reported as ambiguous, never guessed |

Comments written before this existed have no fingerprint. They still load, and
are shown as `(unverified)` in the pill and named by `check` — re-anchor one
from the pill to give it a verified identity.

### Where it was written

"Show on page" finds where an element **renders**. **Where in source** finds
where it was **written** — and they are different questions.

Source survives what rendering cannot: an element behind a branch that is
currently false, an element emptied of its text, and duplicate rows that are
indistinguishable once rendered but sit at different byte offsets. A comment
saying *"this markup is wrong"* is only actionable from the source.

The span is **resolved on every press**, never stored on the anchor. A stored
byte offset goes stale the moment anyone edits the file above it, and would
then point confidently at the wrong markup — the same positional failure
[fingerprinting](#how-an-element-anchor-survives-an-edit) already fixed once.
The stored content is the identity; the span is a view of the file as it is
now.

The positional `data-st-node` hint is *confirmed* before its span is used. If
markup was inserted above your element, the hint now names a different node —
so a hit that disagrees with the stored content is discarded and identity finds
the element instead. When that happens the answer says `moved`, because a
surprising location is only trustworthy if the drift is stated.

The pill asks `GET source.json`. Outcomes, all of them sayable:

| outcome | meaning |
|---|---|
| `found` | resolved; `moved` says whether the hint had gone stale |
| `gone` | nothing in the source matches — the comment has drifted from the code |
| `ambiguous` | several elements are written identically and cannot be told apart |
| `unverified` | the comment predates element identity, so nothing can be confirmed |
| `synthesized` | the HTML parser inserted this element; there is no authored source |
| `no-source` | the route does not resolve to a `.st` entry, or it does not compile |
| `not-an-element` | inline anchors already carry file+line; page/file anchors *are* the address |

**Spans are currently relative to the template body, not the file.** The
response says so (`relative_to: "template-body"`) and names the template, rather
than reporting a body offset as if it were a file offset — that would send a
reader to the top of the file to look at unrelated markup. A file line needs a
body offset on the template bundle; until then the answer is honest about its
frame of reference.

### Who moved it

Every status change records its **author**, the status it moved **from**, and
when. A bare status answers "where is this now" but not "who decided that" —
and on a shared record the second question is the accountable one. `resolved`
reads identically whether someone verified the fix or an agent closed work it
never read, and a reviewer who disagrees needs somebody to ask.

The origin is kept because *reopened from resolved* and *claimed from open* are
different events, and a log of destinations alone cannot tell them apart.
Re-submitting a status a comment already has is not a transition and records
nothing.

Comments written before this existed have an empty log. That means **unknown**,
not "never changed" — the pill shows no history rather than implying none
happened.

### When a header disappears

Edit or delete a `//@` header and its workflow state survives with no live
anchor. That record becomes an **orphan** — surfaced, never silently dropped,
because the thread is the valuable part. From the pill an orphan can be:

- **re-anchored** to a page, file, or element anchor, preserving its id, status,
  author, and entire thread; or
- **dismissed**, which deletes the record.

`spacetime check` reports orphans with both options named, so someone reading
CI output knows the record is actionable.

---

## The three surfaces

### The pill (`spacetime serve`)

Leftmost in the dev dock. Shows open comments with a count, filters by type,
claims and resolves, threads replies, and composes new comments with a type
picker driven by `types.json`. Written entirely in Spacetime — no custom
JavaScript.

Routes, all under `/__spacetime/comments/`:

| route | does |
|---|---|
| `GET types.json` | the declared roster |
| `GET status.json` | merged records, orphans, diagnostics, counts (filterable by `route`, `file`, `status`, `type`) |
| `GET source.json` | where an element comment was WRITTEN (`?id=<id>`) — resolved against the current file, never stored |
| `POST add` | create a page/file/element-anchored comment |
| `POST update` | change status, append a reply, edit text |
| `POST re-anchor` | move an orphan to a new anchor |
| `POST dismiss` | delete a record |
| `POST prune` | remove resolved inline comments from source |

Validation is strict: an unknown type, a missing required field, or an anchor
outside the project is a `4xx` with a message naming what would have worked —
never a silent write.

### `spacetime check`

An informational section: counts by status and type, a row per open comment
with its hint, and the scanner's warnings (orphans, continuations without
headers, unknown types, shape mismatches).

**It never fails the build.** A build that broke because someone typed a comment
badly would teach people to stop writing comments.

### MCP (agents)

| tool | does |
|---|---|
| `st_comments_types` | the declared roster |
| `st_comments_list` | matching comments, each row `{ comment, agent_hint }` |
| `st_comments_add` | create a comment authored by the agent |
| `st_comments_update` | change status and/or append a reply |

Plus a `comments://<env>` resource: the same index, readable as a resource for
clients that attach it to context rather than calling a tool. Subscribing to it
is accepted, but no change notifications are emitted yet — re-read to refresh.

The hint travels **beside** the record, never inside it: `CommentRecord` is one
shape on disk, on the routes, and in MCP results.

---

## Pruning resolved comments

A resolved inline comment can be removed from source through the pill. This is
the only action in the feature that writes to a person's source file, and it is
built to refuse:

- it rides the **existing guarded edit rail** — the same one the inspector uses
  — carrying a whole-file hash *and* the exact expected text of every span;
- a stale hash or a moved span **refuses and writes nothing**, because the spans
  no longer mean what the caller was shown;
- deletion covers exactly the `//@` header and its `//@>` continuation run —
  never an adjacent plain comment, never a following line of code;
- only `resolved` comments are eligible; pruning open work would delete work in
  progress.

The pill states what it will remove before it runs.

---

## The invariant that makes all of this cheap

**A comment is trivia.** Nothing in this feature can change emitted output or
fail a build. An unknown type, a malformed header, an unreadable `.comments/`
directory — every one of them degrades to a diagnostic.

That is what lets the surface evolve without ceremony:

| what changes | how it migrates |
|---|---|
| the `//@` syntax | the scanner parses leniently and `check` flags deprecated forms — no `%migration` wave, because there are no compile semantics to preserve |
| `%comment_type` grammar | ordinary `%migration` machinery; it is real metasystem source |
| the sidecar schema | records carry `"v"`; reads are lenient and upgrade on write |

Keep it true. The moment a comment becomes compile-relevant, every row of that
table gets more expensive.

---

## See also

- `stdlib/comments/MODULE.st` — the module's own documentation
- `stdlib/comments/entries/defaults.st` — the six default types, as data
- [migrations.md](migrations.md) — the same kinds-as-data pattern, for retired syntax
