//! Screen-specific state management.
//!
//! This module provides a unified enum for all screen states, ensuring
//! that only one screen state is active at a time.

use crate::ui::{
    DotfileSelectionState, GitHubAuthState, PackageManagerState, ProfileSelectionState,
    SyncWithRemoteState,
};
use crate::components::profile_manager::ProfileManagerState;

/// State for the main menu screen.
#[derive(Debug, Clone, Default)]
pub struct MainMenuState {
    /// Currently selected menu item index.
    pub selected_index: usize,
}

impl MainMenuState {
    /// Create a new main menu state.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Union type for all screen-specific states.
///
/// Using an enum ensures that only one screen state exists at a time,
/// preventing state synchronization bugs that occur when state is
/// duplicated between components and a central state object.
///
/// # Example
///
/// ```rust,ignore
/// match &mut app.screen_state {
///     ScreenState::MainMenu(state) => {
///         // Access main menu state exclusively
///         state.selected_index += 1;
///     }
///     ScreenState::GitHubAuth(state) => {
///         // Access github auth state exclusively
///         state.token_input.push('x');
///     }
///     _ => {}
/// }
/// ```
#[derive(Debug)]
pub enum ScreenState {
    /// Main menu screen state.
    MainMenu(MainMenuState),

    /// GitHub authentication/setup screen state.
    GitHubAuth(Box<GitHubAuthState>),

    /// Dotfile selection screen state.
    DotfileSelection(Box<DotfileSelectionState>),

    /// View synced files screen state.
    /// Note: This screen is stateless, just showing current config.
    ViewSyncedFiles,

    /// Sync with remote screen state.
    SyncWithRemote(Box<SyncWithRemoteState>),

    /// Profile manager screen state.
    ManageProfiles(Box<ProfileManagerState>),

    /// Profile selection screen state (after setup).
    ProfileSelection(ProfileSelectionState),

    /// Package manager screen state.
    ManagePackages(Box<PackageManagerState>),
}

impl Default for ScreenState {
    fn default() -> Self {
        Self::MainMenu(MainMenuState::default())
    }
}

impl ScreenState {
    /// Create a new screen state for the given screen.
    pub fn for_screen(screen: crate::ui::Screen) -> Self {
        use crate::ui::Screen;
        match screen {
            Screen::MainMenu => Self::MainMenu(MainMenuState::default()),
            Screen::GitHubAuth => Self::GitHubAuth(Box::new(GitHubAuthState::default())),
            Screen::DotfileSelection => {
                Self::DotfileSelection(Box::new(DotfileSelectionState::default()))
            }
            Screen::ViewSyncedFiles => Self::ViewSyncedFiles,
            Screen::SyncWithRemote => Self::SyncWithRemote(Box::new(SyncWithRemoteState::default())),
            Screen::ManageProfiles => Self::ManageProfiles(Box::new(ProfileManagerState::default())),
            Screen::ProfileSelection => Self::ProfileSelection(ProfileSelectionState::default()),
            Screen::ManagePackages => Self::ManagePackages(Box::new(PackageManagerState::default())),
        }
    }

    /// Get the current screen type.
    pub fn current_screen(&self) -> crate::ui::Screen {
        use crate::ui::Screen;
        match self {
            Self::MainMenu(_) => Screen::MainMenu,
            Self::GitHubAuth(_) => Screen::GitHubAuth,
            Self::DotfileSelection(_) => Screen::DotfileSelection,
            Self::ViewSyncedFiles => Screen::ViewSyncedFiles,
            Self::SyncWithRemote(_) => Screen::SyncWithRemote,
            Self::ManageProfiles(_) => Screen::ManageProfiles,
            Self::ProfileSelection(_) => Screen::ProfileSelection,
            Self::ManagePackages(_) => Screen::ManagePackages,
        }
    }

    /// Get mutable access to main menu state if on that screen.
    pub fn as_main_menu_mut(&mut self) -> Option<&mut MainMenuState> {
        if let Self::MainMenu(state) = self {
            Some(state)
        } else {
            None
        }
    }

    /// Get mutable access to github auth state if on that screen.
    pub fn as_github_auth_mut(&mut self) -> Option<&mut GitHubAuthState> {
        if let Self::GitHubAuth(state) = self {
            Some(state)
        } else {
            None
        }
    }

    /// Get mutable access to dotfile selection state if on that screen.
    pub fn as_dotfile_selection_mut(&mut self) -> Option<&mut DotfileSelectionState> {
        if let Self::DotfileSelection(state) = self {
            Some(state)
        } else {
            None
        }
    }

    /// Get mutable access to sync with remote state if on that screen.
    pub fn as_sync_with_remote_mut(&mut self) -> Option<&mut SyncWithRemoteState> {
        if let Self::SyncWithRemote(state) = self {
            Some(state)
        } else {
            None
        }
    }

    /// Get mutable access to profile manager state if on that screen.
    pub fn as_profile_manager_mut(&mut self) -> Option<&mut ProfileManagerState> {
        if let Self::ManageProfiles(state) = self {
            Some(state)
        } else {
            None
        }
    }

    /// Get mutable access to profile selection state if on that screen.
    pub fn as_profile_selection_mut(&mut self) -> Option<&mut ProfileSelectionState> {
        if let Self::ProfileSelection(state) = self {
            Some(state)
        } else {
            None
        }
    }

    /// Get mutable access to package manager state if on that screen.
    pub fn as_package_manager_mut(&mut self) -> Option<&mut PackageManagerState> {
        if let Self::ManagePackages(state) = self {
            Some(state)
        } else {
            None
        }
    }

    /// Check if text input is active on the current screen.
    ///
    /// This is used to determine if navigation keybindings should be disabled.
    pub fn is_input_focused(&self) -> bool {
        use crate::components::profile_manager::ProfilePopupType;
        match self {
            Self::GitHubAuth(state) => state.input_focused,
            Self::DotfileSelection(state) => {
                state.adding_custom_file || state.file_browser_path_focused
            }
            Self::ProfileSelection(state) => state.show_create_popup,
            Self::ManageProfiles(state) => state.popup_type != ProfilePopupType::None,
            Self::ManagePackages(state) => state.popup_type != crate::ui::PackagePopupType::None,
            _ => false,
        }
    }
}
