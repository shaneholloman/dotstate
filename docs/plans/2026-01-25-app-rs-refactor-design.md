# App.rs Refactor Design

**Date:** 2026-01-25
**Status:** Approved
**Goal:** Extract screen-specific logic from app.rs to appropriate locations (screens, services)

## Problem

`app.rs` has grown to 2,371 lines with two main culprits:
- `process_screen_action()` - 535 lines of business logic
- `process_storage_setup_step()` - 491 lines (GitHub setup state machine)

Screen implementation details are scattered in app.rs instead of living with their screens.

## Solution: Hybrid Approach

- **Complex/stateful logic** → Services (StorageSetupService)
- **Routine screen CRUD** → Screen `process_action()` methods
- **Coordination/routing** → Stays in App

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Config access | Pass `&mut Config` directly | Matches existing pattern, simplest |
| Async handling | Oneshot channels, polled in main loop | Matches existing `git_status_receiver` pattern |
| Action processing | Screens get `process_action()` method | Related code lives together |

## New Files

### `src/services/storage_setup_service.rs` (~300 lines)

Handles the GitHub setup state machine with async steps.

```rust
pub enum StepResult {
    Continue {
        next_step: GitHubSetupStep,
        data: GitHubSetupData,
        status_message: String,
    },
    Complete {
        profiles: Vec<String>,
        is_new_repo: bool,
    },
    Failed {
        error_message: String,
    },
}

pub struct StepHandle {
    pub receiver: oneshot::Receiver<Result<StepResult>>,
}

impl StorageSetupService {
    /// Start an async setup step - returns immediately
    pub fn start_step(
        step: GitHubSetupStep,
        data: GitHubSetupData,
        config: &Config,
        runtime: &Runtime,
    ) -> StepHandle;

    /// Cleanup after failed setup
    pub fn cleanup_failed_setup(
        config: &mut Config,
        config_path: &Path,
        cleanup_repo: bool,
    );
}
```

**Steps handled:**
- Connecting, ValidatingToken, CheckingRepo
- CloningRepo, CreatingRepo, InitializingRepo
- DiscoveringProfiles, Complete

## Screen Modifications

### Common Pattern

Each screen gets a `process_action()` method:

```rust
pub fn process_action(
    &mut self,
    action: ScreenSpecificAction,
    config: &mut Config,
    config_path: &Path,
) -> Result<ActionResult>
```

### ActionResult Enum

```rust
pub enum ActionResult {
    None,
    ShowToast { message: String, variant: ToastVariant },
    ShowDialog { title: String, content: String, variant: DialogVariant },
    Navigate(Screen),
}
```

### DotfileSelectionScreen

**Actions:** ScanDotfiles, RefreshFileBrowser, ToggleFileSync, AddCustomFileToSync, SetBackupEnabled, MoveToCommon

**Methods moving in:**
- `scan_dotfiles_into()` → internal method
- `refresh_file_browser_into()` → internal method
- `add_file_to_sync_with_state()` → internal method
- `remove_file_from_sync_with_state()` → internal method

### ManageProfilesScreen

**Actions:** CreateProfile, RenameProfile, DeleteProfile, SwitchProfile

**Methods moving in:**
- `switch_profile()` → internal method
- `rename_profile()` → internal method

### ProfileSelectionScreen

**Actions:** CreateAndActivateProfile, ActivateProfile

**Methods moving in:**
- `activate_profile_after_setup()` → internal method

### StorageSetupScreen

**Actions:** SaveLocalRepoConfig, UpdateGitHubToken

## App.rs After Refactor (~800-1000 lines)

### Responsibilities

- `App` struct with screen instances, runtime, receivers
- `new()` - initialization
- `run()` - main event loop, polling receivers
- `draw()` - screen routing (~200 lines, unchanged)
- `handle_event()` - global keys, screen routing (~160 lines, unchanged)
- `process_screen_action()` - thin router (~100 lines)

### Thin Router Pattern

```rust
fn process_screen_action(&mut self, action: ScreenAction) -> Result<()> {
    match action {
        // Simple actions handled directly
        ScreenAction::None => {}
        ScreenAction::Navigate(target) => self.navigate_to(target)?,
        ScreenAction::Quit => self.should_quit = true,
        ScreenAction::ShowToast { .. } => { /* push to toast_manager */ }
        ScreenAction::ShowMessage { .. } => { /* set dialog_state */ }

        // Delegate to screens
        ScreenAction::ScanDotfiles | ScreenAction::ToggleFileSync { .. } => {
            let result = self.dotfile_selection_screen
                .process_action(action.into(), &mut self.config, &self.config_path)?;
            self.handle_action_result(result)?;
        }

        // Delegate to service (async)
        ScreenAction::StartGitHubSetup { .. } => {
            self.start_github_setup(/* ... */);
        }

        // ... other delegations
    }
}
```

## Testing Checklist

### StorageSetup Flow
- [ ] GitHub setup with new repo - creates repo, clones, shows profile selection
- [ ] GitHub setup with existing repo - clones, discovers profiles
- [ ] GitHub setup with invalid token - shows error, returns to input
- [ ] GitHub setup network failure mid-flow - cleanup runs, returns to input
- [ ] Local repo setup - validates path, loads profiles
- [ ] Token update for existing GitHub repo - validates, updates remote URL

### DotfileSelection Flow
- [ ] Scan dotfiles - shows files from home + custom files
- [ ] Add file to sync - creates symlink, updates manifest
- [ ] Remove file from sync - removes symlink, restores original
- [ ] Move to common - file appears in common/, symlinked for all profiles
- [ ] Move from common to profile - file moves to profile dir
- [ ] File browser navigation - can browse and add custom files
- [ ] Backup toggle - creates backups when enabled

### Profile Management
- [ ] Create profile - new dir created, manifest updated
- [ ] Switch profile - symlinks updated to new profile's files
- [ ] Rename profile - dir renamed, manifest updated, symlinks updated if active
- [ ] Delete profile - dir removed, manifest updated, can't delete active

### General
- [ ] Theme cycling still works (t key)
- [ ] Help overlay still works (? key)
- [ ] Toast notifications display correctly
- [ ] Dialog errors display and dismiss
- [ ] Navigation between all screens works

## Migration Strategy

1. Create `StorageSetupService` first (isolated, no breaking changes)
2. Add `process_action()` to screens one at a time
3. Update `app.rs` to delegate after each screen is ready
4. Remove dead code from `app.rs` after all delegations complete
5. Run full test checklist after each major step
