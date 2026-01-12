//! Screen trait and associated types.
//!
//! This module defines the `Screen` trait which provides a cleaner alternative
//! to the existing `Component` trait. The key differences are:
//!
//! 1. Screens own their state instead of receiving it from outside
//! 2. Event handling returns an action instead of mutating external state
//! 3. Context objects provide read-only access to shared resources

use crate::config::Config;
use crate::ui::Screen as ScreenId;
use anyhow::Result;
use crossterm::event::Event;
use ratatui::Frame;
use ratatui::layout::Rect;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

/// Context provided for rendering screens.
///
/// This struct provides read-only access to resources needed for rendering,
/// such as syntax highlighting themes and configuration.
pub struct RenderContext<'a> {
    /// Application configuration.
    pub config: &'a Config,
    /// Syntax highlighting syntax set.
    pub syntax_set: &'a SyntaxSet,
    /// Syntax highlighting theme set.
    pub theme_set: &'a ThemeSet,
    /// Current syntax theme.
    pub syntax_theme: &'a Theme,
}

impl<'a> RenderContext<'a> {
    /// Create a new render context.
    pub fn new(
        config: &'a Config,
        syntax_set: &'a SyntaxSet,
        theme_set: &'a ThemeSet,
        syntax_theme: &'a Theme,
    ) -> Self {
        Self {
            config,
            syntax_set,
            theme_set,
            syntax_theme,
        }
    }
}

/// Context provided for handling events.
///
/// This struct provides read access to configuration and other resources
/// needed for event handling.
pub struct ScreenContext<'a> {
    /// Application configuration.
    pub config: &'a Config,
    /// Configuration file path (for saving).
    pub config_path: &'a std::path::Path,
    /// Repository path.
    pub repo_path: &'a std::path::Path,
    /// Active profile name.
    pub active_profile: &'a str,
}

impl<'a> ScreenContext<'a> {
    /// Create a new screen context.
    pub fn new(config: &'a Config, config_path: &'a std::path::Path) -> Self {
        Self {
            config,
            config_path,
            repo_path: &config.repo_path,
            active_profile: &config.active_profile,
        }
    }
}

/// Actions that a screen can return after handling an event.
///
/// This enum allows screens to signal navigation and state changes without
/// directly mutating global state.
#[derive(Debug, Clone)]
pub enum ScreenAction {
    /// No action needed, stay on current screen.
    None,
    /// Navigate to a different screen.
    Navigate(ScreenId),
    /// Navigate to a screen and pass data.
    NavigateWithMessage {
        screen: ScreenId,
        title: String,
        message: String,
    },
    /// Show a message popup.
    ShowMessage {
        title: String,
        content: String,
    },
    /// Request to quit the application.
    Quit,
    /// Trigger a data refresh (e.g., reload dotfiles).
    Refresh,
    /// Mark that there are changes to push.
    SetHasChanges(bool),
    /// Update the config (signals app to reload).
    ConfigUpdated,
    /// Open help overlay.
    ShowHelp,
}

impl Default for ScreenAction {
    fn default() -> Self {
        Self::None
    }
}

/// Trait for screen controllers.
///
/// This trait provides a cleaner alternative to the existing `Component` trait.
/// Screens that implement this trait own their state and handle both rendering
/// and events in a self-contained way.
///
/// # Example
///
/// ```rust,ignore
/// struct MyScreen {
///     state: MyState,
/// }
///
/// impl Screen for MyScreen {
///     fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &RenderContext) -> Result<()> {
///         // Draw widgets
///         Ok(())
///     }
///
///     fn handle_event(&mut self, event: Event, ctx: &ScreenContext) -> Result<ScreenAction> {
///         match event {
///             Event::Key(key) => {
///                 // Handle key press
///                 Ok(ScreenAction::Navigate(Screen::MainMenu))
///             }
///             _ => Ok(ScreenAction::None),
///         }
///     }
/// }
/// ```
pub trait Screen {
    /// Render the screen.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to render to.
    /// * `area` - The area to render within.
    /// * `ctx` - Render context with shared resources.
    fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &RenderContext) -> Result<()>;

    /// Handle an input event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle.
    /// * `ctx` - Screen context with configuration.
    ///
    /// # Returns
    ///
    /// An action indicating what should happen next.
    fn handle_event(&mut self, event: Event, ctx: &ScreenContext) -> Result<ScreenAction>;

    /// Check if a text input is currently focused.
    ///
    /// When true, navigation keybindings are disabled so users can type freely.
    fn is_input_focused(&self) -> bool {
        false
    }

    /// Called when the screen is entered (navigated to).
    ///
    /// This is useful for initializing state that depends on current config.
    fn on_enter(&mut self, _ctx: &ScreenContext) -> Result<()> {
        Ok(())
    }

    /// Called when the screen is exited (navigated away from).
    ///
    /// This is useful for cleanup or saving state.
    fn on_exit(&mut self, _ctx: &ScreenContext) -> Result<()> {
        Ok(())
    }
}
