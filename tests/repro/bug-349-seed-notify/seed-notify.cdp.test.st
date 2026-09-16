@test "a named template's holes are filled in a real browser" {
  @mount {
    @data inline $strs : ["ab", "cd"] ;
    @template &srow($i) { <li class="b">`$i`</li> }
    <ul class="a"></ul>
    .a { @each($strs as $i) { &srow($i); } }
  }
  @then .a .b should have_length 2
  @then .a .b should have_text "ab"
}
@test "the inline spelling agrees with the named one" {
  @mount {
    @data inline $strs : ["ab", "cd"] ;
    <ul class="a"></ul>
    .a { @each($strs as $i) { @template { <li class="b">`$i`</li> } } }
  }
  @then .a .b should have_length 2
  @then .a .b should have_text "ab"
}
