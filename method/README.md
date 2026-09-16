# method/

A reusable process for building brand + website copy that is **elicited, never
invented**, and that each reader completes with their own situation.

Project-agnostic by design: this is the playbook, a project is an instance of it.

```
cargo run -- serve method/
```

Read `index.st.md` first (or `/` when served) — it explains the pipeline and
demonstrates the core completion mechanic live.

- `00-PRINCIPLES` — the laws that gate every later stage
- `01-INTERVIEW` — two-pass elicitation protocol + question banks
- `02-RECORD` — typed evidence-ledger schema
- `03-POSITIONING` — record → brief derivation rules
- `04-SITEMAP` — which pages exist, by a split test
- `05-COPY` — the completion mechanic + the five-gate lint
- `06-AGENT-NEW` — guidelines: build a new page
- `07-AGENT-UPDATE` — guidelines: update a page
- `08-SOURCES` — every claim, with epistemic status

`reader.st` is the shared stylesheet (imported by every page).
