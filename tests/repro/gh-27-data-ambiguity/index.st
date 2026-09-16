// GH-27: `@data inline $events : [];` and `@data inline $filterCategory : "all";`
// are literal-aware: parse selected data-inline and stored it in FormMatch.
// The ambiguity lint should NOT fire E0923 for these canonical kind forms.
@import "stdlib"

@data inline $events : [];
@data inline $filterCategory : "all";

.probe { color: red; }
