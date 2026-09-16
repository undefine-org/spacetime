// GH-34: warning-only files print green success lines with no warning text.
// `@data inline $unused : 1;` warns "defined but never used" (W0201).
@import "stdlib"

@data inline $unused : 1;

body { margin: 0; }
