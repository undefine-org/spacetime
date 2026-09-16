// ============================================
// COMPONENT STATE — signals + reactive classes
// ============================================
// Demonstration library: each component models its state as a signal + reactive
// class binding. Click/pointer triggers are wired; gesture-only interactions
// (swipe, custom open/close events) are left as signals with a note where they
// have no Spacetime trigger in this demo.


// App Shell - View navigation
$view string: "tasks";

.app {
    .is-settings: $view == "settings";
}

.task-list-view {
    [hidden]: $view != "tasks";
}

.settings-view {
    [hidden]: $view != "settings";
}


// Add Button - Press states
.add-button {
    $pressed bool: false;
    .is-pressed: $pressed;

    @on &.pointerdown { $pressed <- true; }
    @on &.pointerup   { $pressed <- false; }
}

.add-button.is-pressed {
    transform: scale(0.95)
}


// Task Row - Swipe to delete (swipe gestures have no Spacetime trigger here)
.task-row {
    $swipeState string: "idle";
    .is-swiping: $swipeState == "swiping";
    .is-deleting: $swipeState == "deleting";
}

.task-row.is-swiping {
    transform: translateX(-80px)
}

.task-row.is-deleting {
    opacity: 0;
    transform: translateX(-100%)
}


// Checkbox - Check/uncheck with a boolean signal
.task-checkbox {
    $checked bool: false;
    .is-checked: $checked;

    @on &.click { $checked <- !$checked; }
}

.task-checkbox.is-checked {
    background-color: var(--st-color-primary);
    border-color: var(--st-color-primary)
}

.task-checkbox {
    transition: background-color 200ms, border-color 200ms
}


// Tab Item - Active/inactive demonstration (per-element toggle)
.tab-item {
    $active bool: false;
    .is-active: $active;

    @on &.click { $active <- !$active; }
}

.tab-item.is-active {
    color: var(--st-color-primary)
}


// Toggle Switch - On/off
.toggle-switch {
    $on bool: false;
    .is-on: $on;

    @on &.click { $on <- !$on; }
}

.toggle-switch.is-on {
    background-color: var(--st-color-primary)
}


// Bottom Sheet - Open/close (open/close triggers wired by the caller)
.task-edit-sheet {
    $open bool: false;
    .is-open: $open;
}

.task-edit-sheet {
    transform: translateY(100%);
    opacity: 0;
    pointer-events: none;
    transition: transform 300ms, opacity 300ms
}

.task-edit-sheet.is-open {
    transform: translateY(0);
    opacity: 1;
    pointer-events: auto
}
