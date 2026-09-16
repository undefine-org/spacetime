// Collaborative Presence Example
// See other users' cursors in real-time

@type Cursor {
  x: number
  y: number
  name: string
  color: string
}

@template &cursor($c) {
  <div class="cursor" data-user-id="`$c.id`" style="--cursor-x: `$c.x`; --cursor-y: `$c.y`; --cursor-color: `$c.color`;">
    <span slot="name">`$c.name`</span>
  </div>
}

.canvas {
  @presence(room: "demo-room", throttle: 30ms) as cursors: Cursor;

  // Remote cursors - @each over the presence-yielded $users signal.
  @each($users as $c, key: $c.id) {
    &cursor($c);
  }

  // Track mouse movement and update presence via the yielded $set function.
  @on &.mousemove {
    $set({ x: $.clientX, y: $.clientY });
  }
}
