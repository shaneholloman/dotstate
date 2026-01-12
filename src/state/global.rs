//! Global application state shared across all screens.

use super::Dialog;

/// Global state that persists across screen changes.
///
/// This state contains information that is relevant across all screens
/// and shouldn't be reset when navigating between screens.
#[derive(Debug, Clone)]
pub struct GlobalState {
    /// Whether there are uncommitted or unpushed changes in the repository.
    pub has_changes_to_push: bool,

    /// Currently active dialog or overlay.
    pub dialog: Dialog,

    /// Whether a text input is currently focused.
    /// When true, keymap navigation is disabled so users can type freely.
    pub input_mode_active: bool,

    /// Whether the help overlay is visible.
    pub show_help_overlay: bool,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            has_changes_to_push: false,
            dialog: Dialog::None,
            input_mode_active: false,
            show_help_overlay: false,
        }
    }
}

impl GlobalState {
    /// Create a new global state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any dialog or overlay is shown.
    pub fn has_dialog(&self) -> bool {
        !matches!(self.dialog, Dialog::None)
    }

    /// Show a message dialog.
    pub fn show_message(&mut self, title: String, content: String, on_dismiss: crate::ui::Screen) {
        self.dialog = Dialog::Message {
            title,
            content,
            on_dismiss_screen: on_dismiss,
        };
    }

    /// Show a confirmation dialog.
    pub fn show_confirm(
        &mut self,
        title: String,
        content: String,
        on_yes: crate::ui::Screen,
        on_no: crate::ui::Screen,
    ) {
        self.dialog = Dialog::Confirm {
            title,
            content,
            on_yes_screen: on_yes,
            on_no_screen: on_no,
        };
    }

    /// Close any open dialog.
    pub fn close_dialog(&mut self) {
        self.dialog = Dialog::None;
    }

    /// Toggle the help overlay.
    pub fn toggle_help(&mut self) {
        self.show_help_overlay = !self.show_help_overlay;
    }
}
