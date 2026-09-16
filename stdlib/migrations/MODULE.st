/// Spacetime `migrations` module — syntax retirements as stdlib DATA (PLAN-076).
///
/// When the language retires an authoring surface, the retirement is declared
/// HERE — one `%migration` entry per retired shape — never as a Rust
/// match-arm in the pipeline:
///
///   %migration bind-text-to-arrow {
///     %date 2026-06-09
///     %docs "FEAT-072: @bind(text:) became the `text <- $expr` arrow binding."
///     %match   { @bind(text: $x:expr) }
///     %rewrite { text <- `$x`; }
///   }
///
/// - `%date` is THE wave key: a wave = every migration sharing one date;
///   waves apply oldest-first. Entry filenames MUST start with the wave date
///   (`YYYY-MM-DD-<id>.st`) — sorted load order is what makes the
///   forward-only chain check sound.
/// - `%match` is EXACTLY the `%form` grammar; the parse-side registry sees it
///   as a synthetic macro (whose name lands in `FormMatch.matched_macro`) so
///   retired directives still parse into a FormMatch instead of being
///   dropped as unmatched children.
/// - `%rewrite` is a source-text template with backtick-quoted holes
///   (`` `$x` `` — THE hole form; a bare `$x` is literal output). Holes splice
///   the RAW SOURCE SLICE of the capture, never a typed re-serialization.
/// - `%hint` is the alternative for retirements that are NOT mechanically
///   rewritable: matching source hard-errors (E0910) with the hint text.
///
/// Compile-time semantics: rewrite-kind matches are rewritten IN MEMORY
/// (compat shim) with a warning diagnostic, so old syntax keeps compiling;
/// the migrations pill (`__dev__/pill.st`) or `spacetime migrate` persists
/// the rewrite to disk and bumps the project's `@version` fact.
///
/// Layout:
///   entries/   %migration defs — the ONLY registry-loaded part of the module
///   __dev__/   the migrations pill — dev-server-compiled, NEVER registry-loaded
///   tests/     *.test.st
