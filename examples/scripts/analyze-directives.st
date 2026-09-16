/// Analysis Script: Directive Usage Report
///
/// Demonstrates AST traversal to analyze directive usage across files.
/// Generates a report of all @directives found with their parameters.
///
/// Usage:
///   spacetime script examples/scripts/analyze-directives.st ./src --format json

@script "Analyze directive usage" {

  @log "Analyzing: " + $path

  // Track found directives
  const directives = [];

  // Find all macro calls in the AST
  @find "@" {
    // $node and $span are available in the body
    directives.push({
      name: $node.name,
      line: $span.start?.line || 0,
      args: $node.args?.length || 0
    });
  }

  // Log summary
  @log "Found " + directives.length + " directives"

  // Output JSON report
  @output-json "-" data: {
    file: $path,
    directives: directives,
    count: directives.length
  }
}
