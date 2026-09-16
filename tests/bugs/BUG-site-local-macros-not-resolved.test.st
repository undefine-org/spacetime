/// BUG: Site-local %primitive/%macro definitions are never matched during parsing
///
/// When a site defines a %primitive and %macro in an imported file (e.g.,
/// `@import "./_my-primitive.st"`), the meta_defs are correctly merged into the
/// AST by resolve_imports → merge_ast. The compiler's `register_user_meta_defs`
/// step (compiler.rs:541) successfully registers them in the MetaRegistry.
///
/// However, FormMatch extraction happens EARLIER, during cst_to_stfile (parser/mod.rs:385),
/// which uses only the STDLIB_REGISTRY (a static SyntaxRegistry populated from
/// embedded stdlib files). Site-local macro forms are never added to this registry,
/// so directives like `@my-macro(...)` produce zero FormMatches. The pipeline then
/// has nothing to resolve/expand/emit → 0-byte JS/CSS output.
///
/// The fix requires either:
/// (a) A two-pass approach: parse imports first to extract meta_defs, register them
///     in the SyntaxRegistry, then re-parse the main file with the augmented registry.
/// (b) A deferred matching pass after compile_pipeline registers user meta_defs.
///
/// Expected: site-local macros produce JS/CSS output identical to stdlib macros.
/// Actual: 0 bytes emitted; the directive is silently ignored.
///
/// Impact: No site can define custom %primitive/%macro files — only stdlib works.
/// This blocks the entire site-local abstraction story.

// This file cannot be a runnable test because the bug is in the compiler pipeline.
// The test is structural: build /tmp/test-site3/ and verify non-zero JS output.
//
// Repro:
//   1. Create /tmp/test-site3/_test-prim.st with a %primitive and %macro
//   2. Create /tmp/test-site3/index.st with @import "./_test-prim.st" and @test-local(...)
//   3. Run: cargo run -- inspect --layer emit /tmp/test-site3/index.st
//   4. Observe: 0 bytes JS, 0 bytes CSS

body {
  // If site-local macros worked, this would emit JS.
  // Currently silently ignored.
  // @test-local(size: 50)
}
