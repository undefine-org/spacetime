# Vendored dependency provenance

> The gingerbill contract: each dependency is a liability we are responsible for.
> Pinned · owned-behind-a-primitive · manually bumped. See `docs/stdlib/VENDORING.md`.

## snarkdown

| field | value |
|-------|-------|
| name | `snarkdown` |
| source | git submodule → https://github.com/developit/snarkdown |
| pinned commit | `7619436b10a3e31e6779bf1209da4762409b86be` |
| license | MIT — © Jason Miller |
| entry wrapper | `snarkdown-entry.ts` (re-exports default `parse` onto `globalThis.snarkdown`) |
| bundled artifact | `snarkdown.bundle.js` (generated, ~2KB IIFE) |
| bundler | `bun build --format=iife --global-name=snarkdown --minify` |

### Why we depend on it

Markdown → HTML so Spacetime sites can render `.md` docs (the testing guides) as
HTML behind the `@doc` macro. AGENTS.md forbids hand-rolling parsers in Rust for
stdlib constructs, and snarkdown is a single ~1KB function — exactly the kind of
small, owned, auditable dependency the vendoring rail is for. We do NOT
reimplement markdown in Rust.

### Public surface we use

`snarkdown(markdownText) -> htmlString` (the library's default export, bound to
`globalThis.snarkdown` by `snarkdown-entry.ts`).

### Updating (manual — friction is a feature)

```sh
git -C stdlib/md/vendor/snarkdown fetch origin
git -C stdlib/md/vendor/snarkdown checkout <new-commit>
cargo run -- vendor build md          # rebundle from the new pin
# update this file's pinned-commit, re-run the md module tests, commit
```

### Runtime requirements

None beyond a DOM `Element.innerHTML` sink. Pure string→string transform; runs
in the V8 headless test runtime and in browsers alike.
