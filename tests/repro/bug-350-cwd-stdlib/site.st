@import "stdlib"
@data inline $items : ["ready"] ;
@template &row($i) { <li class="repo">`$i`</li> }
<ul class="l"></ul>
.l { @each($items as $i) { &row($i); } }
