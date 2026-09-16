@import "stdlib"
@type Size { code: string; label: string; }
@data inline $sizes Size[] : [
  { "code": "S", "label": "Small" },
  { "code": "M", "label": "Medium" }
] ;
@template &size-opt($s) { <li class="opt" data-code="`$s.code`">`$s.label`</li> }
<ul class="size-list"></ul>
.size-list { @each($sizes as $s) { &size-opt($s); } }
