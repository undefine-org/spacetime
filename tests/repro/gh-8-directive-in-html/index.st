// GH-8: directive nested inside a file-scope HTML literal.
// Issue shape: `<div class="x"> @scroll s(...) { ... } </div>`.
// A directive written next to the element it drives should either work
// or produce an E0900-class diagnostic, never silently vanish / leak text.
<div class="x">
  @scroll fade(start: 0, end: 1) { opacity: 0 -> 1; }
</div>
