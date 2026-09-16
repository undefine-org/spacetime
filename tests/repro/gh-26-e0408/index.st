// GH-26: E0408 ("signal used but not defined") is implemented in
// src/analysis/signals.rs but never invoked by the production check_file path.
// Unknown reactive signals emit runtime lookups and render empty instead of
// failing compilation.
@data inline $present : "ok";

.probe {
  text <- $definitelyMissing;
}
