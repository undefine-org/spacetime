// ============================================
// ANIMATIONS
// ============================================
// Entrance, hover, click, and loop animations


// App entrance

.app {
    @on &.load(name: app-reveal, duration: 500ms, delay: 0ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            opacity: 0 -> 1;
            easing: --ease-out-quad;
        }

      }
}


// Header entrance
.header {
    @on &.load(name: header-reveal, duration: 600ms, delay: 100ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        .header-title {
            opacity: 0 -> 1;
            translate-y: -20px -> 0;
            easing: --ease-out-expo;
        }

        .add-button {
            opacity: 0 -> 1;
            scale: 0.8 -> 1;
            easing: --ease-out-back;
            range: 0.3 to 1;
        }

      }
}


// Add button interactions
.add-button {
    @on &.hover(name: add-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 1.1;
            easing: --ease-out-quad;
        }

      }

    @on &.click(name: add-press, duration: 150ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 0.9 -> 1;
            easing: --ease-out-quad;
        }

      }
}


// Task list staggered entrance
.task-list {
    @on &.load(name: tasks-reveal, duration: 800ms, delay: 200ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        .task-row {
            opacity: 0 -> 1;
            translate-y: 30px -> 0;
            easing: --ease-out-expo;
            stagger: 0.08 first;
        }

      }
}


// Task row hover
.task-row {
    @on &.hover(name: task-hover, duration: 250ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            translate-x: 0 -> 4px;
            background-color: var(--st-color-surface) -> var(--st-color-background);
            easing: --ease-out-quad;
        }

      }
}


// Checkbox interactions
.task-checkbox {
    @on &.hover(name: checkbox-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 1.15;
            easing: --ease-out-quad;
        }

      }

    @on &.click(name: checkbox-press, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 0.85 -> 1.1 -> 1;
            easing: --ease-out-back;
        }

        .check-icon {
            scale: 0.5 -> 1.2 -> 1;
            easing: --ease-out-back;
        }

      }
}


// Tab bar entrance
.tab-bar {
    @on &.load(name: tabs-reveal, duration: 500ms, delay: 300ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        .tab-item {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: --ease-out-expo;
            stagger: 0.1 first;
        }

      }
}


// Tab item interactions
.tab-item {
    @on &.hover(name: tab-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            translate-y: 0 -> -3px;
            easing: --ease-out-quad;
        }

        .tab-icon {
            scale: 1 -> 1.15;
            easing: --ease-out-back;
        }

      }

    @on &.click(name: tab-press, duration: 150ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 0.95 -> 1;
            easing: --ease-out-quad;
        }

      }
}


// Toggle switch interactions
.toggle-switch {
    @on &.hover(name: toggle-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 1.05;
            easing: --ease-out-quad;
        }

      }

    @on &.click(name: toggle-press, duration: 250ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        .toggle-knob {
            scale: 1 -> 1.2 -> 1;
            easing: --ease-out-back;
        }

      }
}


// Settings view entrance
.settings-view {
    @on &.load(name: settings-reveal, duration: 600ms, delay: 0ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        .settings-header {
            opacity: 0 -> 1;
            translate-y: -15px -> 0;
            easing: --ease-out-expo;
            range: 0 to 0.4;
        }

        .settings-section {
            opacity: 0 -> 1;
            translate-y: 25px -> 0;
            easing: --ease-out-expo;
            stagger: 0.12 first;
            range: 0.2 to 1;
        }

      }
}


// Settings row hover
.settings-row {
    @on &.hover(name: settings-row-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            translate-x: 0 -> 4px;
            background-color: var(--st-color-surface) -> var(--st-color-background);
            easing: --ease-out-quad;
        }

      }
}


// Empty state entrance
.empty-state {
    @on &.load(name: empty-reveal, duration: 800ms, delay: 400ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        .empty-icon {
            opacity: 0 -> 1;
            scale: 0.5 -> 1.1 -> 1;
            easing: --ease-out-back;
            range: 0 to 0.5;
        }

        .empty-title {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: --ease-out-expo;
            range: 0.2 to 0.7;
        }

        .empty-description {
            opacity: 0 -> 1;
            translate-y: 15px -> 0;
            easing: --ease-out-quad;
            range: 0.4 to 0.9;
        }

      }
}


// Loading spinner
.loading-spinner {
    @on &.loop(name: spinner-rotate, duration: 1s, mode: loop, phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        & {
            rotate: 0deg -> 360deg;
            easing: linear;
        }

      }
}


// Bottom sheet reveal
.task-edit-sheet {
    @on &.load(name: sheet-reveal, duration: 500ms, delay: 0ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            opacity: 0 -> 1;
            translate-y: 100% -> 0;
            easing: --ease-out-expo;
        }

        .sheet-handle {
            opacity: 0 -> 1;
            scale-x: 0 -> 1;
            easing: --ease-out-expo;
            range: 0.3 to 0.8;
        }

      }
}


// Sheet button interactions
.sheet-button {
    @on &.hover(name: sheet-btn-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 1.03;
            easing: --ease-out-quad;
        }

      }

    @on &.click(name: sheet-btn-press, duration: 150ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 0.97 -> 1;
            easing: --ease-out-quad;
        }

      }
}


// Pull indicator
.pull-indicator {
    @on &.load(name: pull-show, duration: 300ms, delay: 0ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            opacity: 0 -> 1;
            scale: 0.8 -> 1;
            easing: --ease-out-back;
        }

      }
}


// Biometric button
.biometric-toggle {
    @on &.hover(name: bio-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 1.02;
            easing: --ease-out-quad;
        }

      }

    @on &.click(name: bio-press, duration: 150ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 0.98 -> 1;
            easing: --ease-out-quad;
        }

      }
}
