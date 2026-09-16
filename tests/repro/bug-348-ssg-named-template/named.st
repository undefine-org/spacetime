@import "stdlib"
@data inline $items : ["ab","cd"] ;
@template &row($i) { <li>`$i`</li> }
<ul class="l"></ul>
.l { @each($items as $i) { &row($i); } }
