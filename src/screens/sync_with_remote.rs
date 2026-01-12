//! Sync with remote screen controller.
//!
//! This screen handles syncing changes with the remote repository (push/pull).
//! Note: Event handling is still partially in app.rs.

use crate::components::PushChangesComponent;
use crate::config::Config;
use crate::screens::screen_trait::{RenderContext, Screen, ScreenAction, ScreenContext};
use crate::ui::SyncWithRemoteState;
use anyhow::Result;
use crossterm::event::Event;
use ratatui::layout::Rect;
use ratatui::Frame;
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxSet;

/// Sync with remote screen controller.
///
/// # Migration Status
///
/// Event handling is still in app.rs (lines ~942-1000).
/// The screen is mostly presentation - the sync logic is in GitService.
pub struct SyncWithRemoteScreen {
    component: PushChangesComponent,
    /// State is owned here but synced with ui_state during transition
    pub state: SyncWithRemoteState,
}

impl SyncWithRemoteScreen {
    /// Create a new sync with remote screen.
    pub fn new() -> Self {
        Self {
            component: PushChangesComponent::new(),
            state: SyncWithRemoteState::default(),
        }
    }

    /// Render with all required context.
    pub fn render_with_context(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        config: &Config,
        syntax_set: &SyntaxSet,
        theme: &Theme,
    ) -> Result<()> {
        self.component.render_with_state(
            frame,
            area,
            &mut self.state,
            config,
            syntax_set,
            theme,
        )
    }

    /// Get a reference to the state.
    pub fn get_state(&self) -> &SyncWithRemoteState {
        &self.state
    }

    /// Get a mutable reference to the state.
    pub fn get_state_mut(&mut self) -> &mut SyncWithRemoteState {
        &mut self.state
    }

    /// Reset state to default.
    pub fn reset_state(&mut self) {
        self.state = SyncWithRemoteState::default();
    }
}

impl Default for SyncWithRemoteScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for SyncWithRemoteScreen {
    fn render(&mut self, _frame: &mut Frame, _area: Rect, _ctx: &RenderContext) -> Result<()> {
        // Note: Use render_with_context instead as this screen needs syntax highlighting
        Ok(())
    }

    fn handle_event(&mut self, _event: Event, _ctx: &ScreenContext) -> Result<ScreenAction> {
        // TODO: Move event handling from app.rs here
        // Currently handled in app.rs (handle_event, Screen::SyncWithRemote case)
        Ok(ScreenAction::None)
    }

    fn is_input_focused(&self) -> bool {
        false // No text inputs on this screen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_with_remote_screen_creation() {
        let screen = SyncWithRemoteScreen::new();
        assert!(!screen.is_input_focused());
        assert!(screen.state.changed_files.is_empty());
    }

    #[test]
    fn test_reset_state() {
        let mut screen = SyncWithRemoteScreen::new();
        screen.state.is_syncing = true;
        screen.reset_state();
        assert!(!screen.state.is_syncing);
    }
}
