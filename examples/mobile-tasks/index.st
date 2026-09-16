// ============================================
// SPACETIME TASKS - Mobile Demo App
// ============================================
//
// A task management app showcasing:
// - Modular imports
// - Reusable patterns
// - Reactive $-signals
// - Smooth animations
//

@version 2026-07-26;

@import "./patterns.st"


// ============================================
// REACTIVE SHELL STATE
// ============================================

$view string: "tasks";
$activeTab string: "tasks";
$sheetOpen bool: false;

.task-list-view {
    [hidden]: $view != "tasks";
}

.settings-view {
    [hidden]: $view != "settings";
}

.tab-item.is-active {
    color: var(--st-color-primary)
}


// ============================================
// APP SHELL
// ============================================

.app {
    .is-settings: $view == "settings";

    @fade-in-up(duration: 500ms)
}


// ============================================
// HEADER
// ============================================

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

.add-button {
    @hover-scale(scale: 1.1)
    @press-feedback(duration: 150ms)

    @on &.click { $sheetOpen <- true; }
}


// ============================================
// TASK LIST
// ============================================

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

.task-row {
    @swipe-delete()
    @row-hover(distance: 4px)
}

.task-checkbox {
    @checkbox()
    @hover-scale(scale: 1.15)

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


// ============================================
// TAB BAR
// ============================================

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

.tab-item {
    @hover-lift(distance: -3px)
    @press-feedback()

    @on &.hover(name: tab-icon-scale, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        .tab-icon {
            scale: 1 -> 1.15;
            easing: --ease-out-back;
        }

      }
}

.tab-item:first-of-type {
    .is-active: $activeTab == "tasks";

    @on &.click { $activeTab <- "tasks"; $view <- "tasks"; }
}

.tab-item:last-of-type {
    .is-active: $activeTab == "settings";

    @on &.click { $activeTab <- "settings"; $view <- "settings"; }
}


// ============================================
// TOGGLE SWITCHES
// ============================================

.toggle-switch {
    @toggle-switch()
    @hover-scale(scale: 1.05)

    @on &.click(name: toggle-knob-bounce, duration: 250ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        .toggle-knob {
            scale: 1 -> 1.2 -> 1;
            easing: --ease-out-back;
        }

      }
}


// ============================================
// SETTINGS VIEW
// ============================================

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

.settings-row {
    @row-hover(distance: 4px)
}


// ============================================
// EMPTY STATE
// ============================================

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


// ============================================
// LOADING SPINNER
// ============================================

.loading-spinner {
    @spin(duration: 1s)
}


// ============================================
// BOTTOM SHEET
// ============================================

.task-edit-sheet {
    .is-open: $sheetOpen;

    @on &.load(name: sheet-reveal, duration: 500ms, delay: 0ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        .sheet-handle {
            opacity: 0 -> 1;
            scale-x: 0 -> 1;
            easing: --ease-out-expo;
            range: 0.3 to 0.8;
        }

      }
}

.sheet-button {
    @hover-scale(scale: 1.03)
    @press-feedback()

    @on &.click { $sheetOpen <- false; }
}


// ============================================
// PULL TO REFRESH
// ============================================

.pull-indicator {
    @bounce-in(duration: 300ms)
}


// ============================================
// BIOMETRIC BUTTON
// ============================================

.biometric-toggle {
    @hover-scale(scale: 1.02)
    @press-feedback()
}
