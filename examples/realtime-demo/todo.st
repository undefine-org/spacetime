// Real-time Todo List Example
// Multiple browser tabs will sync automatically

@type Todo {
  id: string
  text: string
  done: boolean
}

@realtime todos: Todo[] {
  channel: "todos"
}

@template &todo-item($t) {
  <div class="todo-item" data-done="`$t.done`" data-id="`$t.id`">
    <span slot="text">`$t.text`</span>
  </div>
}

.todo-app {
  // @each handles iteration with template instantiation
  @each($todos as $t, key: $t.id) {
    &todo-item($t);
  }

  @on &.submit [form] {
    $todos.insert({ text: &input.value, done: false });
    &input <- "";
  }
}
