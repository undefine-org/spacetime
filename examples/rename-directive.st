/// Example script: Rename @fade-in to @reveal
///
/// This script demonstrates how to use the scripting framework
/// to perform source transformations on Spacetime files.

@script "Rename @fade-in to @reveal" {
  @log "Processing file..."

  // Simple text replacement
  @replace "@fade-in" with: "@reveal"

  @log "Replacement complete!"
}
