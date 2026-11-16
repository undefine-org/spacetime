@import "./components.st";
<main><--card>A page without a film.</--card></main>
@template &nested-leaf($text string) { <span class="nested-leaf">`$text`</span> }
@template &main($text string) {
  <section class="nested-parent"></section>
  &nested-leaf($text);
  &named &nested-leaf($text);
}
<div class="nested-example"></div>
.nested-example { &main("Reusable"); }
body { margin: 0; padding: 60px; font-family: sans-serif; background: #f4f4ee; }
.card { border: 2px solid var(--card-accent); border-radius: 16px; padding: 30px; background: white; }
