// GH-22: @use leaks '// Primitive not found: use' into dist/spacetime.js
@use "./module.st" as b
<main><span class="badge"></span></main>
.badge { @b/badge("NEW") }
