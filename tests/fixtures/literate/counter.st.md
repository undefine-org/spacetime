# The counter

Press the button; the number after it is **live** — not a screenshot. This
document IS the program: the fence below runs where it sits.

```st hidden
@data inline $n : 0;
```

```st src
<div class="demo">
  <button class="plus">+</button>
  <span class="out"></span>
</div>

.plus { @on &.click { $n <- $n + 1; } }
.out  { text <- $n; }
```

That is the whole reactive loop: the event writes the signal, the binding
renders it. The signal declared above stays live down here.

```spacetime
// An inert quote — shown, never run.
.never { @on &.click { $n <- 999; } }
```
