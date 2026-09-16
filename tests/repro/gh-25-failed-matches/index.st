// GH-25: registered directives in shapes matching none of their %forms.
// Issue: best MatchDiagnostic is discarded → zero diagnostics + empty expansion.
.probe { @scroll(name: demo) { opacity: 0 -> 1; } }
.probe { @load(name: intro) { opacity: 0 -> 1; } }
.probe { @each(: $items) { &item($item); } }
