@import "stdlib"
@data inline $items : ["ab","cd"] ;
<ul class="l"></ul>
.l { @each($items as $i) { @template { <li>`$i`</li> } } }
