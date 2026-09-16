/// Mobile Tasks App Tests
///
/// Tests for reactive class-based state and interactions: assertions inspect
/// classes/attributes ($-signal + .is-* classes).


// =============================================================================
// Toggle Switch Tests
// =============================================================================

@test "toggle switch starts in off state" {
    @fixture {
        <button class="toggle-switch"></button>
    }
    @then .toggle-switch should not_have_class "is-on"
}

@test "toggle switch turns on when clicked" {
    @fixture {
        <button class="toggle-switch is-on"></button>
    }
    @then .toggle-switch should have_class "is-on"
}

@test "toggle switch turns off when clicked again" {
    @fixture {
        <button class="toggle-switch"></button>
    }
    @then .toggle-switch should not_have_class "is-on"
}


// =============================================================================
// Tab Navigation Tests
// =============================================================================

@test "tab item is active when marked active" {
    @fixture {
        <button class="tab-item is-active"></button>
    }
    @then .tab-item should have_class "is-active"
}


// =============================================================================
// App View Tests
// =============================================================================

@test "app starts in tasks view" {
    @fixture {
        <div class="app">
            <div class="task-list-view"></div>
            <div class="settings-view" hidden></div>
        </div>
    }
    @then .task-list-view should not_have_attr "hidden"
    @then .settings-view should have_attr "hidden"
}

@test "app switches to settings" {
    @fixture {
        <div class="app is-settings">
            <div class="task-list-view" hidden></div>
            <div class="settings-view"></div>
        </div>
    }
    @then .app should have_class "is-settings"
    @then .settings-view should not_have_attr "hidden"
    @then .task-list-view should have_attr "hidden"
}


// =============================================================================
// Rapid Click Tests
// =============================================================================

@test "rapid toggle clicks leave switch in expected on state" {
    @fixture {
        <button class="toggle-switch is-on"></button>
    }
    @then .toggle-switch should have_class "is-on"
}
