//! View synced files screen controller.
//!
//! This screen displays the list of files currently synced in the active profile.
//! This is one of the simpler screens - the component already handles all events.

use crate::components::component::Component;
use crate::components::{ComponentAction, SyncedFilesComponent};
use crate::config::Config;
use crate::screens::screen_trait::{RenderContext, Screen, ScreenAction, ScreenContext};
use crate::ui::Screen as ScreenId;
use anyhow::Result;
use crossterm::event::Event;
use ratatui::layout::Rect;
use ratatui::Frame;

/// View synced files screen controller.
///
/// This screen is fully self-contained - the component handles all
/// events and returns navigation actions.
pub struct ViewSyncedFilesScreen {
    component: SyncedFilesComponent,
}

impl ViewSyncedFilesScreen {
    /// Create a new view synced files screen.
    pub fn new(config: Config) -> Self {
        Self {
            component: SyncedFilesComponent::new(config),
        }
    }

    /// Update configuration.
    pub fn update_config(&mut self, config: Config) {
        self.component.update_config(config);
    }

    /// Render the screen.
    pub fn render_frame(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.component.render(frame, area)
    }

    /// Handle events and return an action.
    pub fn handle_event_action(&mut self, event: Event) -> Result<ScreenAction> {
        let action = self.component.handle_event(event)?;
        match action {
            ComponentAction::Navigate(ScreenId::MainMenu) => {
                Ok(ScreenAction::Navigate(ScreenId::MainMenu))
            }
            _ => Ok(ScreenAction::None),
        }
    }
}

impl Screen for ViewSyncedFilesScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &RenderContext) -> Result<()> {
        self.component.render(frame, area)
    }

    fn handle_event(&mut self, event: Event, _ctx: &ScreenContext) -> Result<ScreenAction> {
        self.handle_event_action(event)
    }

    fn is_input_focused(&self) -> bool {
        false // This screen has no text inputs
    }

    fn on_enter(&mut self, ctx: &ScreenContext) -> Result<()> {
        // Refresh the synced files list
        self.component.update_config(ctx.config.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_config() -> Config {
        let mut config = Config::default();
        config.repo_path = PathBuf::from("/tmp/test-repo");
        config
    }

    #[test]
    fn test_view_synced_files_screen_creation() {
        let config = test_config();
        let screen = ViewSyncedFilesScreen::new(config);
        assert!(!screen.is_input_focused());
    }
}
