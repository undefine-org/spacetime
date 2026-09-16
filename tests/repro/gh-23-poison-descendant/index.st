// GH-23: descendant-combinator rule whose key (rightmost) selector matches no
// markup poisons every directive AFTER it in source order.
// Current syntax: state is a selector-scope `$name type : val;` declaration on an
// element that exists; the handler head is `@on click { $state <- v; }`.
.wrap { $frame number : 0; }

.wrap { padding: 4px }
.item { padding: 4px }
.out  { color: black }

/* handler BEFORE the poison rule — should emit */
.btn-a { @on &.click { $frame <- 1; } }

/* `.nope` exists in no markup. This rule compiles and passes `check`. */
body:hover .nope { opacity: 1 }

/* handler AFTER — GH-23 says this silently vanishes */
.btn-b { @on &.click { $frame <- 2; } }

.out { text: $frame }
