@import "stdlib/md"

@data inline $n : 0;

<section class="p1"></section>

.p1 { @doc(content: "First prose block, with a live readout below.") }

<div class="demo1"><button class="b1">+1</button><span class="o1">0</span></div>

.b1 { @on &.click { $n <- $n + 1; } }
.o1 { text <- $n; }

<section class="p2"></section>

.p2 { @doc(content: "Second prose block **after** a demo.") }
