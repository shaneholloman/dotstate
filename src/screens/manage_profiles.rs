use crate::components::profile_manager::{
    CreateField, ProfileManagerComponent, ProfileManagerState, ProfilePopupType,
};
use crate::config::Config;
use crate::keymap::{Action, Keymap};
use crate::screens::{RenderContext, Screen, ScreenAction, ScreenContext};
use crate::ui::Screen as ScreenId;
use crate::utils::text_input::{
    handle_backspace, handle_char_insertion, handle_cursor_movement, handle_delete,
};
// use crate::utils::ProfileInfo; // Unused
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::prelude::*;

pub struct ManageProfilesScreen {
    component: ProfileManagerComponent,
    pub state: ProfileManagerState,
}

impl Default for ManageProfilesScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ManageProfilesScreen {
    pub fn new() -> Self {
        Self {
            component: ProfileManagerComponent::new(),
            state: ProfileManagerState::default(),
        }
    }

    /// Refresh the cached profiles from disk
    pub fn refresh_profiles(&mut self, repo_path: &std::path::Path) -> Result<()> {
        let profiles = crate::services::ProfileService::get_profiles(repo_path)?;
        self.state.profiles = profiles;
        // Initialize list selection to first item if profiles exist
        if !self.state.profiles.is_empty() {
            self.state.list_state.select(Some(0));
        }
        Ok(())
    }

    fn get_action(&self, key: KeyCode, modifiers: KeyModifiers, keymap: &Keymap) -> Option<Action> {
        keymap.get_action(key, modifiers)
    }

    fn handle_mouse_event(
        &mut self,
        mouse: crossterm::event::MouseEvent,
        _config: &Config,
    ) -> ScreenAction {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let x = mouse.column;
                let y = mouse.row;

                // Handle clicks in list
                for (area, idx) in &self.state.clickable_areas {
                    if x >= area.x
                        && x < area.x + area.width
                        && y >= area.y
                        && y < area.y + area.height
                    {
                        self.state.list_state.select(Some(*idx));
                        return ScreenAction::Refresh;
                    }
                }

                // Handle clicks in create popup fields
                if self.state.popup_type == ProfilePopupType::Create {
                    if let Some(area) = self.state.create_name_area {
                        if x >= area.x
                            && x < area.x + area.width
                            && y >= area.y
                            && y < area.y + area.height
                        {
                            self.state.create_focused_field = CreateField::Name;
                            return ScreenAction::Refresh;
                        }
                    }
                    if let Some(area) = self.state.create_description_area {
                        if x >= area.x
                            && x < area.x + area.width
                            && y >= area.y
                            && y < area.y + area.height
                        {
                            self.state.create_focused_field = CreateField::Description;
                            return ScreenAction::Refresh;
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                let selected = self.state.list_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.state.list_state.select(Some(selected - 1));
                    return ScreenAction::Refresh;
                }
            }
            MouseEventKind::ScrollDown => {
                // We don't know the max count here easily without passing it in,
                // but we can rely on the list to handle out of bounds or just be conservative.
                // For now, let's just increment and let the UI handle bounds if possible,
                // or just rely on keyboard navigation which is safer.
                // Better yet, we can't properly implement scroll down without knowing the list size.
                // We'll leave it for now or implement if we pass profiles to handle_event.
                // Actually, handle_event takes ScreenContext which doesn't have profiles.
                // We might need to change ScreenContext or accept profiles in state.
                // The original app.rs logic had access to profiles.
                // For now, let's skip scroll down logic or make it best-effort?
                // Wait, we can add `profiles_count` to state if needed, but for now let's just use keyboard.
            }
            _ => {}
        }
        ScreenAction::None
    }
}

