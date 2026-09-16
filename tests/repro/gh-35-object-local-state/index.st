// GH-35 / BUG-342: an object-typed local-state initializer must carry its
// value through to the emitted page.
//
//   $user object: { name: "Ada" };   ->  .card renders "Ada"
//
// The markup is not decoration: without an element the page exports nothing
// ("Export produced no pages"), so a build could not show whether the value
// survived. The scalar `$flag` is the control — if BOTH are blank the harness
// is at fault, if only `$user` is blank the object shape is.
@import "stdlib"

<div class="card"></div>
<div class="flag"></div>

body {
  $user object: { name: "Ada" };
  $flag bool: true;
}

.card { text <- $user.name; }
.flag { text <- $flag; }
