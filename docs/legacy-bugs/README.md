# Legacy Compiler Bug Notes

These markdown files are historical Spacetime-compiler bug investigations recovered from the pre-PLAN-015 root-level `bugs/` directory.

PLAN-015 reorganized the repository to move user-site content into a gitignored `projects/` directory and demoted the original `bugs/` directory into the gitignored `scratch/bugs/` working area. The follow-up audit AUD-006 flagged that those notes are still valuable historical context for the `@each` / `@template` rendering work, so they have been re-archived under `docs/legacy-bugs/` to keep them tracked in git.

## Contents

- `BUG-000-each-template-rendering-overview.md` — overview of the `@each` + `@template` rendering failure chain.
- `BUG-001-template-invocation-extractor-stub.md` — extractor stub that emitted raw text instead of a JS array.
- `BUG-002-body-capture-quantifier-loop.md` — body-capture quantifier infinite loop investigation.
- `BUG-003-template-invocation-js-serialization.md` — JS serialization gap for template invocations.

These are reference material only. The active bug tracker lives under `!tasks/bugs/` as org-mode items; create new bug investigations there.
