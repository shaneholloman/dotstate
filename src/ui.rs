use ratatui::widgets::{ScrollbarState, ListState};
use crate::file_manager::Dotfile;
use std::path::PathBuf;

/// Application screens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    MainMenu,
    DotfileSelection,
    GitHubAuth,
    ViewSyncedFiles,
    PushChanges,
    PullChanges,
}

/// GitHub auth state
#[derive(Debug, Clone)]
pub struct GitHubAuthState {
    pub token_input: String,
    pub step: GitHubAuthStep,
    pub error_message: Option<String>,
    pub status_message: Option<String>,
    pub show_help: bool,
    pub help_scroll: usize,
    pub cursor_position: usize, // For token input
    pub input_focused: bool, // Whether input is currently focused
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHubAuthStep {
    TokenInput,
    Processing,
}

impl Default for GitHubAuthState {
    fn default() -> Self {
        Self {
            token_input: String::new(),
            step: GitHubAuthStep::TokenInput,
            error_message: None,
            status_message: None,
            show_help: true,
            help_scroll: 0,
            cursor_position: 0,
            input_focused: true, // Input starts focused
        }
    }
}

/// Focus area in dotfile selection screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotfileSelectionFocus {
    FilesList,           // Files list pane is focused
    Preview,             // Preview pane is focused
    FileBrowserList,     // File browser list pane is focused
    FileBrowserPreview,  // File browser preview pane is focused
    FileBrowserInput,    // File browser path input is focused
    CustomInput,         // Custom file input is focused
}

/// Dotfile selection state
#[derive(Debug)]
pub struct DotfileSelectionState {
    pub dotfiles: Vec<Dotfile>,
    pub selected_index: usize, // Deprecated, using dotfile_list_state now
    pub preview_index: Option<usize>,
    pub scroll_offset: usize, // Deprecated, using dotfile_list_state now
    pub preview_scroll: usize,
    pub selected_for_sync: std::collections::HashSet<usize>, // Indices of selected files
    pub dotfile_list_scrollbar: ScrollbarState, // Scrollbar state for dotfile list
    pub dotfile_list_state: ListState, // ListState for main dotfile list (handles selection and scrolling)
    pub status_message: Option<String>, // For sync summary
    pub adding_custom_file: bool, // Whether we're in "add custom file" mode
    pub custom_file_input: String, // Input for custom file path
    pub custom_file_cursor: usize, // Cursor position for custom file input
    pub custom_file_focused: bool, // Whether custom file input is focused
    pub file_browser_mode: bool, // Whether we're in file browser mode
    pub file_browser_path: PathBuf, // Current directory in file browser
    pub file_browser_selected: usize, // Selected file index in browser
    pub file_browser_entries: Vec<PathBuf>, // Files/dirs in current directory
    #[allow(dead_code)]
    pub file_browser_scroll: usize, // Scroll offset for file browser list (deprecated, using ListState now)
    pub file_browser_scrollbar: ScrollbarState, // Scrollbar state for file browser
    pub file_browser_list_state: ListState, // ListState for file browser (handles selection and scrolling)
    pub file_browser_preview_scroll: usize, // Scroll offset for file browser preview
    pub file_browser_path_input: String, // Path input for file browser
    pub file_browser_path_cursor: usize, // Cursor position for path input
    pub file_browser_path_focused: bool, // Whether path input is focused
    pub focus: DotfileSelectionFocus, // Which pane currently has focus
}

impl Default for DotfileSelectionState {
    fn default() -> Self {
        Self {
            dotfiles: Vec::new(),
            selected_index: 0,
            preview_index: None,
            scroll_offset: 0,
            preview_scroll: 0,
            selected_for_sync: std::collections::HashSet::new(),
            dotfile_list_scrollbar: ScrollbarState::new(0),
            dotfile_list_state: ListState::default(),
            status_message: None,
            adding_custom_file: false,
            custom_file_input: String::new(),
            custom_file_cursor: 0,
            custom_file_focused: true,
            file_browser_mode: false,
            file_browser_path: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            file_browser_selected: 0,
            file_browser_entries: Vec::new(),
            file_browser_scroll: 0,
            file_browser_scrollbar: ScrollbarState::new(0),
            file_browser_list_state: ListState::default(),
            file_browser_preview_scroll: 0,
            file_browser_path_input: String::new(),
            file_browser_path_cursor: 0,
            file_browser_path_focused: false,
            focus: DotfileSelectionFocus::FilesList, // Start with files list focused
        }
    }
}

/// Application UI state
#[derive(Debug)]
pub struct UiState {
    pub current_screen: Screen,
    pub selected_index: usize,
    pub github_auth: GitHubAuthState,
    pub dotfile_selection: DotfileSelectionState,
    pub has_changes_to_push: bool, // Whether there are uncommitted or unpushed changes
}

impl UiState {
    pub fn new() -> Self {
        Self {
            current_screen: Screen::Welcome,
            selected_index: 0,
            github_auth: GitHubAuthState::default(),
            dotfile_selection: DotfileSelectionState::default(),
            has_changes_to_push: false,
        }
    }
}

// Legacy render functions removed - replaced by components:
// - render_welcome() -> WelcomeComponent
// - render_main_menu() -> MainMenuComponent
// - render_github_auth() -> GitHubAuthComponent
// - render_message() -> MessageComponent
// - render_synced_files() -> SyncedFilesComponent
// - render_dotfile_selection() -> DotfileSelectionComponent::render_with_state()
// popup_area removed - use crate::utils::center_popup instead
