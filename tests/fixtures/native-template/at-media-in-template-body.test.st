// BUG-167 regression: a root `@media (…) { … }` inside a @template body was
// SILENTLY DROPPED — no error, no emitted CSS (the worst failure mode: no
// component_item grammar arm matched it, and the unclassifiable-directive
// branch consumed it with zero diagnostic). Fixed via:
//   1. stdlib/capture-types/component-body.st: new cb_media capture-type +
//      component_item Choice arm (so @media is a RECOGNIZED, not silently
//      swallowed, construct at the grammar level).
//   2. src/parser/mod.rs: the template-scope construction loop now harvests
//      a body-root @media/@supports/@keyframes directive into
//      file.raw_css_blocks — the SAME sink the top-level `"media" | ...`
//      arm already uses — so it emits identically to page-level @media.
//   3. src/syntax/events/form_compiler.rs: the unclassifiable-directive
//      silent-drop is now registry-aware — an UNKNOWN directive (zero forms
//      in the registry) surfaces E0900; a KNOWN one (@effect, @portal, @fn,
//      …) stays legitimately silent (it's matched by the wider macro-form
//      system, not a component_item arm).
@import "stdlib/testing/test"

@test "@media inside a @template body reaches the stylesheet and actually applies" needs layout {
  @mount {
    @template &probe-media() {
      <div class="mq-box">x</div>
      @media (max-width: 100000px) {
        .mq-box { color: rgb(0, 128, 0); }
      }
    }
    <main class="page"></main>
    .page { &probe-media(); }
  }

  @then .mq-box {
    have_style: color: rgb(0, 128, 0);
  }
}
