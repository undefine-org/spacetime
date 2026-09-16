/// Headless test: `@on $sig.change` fires when the watched signal is
/// REASSIGNED — the head-watch semantic the retired `@effect(on: $sig.field)`
/// delivered (BUG-155-class regression: `@effect`'s effect-runner keyed its
/// subscription by the LITERAL dotted string, so `@effect(on: $wrap.count)`
/// never fired because no signal is literally named "wrap.count"; the fix
/// watched only the HEAD signal).
///
/// `@on` has no dotted syntax — `$sig.change` watches the signal by name, and
/// the change body re-reads whatever it needs via ST.runMutations' own $name
/// substitution — so reassigning the watched signal is exactly what fires it.
/// This pins that once-per-reassignment contract for the change-driver. The
/// watched signal is DECLARED (`@data inline`), as the change-driver resolves
/// the watch in the scope the author's reads use (an undeclared bridged name
/// would not be a resolvable global signal for `$sig.change`).

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"

@data inline $edpWrap : null;
@data inline $edpWrap2 : null;
@data inline $edpLog : 0;
@data inline $edpLog2 : 0;

@template &edp-effect() {
    <div class="edp-host"></div>

    .edp-host {
        @on $edpWrap.change {
            $edpLog <- $edpLog + 1;
        }
    }
}

@template &edp-effect2() {
    <div class="edp-host2"></div>

    .edp-host2 {
        @on $edpWrap2.change {
            $edpLog2 <- $edpLog2 + 1;
        }
    }
}

@test "@on $sig.change fires when the watched signal is reassigned" {
    @fixture {
        <div class="host-edp-effect"></div>
    }
    @eval (document.querySelector('.host-edp-effect').appendChild(Spacetime.invokeTemplate('edp-effect')))
    @eval (ST.initElement(document.querySelector('.host-edp-effect .edp-host')))
    @wait 60ms
    @eval (window.SpacetimeLocal.edpWrap = { count: 1 })
    @wait_for_signal $edpLog on .edp-host to_equal 1 timeout 1500ms
}

@test "@on $sig.change fires again on a subsequent reassignment" {
    @fixture {
        <div class="host-edp-effect2"></div>
    }
    @eval (document.querySelector('.host-edp-effect2').appendChild(Spacetime.invokeTemplate('edp-effect2')))
    @eval (ST.initElement(document.querySelector('.host-edp-effect2 .edp-host2')))
    @wait 60ms
    @eval (window.SpacetimeLocal.edpWrap2 = { count: 1 })
    @wait_for_signal $edpLog2 on .edp-host2 to_equal 1 timeout 1500ms
      @eval (window.SpacetimeLocal.edpWrap2 = { count: 2 })
      @wait_for_signal $edpLog2 on .edp-host2 to_equal 2 timeout 1500ms
}
