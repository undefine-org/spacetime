@import "stdlib"
@data graphql $posts : "/gqlendpoint", query: "{ posts { id title } }";
<ul class="feed"></ul>
.feed {
  @each($posts as $p) {
    <li>`$p.title`</li>
  }
}
