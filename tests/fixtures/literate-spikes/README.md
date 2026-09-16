# SIP-002 spikes — hand-tangled literate output

Evidence fixtures for docs/specs/SIP-002-literate-spacetime-markdown-host.org.
Each file is what the proposed `.st.md` tangler would EMIT, written by hand to
prove the output language needs nothing new:

- `esc.st`        — prose→@doc(content:) with escaped quotes/newlines/code spans
- `interleave.st` — prose / demo / prose document order preserved in served HTML
- `index.st`      — signal declared early stays live across prose boundaries
- `midimport.st`  — @import legal mid-file (a later fence can import)

All verified: `cargo run -- check tests/fixtures/literate-spikes/` passes; the
interleave fixture was served and its DOM order + live counter confirmed in a
real browser (session 2026-07-24). When SIP-002 lands, these graduate into the
tangler's snapshot corpus.
