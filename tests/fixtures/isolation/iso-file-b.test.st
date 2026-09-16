/// BUG-107 isolation guard — file B.
///
/// Sibling of iso-file-a with a DIFFERENT source/template name. Run together in
/// one headless invocation, B's mount used to poison A's leaked global observer
/// (and vice-versa) with a `null nodeType` throw. A fresh V8 context per file
/// removes the leak. See iso-file-a for the full rationale.

@import "stdlib/testing/test"

@template &isob-row($item) {
    <li class="isob-row"></li>
}

@template &isob-list() {
    <ul class="isob-list"></ul>

    .isob-list {
        @each($isobItems as $item) {
            &isob-row($item);
        }
    }
}

@test "iso B ST.setData drives @each render (2 items)" {
    @fixture {
        <div class="host-isob"></div>
    }
    @eval (document.querySelector('.host-isob').appendChild(Spacetime.invokeTemplate('isob-list')))
    @eval (ST.initElement(document.querySelector('.host-isob .isob-list')))
    @wait 60ms
    @eval (ST.setData('isobItems', [{id: 'x'}, {id: 'y'}]))
    @wait 120ms
    @assert (document.querySelector('.host-isob .isob-list').children.length === 2)
        "iso B @each must render 2 rows after ST.setData"
}
