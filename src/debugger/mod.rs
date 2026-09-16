//! Visual Debugger Panel for Spacetime
//!
//! Provides runtime instrumentation and a visual debugging UI for inspecting:
//! - Timeline progress (0-100% for each timeline)
//! - Signal values with change highlighting
//! - State machine states with visual indication
//! - Event log with scrollable history
//!
//! # Usage
//!
//! Enable the debugger by passing `--debug` to the serve command:
//!
//! ```sh
//! spacetime serve --debug projects/<name>/
//! ```
//!
//! The debugger panel will be available at `/__spacetime__/debugger` and
//! the runtime will be instrumented to track state changes.

pub mod panel;
pub mod runtime;

pub use panel::generate_debug_panel;
pub use runtime::generate_debug_hooks;
pub use runtime::generate_debug_runtime;

use serde::{Deserialize, Serialize};

/// Configuration for the visual debugger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggerConfig {
    /// Whether the debugger is enabled
    pub enabled: bool,
    /// Position of the debug panel on screen
    pub panel_position: PanelPosition,
    /// Whether to record state change history
    pub record_history: bool,
    /// Maximum history entries to keep (0 = unlimited)
    pub max_history_entries: usize,
    /// Whether to show the panel overlay or just instrument
    pub show_panel: bool,
}

impl Default for DebuggerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            panel_position: PanelPosition::BottomRight,
            record_history: true,
            max_history_entries: 1000,
            show_panel: true,
        }
    }
}

impl DebuggerConfig {
    /// Create a new debugger config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable the debugger
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the panel position
    pub fn position(mut self, position: PanelPosition) -> Self {
        self.panel_position = position;
        self
    }

    /// Enable or disable history recording
    pub fn record_history(mut self, record: bool) -> Self {
        self.record_history = record;
        self
    }

    /// Set the maximum number of history entries
    pub fn max_history(mut self, max: usize) -> Self {
        self.max_history_entries = max;
        self
    }

    /// Enable or disable the panel overlay
    pub fn show_panel(mut self, show: bool) -> Self {
        self.show_panel = show;
        self
    }
}

/// Position of the debug panel on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PanelPosition {
    /// Bottom-right corner (default)
    #[default]
    BottomRight,
    /// Bottom-left corner
    BottomLeft,
    /// Top-right corner
    TopRight,
    /// Top-left corner
    TopLeft,
}

impl PanelPosition {
    /// Get CSS positioning for the panel
    pub fn css_position(&self) -> &'static str {
        match self {
            PanelPosition::BottomRight => "bottom: 16px; right: 16px;",
            PanelPosition::BottomLeft => "bottom: 16px; left: 16px;",
            PanelPosition::TopRight => "top: 16px; right: 16px;",
            PanelPosition::TopLeft => "top: 16px; left: 16px;",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_config_defaults() {
        let config = DebuggerConfig::default();
        assert!(config.enabled);
        assert!(config.record_history);
        assert!(config.show_panel);
        assert_eq!(config.panel_position, PanelPosition::BottomRight);
        assert_eq!(config.max_history_entries, 1000);
    }

    #[test]
    fn test_debugger_config_builder() {
        let config = DebuggerConfig::new()
            .enabled(false)
            .position(PanelPosition::TopLeft)
            .record_history(false)
            .max_history(500)
            .show_panel(false);

        assert!(!config.enabled);
        assert!(!config.record_history);
        assert!(!config.show_panel);
        assert_eq!(config.panel_position, PanelPosition::TopLeft);
        assert_eq!(config.max_history_entries, 500);
    }

    #[test]
    fn test_panel_position_css() {
        assert_eq!(
            PanelPosition::BottomRight.css_position(),
            "bottom: 16px; right: 16px;"
        );
        assert_eq!(
            PanelPosition::TopLeft.css_position(),
            "top: 16px; left: 16px;"
        );
    }
}
