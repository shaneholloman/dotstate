//! Application state management.
//!
//! This module provides a cleaner state management approach that eliminates
//! dual state ownership between components and UI state. Each screen owns
//! its own state exclusively.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                    AppState                          │
//! ├─────────────────────────────────────────────────────┤
//! │  ┌───────────────┐    ┌──────────────────────────┐  │
//! │  │ GlobalState   │    │ ScreenState              │  │
//! │  │               │    │ ┌────────────────────┐   │  │
//! │  │ - help_shown  │    │ │ MainMenu(state)   │   │  │
//! │  │ - has_changes │    │ │ GitHubAuth(state) │   │  │
//! │  │ - theme       │    │ │ DotfileSelection  │   │  │
//! │  │               │    │ │ ...               │   │  │
//! │  └───────────────┘    │ └────────────────────┘   │  │
//! │                       └──────────────────────────┘  │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! The `ScreenState` enum uses Rust's enum to ensure only one screen state
//! is active at a time, preventing state synchronization issues.

pub mod global;
pub mod screen;

pub use global::GlobalState;
pub use screen::ScreenState;

/// Current dialog or overlay being shown.
#[derive(Debug, Clone)]
pub enum Dialog {
    /// No dialog shown.
    None,
    /// Simple message dialog.
    Message {
        title: String,
        content: String,
        on_dismiss_screen: super::ui::Screen,
    },
    /// Confirmation dialog.
    Confirm {
        title: String,
        content: String,
        /// Screen to return to on Yes.
        on_yes_screen: super::ui::Screen,
        /// Screen to return to on No.
        on_no_screen: super::ui::Screen,
    },
    /// Help overlay.
    Help,
}

impl Default for Dialog {
    fn default() -> Self {
        Self::None
    }
}

/// Navigation intent returned by screen event handlers.
///
/// This pattern allows screens to signal navigation without directly
/// mutating global state.
#[derive(Debug, Clone)]
pub enum NavigationIntent {
    /// Stay on current screen.
    None,
    /// Navigate to a specific screen.
    Navigate(super::ui::Screen),
    /// Show a dialog.
    ShowDialog(Dialog),
    /// Close current dialog.
    CloseDialog,
    /// Quit the application.
    Quit,
}

impl Default for NavigationIntent {
    fn default() -> Self {
        Self::None
    }
}