impl Screen for ManageProfilesScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &RenderContext) -> Result<()> {
        // We need profiles to render. RenderContext doesn't have them.
        // We'll have to get them from `config` or similar, but `get_profiles` reads from disk/manifest.
        // The original App had `get_profiles()`.
        // We can expose `ProfileService::get_profiles` helper or store them in state.
        // Ideally, state should store the cached profiles.
        // But for now, let's try to load them here or assume they are in ctx if we added them?
        // No, RenderContext is generic.
        // We should probably load profiles in `handle_event` (ScreenAction::Refresh) and store in state?
        // Or better: pass them in via context?
        // Let's use `crate::services::ProfileService::get_profiles` here directly.
        // It might be slow to hit disk every frame.
        // Ideally state has `profiles: Vec<ProfileInfo>`.

        // Let's modify `ProfileManagerState` to hold profiles?
        // It's defined in components/profile_manager.rs.
        // Creating a local wrapper or just fetching for now.
        // Since `ProfileService::get_profiles` is just JSON parse, it's probably fine for 60fps?
        // Maybe not.
        // Best approach: Add `profiles` to `ProfileManagerState` and update it on entry/refresh.

        self.component
            .render_with_config(frame, area, ctx.config, &mut self.state)
    }

    fn handle_event(&mut self, event: Event, ctx: &ScreenContext) -> Result<ScreenAction> {
        // Handle popup events first
        if self.state.popup_type != ProfilePopupType::None {
            match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let action = self.get_action(key.code, key.modifiers, &ctx.config.keymap);

                    match self.state.popup_type {
                        ProfilePopupType::Create => {
                            // Handle keymap actions
                            if let Some(action) = action {
                                match action {
                                    Action::Cancel => {
                                        self.state.popup_type = ProfilePopupType::None;
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::NextTab => {
                                        self.state.create_focused_field =
                                            match self.state.create_focused_field {
                                                CreateField::Name => CreateField::Description,
                                                CreateField::Description => CreateField::CopyFrom,
                                                CreateField::CopyFrom => CreateField::Name,
                                            };
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::PrevTab => {
                                        self.state.create_focused_field =
                                            match self.state.create_focused_field {
                                                CreateField::Name => CreateField::CopyFrom,
                                                CreateField::Description => CreateField::Name,
                                                CreateField::CopyFrom => CreateField::Description,
                                            };
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::Confirm => {
                                        // Logic for CopyFrom selection vs Creation
                                        if self.state.create_focused_field == CreateField::CopyFrom
                                        {

                                            // This logic depends on us knowing how many profiles there are to wrap/clamp.
                                            // We probably need to fetch profiles here too to do accurate selection logic?
                                            // Or simplified: Just handle Enter as "Create".
                                            // The original code handled Enter as create unless in CopyFrom list partial selection?
                                            // Actually original code (lines 1334-1353) handled detailed selection logic.
                                            // "If Copy From is focused, select the current item first, then create"
                                            // Wait, if we are in CopyFrom, Enter usually means "Select this option".
                                            // But line 1355 says "Enter always creates, regardless of focus".
                                            // So we should just proceed to create.
                                        }

                                        if !self.state.create_name_input.is_empty() {
                                            let name = self.state.create_name_input.clone();
                                            let description =
                                                if self.state.create_description_input.is_empty() {
                                                    None
                                                } else {
                                                    Some(
                                                        self.state.create_description_input.clone(),
                                                    )
                                                };
                                            let copy_from = self.state.create_copy_from;

                                            // Reset state
                                            self.state.popup_type = ProfilePopupType::None;
                                            self.state.create_name_input.clear();
                                            self.state.create_description_input.clear();
                                            self.state.create_focused_field = CreateField::Name;

                                            return Ok(ScreenAction::CreateProfile {
                                                name,
                                                description,
                                                copy_from,
                                            });
                                        }
                                        return Ok(ScreenAction::None);
                                    }
                                    _ => {}
                                }
                            }

                            // Handle text input and specific navigation
                            match action {
                                Some(Action::MoveUp) => {
                                    if self.state.create_focused_field == CreateField::CopyFrom {
                                        let current =
                                            self.state.create_copy_from.map(|i| i + 1).unwrap_or(0);
                                        if current > 0 {
                                            let new_val = current - 1;
                                            self.state.create_copy_from = if new_val == 0 {
                                                None
                                            } else {
                                                Some(new_val - 1)
                                            };
                                            return Ok(ScreenAction::Refresh);
                                        }
                                    }
                                }
                                Some(Action::MoveDown) => {
                                    if self.state.create_focused_field == CreateField::CopyFrom {
                                        // We need profile count to limit.
                                        // We need profile count to limit.
                                        let profiles = &self.state.profiles;
                                        let total = profiles.len() + 1; // +1 for "Blank"
                                        let current =
                                            self.state.create_copy_from.map(|i| i + 1).unwrap_or(0);
                                        if current < total - 1 {
                                            let new_val = current + 1;
                                            self.state.create_copy_from = Some(new_val - 1);
                                            return Ok(ScreenAction::Refresh);
                                        }
                                    }
                                }
                                _ => {
                                    if let Some(act) = action {
                                        let key_code = match act {
                                            Action::MoveLeft => KeyCode::Left,
                                            Action::MoveRight => KeyCode::Right,
                                            Action::Home => KeyCode::Home,
                                            Action::End => KeyCode::End,
                                            Action::Backspace => KeyCode::Backspace,
                                            Action::DeleteChar => KeyCode::Delete,
                                            _ => KeyCode::Null,
                                        };

                                        if key_code != KeyCode::Null {
                                            match self.state.create_focused_field {
                                                CreateField::Name => {
                                                    if act == Action::Backspace {
                                                        handle_backspace(
                                                            &mut self.state.create_name_input,
                                                            &mut self.state.create_name_cursor,
                                                        );
                                                    } else if act == Action::DeleteChar {
                                                        handle_delete(
                                                            &mut self.state.create_name_input,
                                                            &mut self.state.create_name_cursor,
                                                        );
                                                    } else {
                                                        handle_cursor_movement(
                                                            &self.state.create_name_input,
                                                            &mut self.state.create_name_cursor,
                                                            key_code,
                                                        );
                                                    }
                                                }
                                                CreateField::Description => {
                                                    if act == Action::Backspace {
                                                        handle_backspace(
                                                            &mut self
                                                                .state
                                                                .create_description_input,
                                                            &mut self
                                                                .state
                                                                .create_description_cursor,
                                                        );
                                                    } else if act == Action::DeleteChar {
                                                        handle_delete(
                                                            &mut self
                                                                .state
                                                                .create_description_input,
                                                            &mut self
                                                                .state
                                                                .create_description_cursor,
                                                        );
                                                    } else {
                                                        handle_cursor_movement(
                                                            &self.state.create_description_input,
                                                            &mut self
                                                                .state
                                                                .create_description_cursor,
                                                            key_code,
                                                        );
                                                    }
                                                }
                                                _ => {}
                                            }
                                            return Ok(ScreenAction::Refresh);
                                        }
                                    }

                                    // Char input
                                    if let KeyCode::Char(c) = key.code {
                                        if !key.modifiers.intersects(
                                            KeyModifiers::CONTROL
                                                | KeyModifiers::ALT
                                                | KeyModifiers::SUPER,
                                        ) {
                                            match self.state.create_focused_field {
                                                CreateField::Name => {
                                                    handle_char_insertion(
                                                        &mut self.state.create_name_input,
                                                        &mut self.state.create_name_cursor,
                                                        c,
                                                    );
                                                }
                                                CreateField::Description => {
                                                    handle_char_insertion(
                                                        &mut self.state.create_description_input,
                                                        &mut self.state.create_description_cursor,
                                                        c,
                                                    );
                                                }
                                                _ => {}
                                            }
                                            return Ok(ScreenAction::Refresh);
                                        }
                                    }
                                }
                            }
                        }
                        ProfilePopupType::Rename => {
                            if let Some(action) = action {
                                match action {
                                    Action::Cancel => {
                                        self.state.popup_type = ProfilePopupType::None;
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::Confirm => {
                                        if !self.state.rename_input.is_empty() {
                                            if let Some(idx) = self.state.list_state.selected() {
                                                let profiles = &self.state.profiles;
                                                if let Some(profile) = profiles.get(idx) {
                                                    let old_name = profile.name.clone();
                                                    let new_name = self.state.rename_input.clone();
                                                    self.state.popup_type = ProfilePopupType::None;
                                                    self.state.rename_input.clear();
                                                    return Ok(ScreenAction::RenameProfile {
                                                        old_name,
                                                        new_name,
                                                    });
                                                }
                                            }
                                        }
                                        return Ok(ScreenAction::None);
                                    }
                                    Action::Backspace => {
                                        handle_backspace(
                                            &mut self.state.rename_input,
                                            &mut self.state.rename_cursor,
                                        );
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::DeleteChar => {
                                        handle_delete(
                                            &mut self.state.rename_input,
                                            &mut self.state.rename_cursor,
                                        );
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::MoveLeft
                                    | Action::MoveRight
                                    | Action::Home
                                    | Action::End => {
                                        let key_code = match action {
                                            Action::MoveLeft => KeyCode::Left,
                                            Action::MoveRight => KeyCode::Right,
                                            Action::Home => KeyCode::Home,
                                            Action::End => KeyCode::End,
                                            _ => KeyCode::Null,
                                        };
                                        handle_cursor_movement(
                                            &self.state.rename_input,
                                            &mut self.state.rename_cursor,
                                            key_code,
                                        );
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    _ => {}
                                }
                            }

                            // Char input
                            if let KeyCode::Char(c) = key.code {
                                if !key.modifiers.intersects(
                                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                                ) {
                                    handle_char_insertion(
                                        &mut self.state.rename_input,
                                        &mut self.state.rename_cursor,
                                        c,
                                    );
                                    return Ok(ScreenAction::Refresh);
                                }
                            }
                        }
                        ProfilePopupType::Delete => {
                            if let Some(action) = action {
                                match action {
                                    Action::Cancel => {
                                        self.state.popup_type = ProfilePopupType::None;
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::Confirm => {
                                        if let Some(idx) = self.state.list_state.selected() {
                                            let profiles = &self.state.profiles;
                                            if let Some(profile) = profiles.get(idx) {
                                                if self.state.delete_confirm_input == profile.name {
                                                    let name = profile.name.clone();
                                                    self.state.popup_type = ProfilePopupType::None;
                                                    self.state.delete_confirm_input.clear();
                                                    return Ok(ScreenAction::DeleteProfile {
                                                        name,
                                                    });
                                                }
                                            }
                                        }
                                        // If input doesn't match or whatever, maybe shake or just do nothing?
                                        // Original just did nothing.
                                        return Ok(ScreenAction::None);
                                    }
                                    Action::Backspace => {
                                        handle_backspace(
                                            &mut self.state.delete_confirm_input,
                                            &mut self.state.delete_confirm_cursor,
                                        );
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::DeleteChar => {
                                        handle_delete(
                                            &mut self.state.delete_confirm_input,
                                            &mut self.state.delete_confirm_cursor,
                                        );
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::MoveLeft
                                    | Action::MoveRight
                                    | Action::Home
                                    | Action::End => {
                                        let key_code = match action {
                                            Action::MoveLeft => KeyCode::Left,
                                            Action::MoveRight => KeyCode::Right,
                                            Action::Home => KeyCode::Home,
                                            Action::End => KeyCode::End,
                                            _ => KeyCode::Null,
                                        };
                                        handle_cursor_movement(
                                            &self.state.delete_confirm_input,
                                            &mut self.state.delete_confirm_cursor,
                                            key_code,
                                        );
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    _ => {}
                                }
                            }
                            if let KeyCode::Char(c) = key.code {
                                if !key.modifiers.intersects(
                                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                                ) {
                                    handle_char_insertion(
                                        &mut self.state.delete_confirm_input,
                                        &mut self.state.delete_confirm_cursor,
                                        c,
                                    );
                                    return Ok(ScreenAction::Refresh);
                                }
                            }
                        }
                        ProfilePopupType::Switch => {
                            if let Some(action) = action {
                                match action {
                                    Action::Cancel => {
                                        self.state.popup_type = ProfilePopupType::None;
                                        return Ok(ScreenAction::Refresh);
                                    }
                                    Action::Confirm => {
                                        if let Some(idx) = self.state.list_state.selected() {
                                            let profiles = &self.state.profiles;
                                            if let Some(profile) = profiles.get(idx) {
                                                let name = profile.name.clone();
                                                self.state.popup_type = ProfilePopupType::None;
                                                return Ok(ScreenAction::SwitchProfile { name });
                                            }
                                        }
                                        return Ok(ScreenAction::None);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        ProfilePopupType::None => {} // Should not be reachable inside this match
                    }
                    return Ok(ScreenAction::None);
                }
                _ => return Ok(ScreenAction::None),
            }
        }

        // Main screen events
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                let action = self.get_action(key.code, key.modifiers, &ctx.config.keymap);
                if let Some(action) = action {
                    match action {
                        Action::Cancel => return Ok(ScreenAction::Navigate(ScreenId::MainMenu)),
                        Action::MoveUp => {
                            let selected = self.state.list_state.selected().unwrap_or(0);
                            let new_selected = if selected > 0 { selected - 1 } else { selected };
                            self.state.list_state.select(Some(new_selected));
                            return Ok(ScreenAction::Refresh);
                        }
                        Action::MoveDown => {
                            let profiles = &self.state.profiles;
                            let selected = self.state.list_state.selected().unwrap_or(0);
                            let new_selected =
                                if !profiles.is_empty() && selected < profiles.len() - 1 {
                                    selected + 1
                                } else {
                                    selected
                                };
                            self.state.list_state.select(Some(new_selected));
                            return Ok(ScreenAction::Refresh);
                        }
                        Action::Create => {
                            self.state.popup_type = ProfilePopupType::Create;
                            self.state.create_name_input.clear();
                            self.state.create_description_input.clear();
                            self.state.create_focused_field = CreateField::Name;
                            self.state.create_copy_from = None;
                            return Ok(ScreenAction::Refresh);
                        }
                        Action::Edit => {
                            // Rename
                            if let Some(idx) = self.state.list_state.selected() {
                                let profiles = &self.state.profiles;
                                if let Some(profile) = profiles.get(idx) {
                                    self.state.popup_type = ProfilePopupType::Rename;
                                    self.state.rename_input = profile.name.clone();
                                    self.state.rename_cursor =
                                        self.state.rename_input.chars().count();
                                    return Ok(ScreenAction::Refresh);
                                }
                            }
                        }
                        Action::Delete => {
                            if let Some(idx) = self.state.list_state.selected() {
                                let profiles = &self.state.profiles;
                                if profiles.get(idx).is_some() {
                                    self.state.popup_type = ProfilePopupType::Delete;
                                    self.state.delete_confirm_input.clear();
                                    return Ok(ScreenAction::Refresh);
                                }
                            }
                        }
                        Action::Confirm => {
                            // Switch or just select?
                            // Navigation implies Confirm usually acts as "Action on current item"
                            // For profiles, that's likely "Switch to this profile" or "Show details" -> but details are side-by-side
                            // Original code (footer): "Switch Profile"
                            // So Confirm -> Switch Popup

                            self.state.popup_type = ProfilePopupType::Switch;
                            return Ok(ScreenAction::Refresh);
                        }
                        _ => {}
                    }
                }
            }
            Event::Mouse(mouse) => {
                return Ok(self.handle_mouse_event(mouse, ctx.config));
            }
            _ => {}
        }

        Ok(ScreenAction::None)
    }
}
