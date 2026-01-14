//! View synced files screen controller.
//!
//! This screen displays the list of files currently synced in the active profile.

use crate::components::footer::Footer;
use crate::components::header::Header;
use crate::config::Config;
use crate::screens::screen_trait::{RenderContext, Screen, ScreenAction, ScreenContext};
use crate::styles::{theme, LIST_HIGHLIGHT_SYMBOL};
use crate::ui::Screen as ScreenId;
use crate::utils::{create_standard_layout, focused_border_style, get_home_dir};
use anyhow::Result;
use crossterm::event::{Event, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};

/// View synced files screen controller.
pub struct ViewSyncedFilesScreen {
    config: Config,
    list_state: ListState,
    scrollbar_state: ScrollbarState,
}

impl ViewSyncedFilesScreen {
    /// Create a new view synced files screen.
    pub fn new(config: Config) -> Self {
        // Get synced files from manifest
        let synced_files = Self::get_synced_files(&config);
        let file_count = synced_files.len();
        let mut list_state = ListState::default();
        if !synced_files.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            config,
            list_state,
            scrollbar_state: ScrollbarState::new(file_count.saturating_sub(1)),
        }
    }

    /// Update configuration.
    pub fn update_config(&mut self, config: Config) {
        let synced_files = Self::get_synced_files(&self.config);
        let was_empty = synced_files.is_empty();
        self.config = config;
        let new_synced_files = Self::get_synced_files(&self.config);
        if was_empty && !new_synced_files.is_empty() {
            self.list_state.select(Some(0));
        }
        self.scrollbar_state = ScrollbarState::new(new_synced_files.len().saturating_sub(1));
    }

    /// Get synced files from manifest for the active profile
    fn get_synced_files(config: &Config) -> Vec<String> {
        crate::utils::ProfileManifest::load_or_backfill(&config.repo_path)
            .ok()
            .and_then(|manifest| {
                manifest
                    .profiles
                    .iter()
                    .find(|p| p.name == config.active_profile)
                    .map(|p| p.synced_files.clone())
            })
            .unwrap_or_default()
    }

    /// Render the screen internally
    fn render_internal(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Clear the entire area first
        frame.render_widget(Clear, area);

        // Background - use Reset to inherit terminal's native background
        let background = Block::default().style(Style::default().bg(Color::Reset));
        frame.render_widget(background, area);

        let (header_chunk, content_chunk, footer_chunk) = create_standard_layout(area, 5, 2);

        // Header: Use common header component
        let _ = Header::render(
            frame,
            header_chunk,
            "DotState - View Synced Files",
            "These are the files currently synced to your repository. Files are stored in the repo and symlinked back to their original locations."
        )?;

        // Get synced files from manifest
        let synced_files = Self::get_synced_files(&self.config);

        let t = theme();
        // Content: List of synced files
        if synced_files.is_empty() {
            let empty_message = Paragraph::new(
                "No files are currently synced.\n\nGo to 'Scan & Select Dotfiles' to start syncing your dotfiles."
            )
                .style(t.muted_style())
                .wrap(Wrap { trim: true })
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Synced Files")
                    .title_alignment(Alignment::Center)
                    .padding(ratatui::widgets::Padding::new(2, 2, 2, 2)));
            frame.render_widget(empty_message, content_chunk);
        } else {
            let items: Vec<ListItem> = synced_files
                .iter()
                .enumerate()
                .map(|(i, path)| {
                    // Check if file exists and is a symlink
                    let home_dir = get_home_dir();
                    let full_path = home_dir.join(path);
                    let status = if !full_path.exists() {
                        "⚠ Missing"
                    } else if std::fs::symlink_metadata(&full_path)
                        .map(|m| m.file_type().is_symlink())
                        .unwrap_or(false)
                    {
                        "✓ Synced"
                    } else {
                        "⚠ Not symlinked"
                    };

                    let style = if self.list_state.selected() == Some(i) {
                        t.highlight_style()
                    } else {
                        t.text_style()
                    };

                    ListItem::new(format!("{} {}", status, path)).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(focused_border_style())
                        .title(format!("Synced Files ({})", synced_files.len()))
                        .title_alignment(Alignment::Center)
                        .padding(ratatui::widgets::Padding::new(1, 1, 1, 1)),
                )
                .highlight_style(t.highlight_style())
                .highlight_symbol(LIST_HIGHLIGHT_SYMBOL);

            frame.render_stateful_widget(list, content_chunk, &mut self.list_state);

            // Update scrollbar state
            if let Some(selected) = self.list_state.selected() {
                self.scrollbar_state = self.scrollbar_state.position(selected);
            }

            // Render scrollbar
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            frame.render_stateful_widget(scrollbar, content_chunk, &mut self.scrollbar_state);
        }

        // Footer
        let _ = Footer::render(frame, footer_chunk, "q/Esc/Click: Back to Main Menu")?;

        Ok(())
    }
}

impl Screen for ViewSyncedFilesScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &RenderContext) -> Result<()> {
        self.render_internal(frame, area)
    }

    fn handle_event(&mut self, event: Event, ctx: &ScreenContext) -> Result<ScreenAction> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                if let Some(action) = ctx.config.keymap.get_action(key.code, key.modifiers) {
                    use crate::keymap::Action;
                    match action {
                        Action::Quit | Action::Cancel => {
                            return Ok(ScreenAction::Navigate(ScreenId::MainMenu));
                        }
                        Action::MoveUp => {
                            let synced_files = Self::get_synced_files(&self.config);
                            if !synced_files.is_empty() {
                                self.list_state.select_previous();
                            }
                            return Ok(ScreenAction::None);
                        }
                        Action::MoveDown => {
                            let synced_files = Self::get_synced_files(&self.config);
                            if !synced_files.is_empty() {
                                self.list_state.select_next();
                            }
                            return Ok(ScreenAction::None);
                        }
                        _ => {}
                    }
                }
            }
        }

        match event {
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // Click anywhere to go back (simple for now)
                    Ok(ScreenAction::Navigate(ScreenId::MainMenu))
                }
                MouseEventKind::ScrollUp => {
                    let synced_files = Self::get_synced_files(&self.config);
                    if !synced_files.is_empty() {
                        self.list_state.select_previous();
                    }
                    Ok(ScreenAction::None)
                }
                MouseEventKind::ScrollDown => {
                    let synced_files = Self::get_synced_files(&self.config);
                    if !synced_files.is_empty() {
                        self.list_state.select_next();
                    }
                    Ok(ScreenAction::None)
                }
                _ => Ok(ScreenAction::None),
            },
            _ => Ok(ScreenAction::None),
        }
    }

    fn is_input_focused(&self) -> bool {
        false // This screen has no text inputs
    }

    fn on_enter(&mut self, ctx: &ScreenContext) -> Result<()> {
        // Refresh the synced files list
        self.update_config(ctx.config.clone());
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
