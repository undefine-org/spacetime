@import "stdlib/md"

@data inline $count : 0;

<main class="essay">
  <section class="lit-prose" data-lit-seg="0"></section>
  <pre class="lit-src">.plus { @on &.click { $count <- $count + 1; } }</pre>
  <div class="lit-run" data-lit-run="1">
    <button class="plus">+</button>
    <span class="out">0</span>
  </div>
  <section class="lit-prose" data-lit-seg="2"></section>
</main>

[data-lit-seg="0"] { @doc(content: "# The counter\n\nPress the button; the number that follows is **live**: `$count`.") }
[data-lit-seg="2"] { @doc(content: "That is the whole loop: event writes the signal, the binding renders it.") }

.plus { @on &.click { $count <- $count + 1; } }
.out  { text <- $count; }
