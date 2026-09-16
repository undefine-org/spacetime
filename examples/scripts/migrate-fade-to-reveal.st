/// Migration Script: @fade-in to @reveal
///
/// Demonstrates AST-aware transformation that renames @fade-in directives
/// to @reveal while preserving all parameters and updating related comments.
///
/// Usage:
///   spacetime script examples/scripts/migrate-fade-to-reveal.st ./src --dry-run
///   spacetime script examples/scripts/migrate-fade-to-reveal.st ./src --write

@script "Migrate @fade-in to @reveal" {

  // Replace the directive name
  @replace("@fade-in", "@reveal")

  // Also update any comments referencing the old name
  @replace("// fade-in", "// reveal")
  @replace("/* fade-in", "/* reveal")

  // Update import statements if they exist
  @replace("fade-in.st", "reveal.st")

}
