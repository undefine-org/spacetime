@import "stdlib"
@data inline $people : ["ada lovelace", "grace hopper"];
<ul class="ssg-filters"></ul>
.ssg-filters {
  @each($people as $person) {
    <li class="up">`$person` | uppercase</li>
    <li class="cap">`$person` | capitalize</li>
  }
}
