/// Spacetime `comments` module — structured comments as stdlib DATA (PLAN-123).
///
/// A comment is a conversation attached to a place in a project: a human or an
/// agent writes one, another human or agent picks it up, works it, and resolves
/// it. The KINDS of comment are declared here — one `%comment_type` entry per
/// kind — never as a Rust enum in the scanner:
///
///   %comment_type todo {
///     %label "Todo"
///     %docs  "A small unit of work anyone can pick up."
///     %field owner string?
///   }
///
/// - `%label` + `%docs` are REQUIRED: every type must be able to explain
///   itself in a picker and in `check` output without a human guessing.
/// - `%field <name> <kind>` declares typed metadata the `(…)` header accepts;
///   a trailing `?` makes it optional. Kinds: string, number, bool, ident.
///   SIGIL HARMONY: a SPACE introduces a TYPE (`$price number`), never `:`,
///   which everywhere else introduces a VALUE.
/// - `%agent_hint` is the type's FOLLOW-UP CONTRACT. It travels inline in every
///   MCP list row, so an agent picking a comment up receives the instructions
///   with the work instead of inferring them.
/// - `%color` is an optional accent for the pill's chip.
///
/// Authoring surface (the comment itself) is a plain comment — zero language
/// grammar change, valid anywhere `//` is valid, and INERT to the compiler:
///
///   //@todo: replace the placeholder photography
///   //@bug(severity: high): nav overlaps the CTA below 380px
///   //@> continuation lines carry the explicit marker
///
/// THE INVARIANT: a comment is trivia. Nothing declared here can change
/// emitted output, so an unknown type or a malformed header degrades to a
/// warning — never a broken build. Evolution of the `//@` surface therefore
/// needs no `%migration` wave; the scanner parses leniently and `check`
/// reports what it could not understand.
///
/// A PROJECT declares its own types in `<project>/_prelude.st` — the project
/// overlay — and they join the roster everywhere (pill picker, MCP validation,
/// `check`) with no Rust change, because all three read one registry query.
///
/// Layout:
///   entries/   %comment_type defs — the ONLY registry-loaded part of the module
///   __dev__/   the comments pill — dev-server-compiled, NEVER registry-loaded
