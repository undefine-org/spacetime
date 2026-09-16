// GH-16: bare-tag-led selector as FIRST rule after a semicolon-less @import.
// Issue: silently absorbed into @import's own parsing → zero output.
@import "stdlib"

canvas.hero {
  width: 100%;
  height: 100vh;
}
