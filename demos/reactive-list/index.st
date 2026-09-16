/* ============================================================================
   Reactive List — a full-Spacetime @each showcase
   No external web-component .js files. No custom JavaScript. Everything below
   is Spacetime: an inline @template defines the row markup, @data seeds the
   list, @each renders it, and clicks mutate the $tasks signal so the list
   re-renders reactively.

   Patterns demonstrated:
     1. @template &row(...)        — inline component markup (no external .js)
     2. @data inline $list : [...]  — userland list state as a signal
     3. @each($list as $i) { &row } — reactive list rendering over a signal
     4. @on &.click { $list <- ... }  — mutate the signal; @each re-renders
     5. @data fold + file-scope binding — live derived count
   ============================================================================ */

/* ---- Inline row template (the per-item component) ----------------------- */

@template &task-row($task) {
  <li class="task" data-id="`$task.id`">
    <span class="task__dot"></span>
    <span class="task__label">`$task.label`</span>
    <button class="task__remove" data-remove>&times;</button>
  </li>
}

/* ---- State: the list of tasks (a userland signal) ----------------------- */

@type Task {
  id: string;
  label: string;
}

@data inline $tasks Task[] : [
  { "id": "t1", "label": "Sketch the wireframe" },
  { "id": "t2", "label": "Wire up the signals" },
  { "id": "t3", "label": "Ship the demo" }
];

/* ---- Derived count (computed signal → file-scope binding) --------------- */

@data fold $taskCount number from $tasks : acc + 1 ;

.count { text: $taskCount; }

/* ---- Render the list + interactions ------------------------------------- */

.task-list {
  @each($tasks as $t) {
    &task-row($t);
  }

  /* Remove a task: filter it out of $tasks → @each re-renders reactively. */
  @on &.click [data-remove] {
    $tasks <- $tasks.filter(t => t.id !== $.closest('.task').dataset.id);
  }
}

/* Add a task: append to $tasks with a unique id. */
.add-task {
  @on &.click {
    $tasks <- $tasks.concat([{
      id: "t" + (Date.now()),
      label: "New task " + ($tasks.length + 1)
    }]);
  }
}

/* Clear all. */
.clear-tasks {
  @on &.click {
    $tasks <- [];
  }
}

/* ============================================================================
   Styling (plain CSS → spacetime.css). Dark, minimal.
   ============================================================================ */

body {
  margin: 0;
  min-height: 100vh;
  display: grid;
  place-items: center;
  background: radial-gradient(120% 120% at 50% 0%, #1a1d29 0%, #0b0c10 60%);
  color: #e8eaf2;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

.card {
  width: min(440px, 92vw);
  padding: 2.5rem;
  border-radius: 20px;
  background: linear-gradient(180deg, rgba(255,255,255,0.06), rgba(255,255,255,0.02));
  border: 1px solid rgba(255,255,255,0.08);
  box-shadow: 0 30px 80px rgba(0,0,0,0.45);
}

.eyebrow {
  margin: 0;
  font-size: 0.75rem;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  color: #7c83ff;
}

.card__head h1 {
  margin: 0.25rem 0 1.5rem;
  font-size: 1.8rem;
  letter-spacing: -0.02em;
}

.card__head h1 .count {
  color: #7c83ff;
}

.task-list {
  list-style: none;
  margin: 0 0 1.25rem;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.task {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.8rem 1rem;
  border-radius: 12px;
  background: rgba(0,0,0,0.3);
  border: 1px solid rgba(255,255,255,0.06);
}

.task__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #7c83ff;
  flex: none;
}

.task__label {
  flex: 1;
  font-size: 0.95rem;
}

.task__remove {
  border: none;
  background: transparent;
  color: #6b7080;
  font-size: 1.25rem;
  line-height: 1;
  cursor: pointer;
  transition: color 0.15s ease;
}

.task__remove:hover { color: #ff6b6b; }

.actions {
  display: flex;
  gap: 0.5rem;
}

.actions button {
  flex: 1;
  border: 1px solid rgba(124,131,255,0.4);
  background: rgba(124,131,255,0.12);
  color: #c9ccff;
  padding: 0.6rem 1rem;
  border-radius: 10px;
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease;
}

.actions button:hover { background: rgba(124,131,255,0.22); }

.clear-tasks {
  border-color: rgba(255,107,107,0.4) !important;
  background: rgba(255,107,107,0.1) !important;
  color: #ffb3b3 !important;
}
