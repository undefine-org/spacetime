// ============================================
// REUSABLE PATTERNS
// ============================================
// Common UI patterns for mobile components


// Button press feedback
@pattern press-feedback($duration) {
    @on &.click press($duration) {
        & {
            scale: 1 -> 0.95 -> 1;
            easing: --ease-out-quad;
        }
    }
}


// Hover lift effect
@pattern hover-lift($distance) {
    @on &.hover lift(200ms) {
        & {
            translate-y: 0 -> $distance;
            easing: --ease-out-quad;
        }
    }
}


// Hover scale effect
@pattern hover-scale($scale) {
    @on &.hover scale-up(200ms) {
        & {
            scale: 1 -> $scale;
            easing: --ease-out-quad;
        }
    }
}


// Row slide hover
@pattern row-hover($distance) {
    @on &.hover row-slide(200ms) {
        & {
            translate-x: 0 -> $distance;
            easing: --ease-out-quad;
        }
    }
}


// Fade in entrance
@pattern fade-in-up($duration) {
    @on visible fade-up($duration, immediate: true) {
        & {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: --ease-out-expo;
        }
    }
}


// Bounce scale entrance
@pattern bounce-in($duration) {
    @on visible bounce($duration, immediate: true) {
        & {
            opacity: 0 -> 1;
            scale: 0.5 -> 1.1 -> 1;
            easing: --ease-out-back;
        }
    }
}


// Continuous rotation (for spinners)
@pattern spin($duration) {
    @loop rotate($duration) {
        & {
            rotate: 0deg -> 360deg;
            easing: linear;
        }
    }
}


// Toggle switch — reactive boolean signal toggled on click
@pattern toggle-switch() {
    $on bool: false;
    .is-on: $on;

    @on &.click { $on <- !$on; }
}

.toggle-switch.is-on {
    background-color: var(--st-color-primary)
}


// Checkbox — reactive boolean signal toggled on click. (Intermediate
// checking/unchecking animation states are not modelled: no after-delay trigger.)
@pattern checkbox() {
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


// Bottom sheet — reactive boolean signal.
// open/close triggers are wired by the caller (see index.st); the pattern only
// exposes the signal + visibility classes.
@pattern bottom-sheet() {
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


// Swipe to delete row — reactive string signal.
// Swipe gestures (swipe-start, swipe-cancel, swipe-complete) have no Spacetime
// trigger in this demo; the pattern demonstrates the reactive state mapping.
@pattern swipe-delete() {
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
