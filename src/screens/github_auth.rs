//! GitHub authentication screen controller.
//!
//! This screen handles the GitHub authentication and repository setup flow.
//! Note: Complex event handling remains in app.rs for now (see handle_github_auth_input).
//! This is a wrapper screen that will gradually take over event handling.

use crate::components::component::Component;
use crate::components::{ComponentAction, GitHubAuthComponent};
use crate::config::Config;
use crate::screens::screen_trait::{RenderContext, Screen, ScreenAction, ScreenContext};
use crate::ui::GitHubAuthState;
use anyhow::Result;
use crossterm::event::Event;
use ratatui::layout::Rect;
use ratatui::Frame;

/// GitHub authentication screen controller.
///
/// # Migration Status
///
/// This screen wraps `GitHubAuthComponent` but event handling is still mostly
/// in `app.rs` (see `handle_github_auth_input`, `handle_local_setup_input`,
/// `update_github_token`, `process_github_setup_step`).
///
/// **To complete migration:**
/// 1. Move `handle_github_auth_input` logic into `handle_event`
/// 2. Move `handle_local_setup_input` logic into `handle_event`
/// 3. Move `update_github_token` into a method here or GitService
/// 4. Move `process_github_setup_step` into an async handler
pub struct GitHubAuthScreen {
    component: GitHubAuthComponent,
}

impl GitHubAuthScreen {
    /// Create a new GitHub auth screen.
    pub fn new() -> Self {
        Self {
            component: GitHubAuthComponent::new(),
        }
    }

    /// Update configuration.
    pub fn update_config(&mut self, config: Config) {
        self.component.update_config(config);
    }

    /// Get the current auth state.
    pub fn get_auth_state(&self) -> &GitHubAuthState {
        self.component.get_auth_state()
    }

    /// Get mutable auth state.
    pub fn get_auth_state_mut(&mut self) -> &mut GitHubAuthState {
        self.component.get_auth_state_mut()
    }

    /// Render the screen.
    pub fn render_frame(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.component.render(frame, area)
    }

    /// Handle mouse events (delegates to component).
    pub fn handle_mouse_event(&mut self, event: Event) -> Result<ComponentAction> {
        self.component.handle_event(event)
    }
}

impl Default for GitHubAuthScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for GitHubAuthScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &RenderContext) -> Result<()> {
        self.component.render(frame, area)
    }

    fn handle_event(&mut self, event: Event, _ctx: &ScreenContext) -> Result<ScreenAction> {
        // NOTE: Complex keyboard event handling is still in app.rs (handle_github_auth_input)
        // This method only handles mouse events for now.
        // See migration notes in struct docs for what needs to be moved here.

        if matches!(event, Event::Mouse(_)) {
            let action = self.component.handle_event(event)?;
            if action == ComponentAction::Update {
                // State was updated, app.rs handles syncing
                return Ok(ScreenAction::Refresh);
            }
        }

        Ok(ScreenAction::None)
    }

    fn is_input_focused(&self) -> bool {
        // Check if any input field is focused
        self.component.get_auth_state().input_focused
    }

    fn on_enter(&mut self, _ctx: &ScreenContext) -> Result<()> {
        // Reset state when entering the screen
        // App.rs handles the actual state initialization
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_auth_screen_creation() {
        let screen = GitHubAuthScreen::new();
        // Default state has input_focused = true
        assert!(screen.is_input_focused());
    }

    #[test]
    fn test_get_auth_state() {
        let screen = GitHubAuthScreen::new();
        let state = screen.get_auth_state();
        // Default state should be in Choosing mode
        assert_eq!(state.setup_mode, crate::ui::SetupMode::Choosing);
    }
}
