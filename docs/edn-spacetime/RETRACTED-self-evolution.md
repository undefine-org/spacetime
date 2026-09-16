# Retracted: "A System That Rewrites Itself"

`self-evolution.typ` was removed (added 143393f5, retracted 9705014c). It named
two "gaps" in the self-evolution loop. **Both were already solved in this repo and
in beam-lisp.** It is deleted rather than corrected because it is wrong at the
foundation, and a wrong design doc that looks rigorous is worse than none.

## Gap A ("no stable identity") — did not exist

Structural addressing needs no identity. A template says what **shape** to match;
a rewrite applies to every match. Nothing is *referred to*, so nothing needs a
stable name — and the diff is the audit record.

Both halves already ship:

- **verse** — `stdlib/migrations/entries/*.st` declare
  `%rewrite name { %drops(…) %match {…} %into {…} }`: the template language,
  written in Spacetime, as data — the same thesis as `%form`. `src/migrate.rs`
  (1879 lines) is the engine: `instantiate_template`, `plan_file_apply`,
  `render_pending_hunk`, `write_apply_plans`. `FileApplyPlan` carries
  `old_source`, `new_source`, `changed`, `content_hash`.
- **beam-lisp** — `priv/optics.bl`: self-hosted optics with `in` `idx`
  `traversed` `filtered` `optional` and `*>` composing outside-in (Specter
  order), plus `examples/optics.bl` and `test/bl/optics_test.bl`.

## Gap B ("verification is advisory") — same error, one level down

Structural rewriting is verifiable by construction: the plan carries old **and**
new source. There is no swap-time refusal problem when the transformation is the
record.

## Why it went wrong (the part worth keeping)

The doc ran ~15 verification calls — every one on something already believed
(`FormMatch` fields, the absent stable-id, `contract_hash_of`, `fence`) and marked
them "verified". It never once searched for **what already solves this**.

It also invented syntax (`@data user from /api/user;`, `@set expanded = true` —
`@set` has no `%form` anywhere) in a document arguing that the registry is the
authority on what is legal.

Rule, saved as `CON-search-for-what-solves-it-before-designi`: before designing a
mechanism, grep for the **capability** (`%rewrite`, `optics`, `migrat*`), not just
the absence you suspect (`stable_id`, `node_id`). Read the sibling dirs. An
example in a document about a grammar must come *from* that grammar.

Verifying "X is absent" is worthless until "is X needed?" has been asked.
