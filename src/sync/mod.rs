//! Dev Edit Module
//!
//! Content editing protocol for the Spacetime dev tools system.
//! Handles live editing of JSON data files, HTML content, and AST patches
//! via WebSocket messages from browser-based dev tools.

pub mod handlers;
pub mod protocol;
pub mod richtext;

pub use handlers::{HandleResult, handle_message};
pub use protocol::{ClientMessage, ServerMessage};
