//! Sync service for file synchronization operations.
//!
//! This module provides a service layer for file sync operations,
//! abstracting the details of file copying, symlink creation, and
//! manifest management from the UI layer.

use crate::config::Config;
use crate::file_manager::{copy_dir_all, Dotfile, FileManager};
use crate::utils::{get_home_dir, sync_validation, ProfileManifest, SymlinkManager};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Result of a sync validation.
#[derive(Debug)]
pub struct SyncValidationResult {
    /// Whether the operation is safe to perform.
    pub is_safe: bool,
    /// Error message if not safe.
    pub error_message: Option<String>,
}

/// Result of adding a file to sync.
#[derive(Debug)]
pub enum AddFileResult {
    /// File was successfully synced.
    Success,
    /// File was already synced, no action taken.
    AlreadySynced,
    /// Validation failed with the given error message.
    ValidationFailed(String),
}

/// Result of removing a file from sync.
#[derive(Debug)]
pub enum RemoveFileResult {
    /// File was successfully removed from sync.
    Success,
    /// File was not synced, no action taken.
    NotSynced,
}

/// Service for file synchronization operations.
///
/// This service provides a clean interface for file sync operations without
/// direct dependencies on UI state.
pub struct SyncService;

impl SyncService {
    /// Add a file to sync.
    ///
    /// This is the unified function that handles both regular dotfiles and custom files.
    /// It performs the following operations:
    /// 1. Validates the operation is safe
    /// 2. Copies the file to the repository
    /// 3. Creates a symlink from home to repo
    /// 4. Updates the profile manifest
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration.
    /// * `full_path` - Full path to the source file.
    /// * `relative_path` - Path relative to home directory.
    /// * `backup_enabled` - Whether to enable backups.
    ///
    /// # Returns
    ///
    /// Result indicating success, already synced, or validation failure.
    pub fn add_file_to_sync(
        config: &Config,
        full_path: &Path,
        relative_path: &str,
        backup_enabled: bool,
    ) -> Result<AddFileResult> {
        let profile_name = &config.active_profile;
        let repo_path = &config.repo_path;

        // Get previously synced files
        let previously_synced = Self::get_synced_files(repo_path, profile_name)?;

        // Check if already synced
        if previously_synced.contains(relative_path) {
            debug!("File already synced: {}", relative_path);
            return Ok(AddFileResult::AlreadySynced);
        }

        // VALIDATE BEFORE ANY OPERATIONS - prevent data loss
        let validation = sync_validation::validate_before_sync(
            relative_path,
            full_path,
            &previously_synced,
            repo_path,
        );
        if !validation.is_safe {
            let error_msg = validation
                .error_message
                .unwrap_or_else(|| "Cannot add this file or directory".to_string());
            warn!("Validation failed for {}: {}", relative_path, error_msg);
            return Ok(AddFileResult::ValidationFailed(error_msg));
        }

        // Create file manager for symlink resolution
        let file_manager = FileManager::new()?;

        // Validate symlink can be created before deleting original file
        let home_dir = get_home_dir();
        let target_path = home_dir.join(relative_path);
        let profile_path = repo_path.join(profile_name);
        let repo_file_path = profile_path.join(relative_path);

        // Handle symlinks: resolve to original file for validation
        let original_source = if file_manager.is_symlink(full_path) {
            file_manager.resolve_symlink(full_path)?
        } else {
            full_path.to_path_buf()
        };

        let symlink_validation = sync_validation::validate_symlink_creation(
            &original_source,
            &repo_file_path,
            &target_path,
        )
        .context("Failed to validate symlink creation")?;
        if !symlink_validation.is_safe {
            let error_msg = symlink_validation
                .error_message
                .unwrap_or_else(|| "Cannot create symlink".to_string());
            warn!(
                "Symlink validation failed for {}: {}",
                relative_path, error_msg
            );
            return Ok(AddFileResult::ValidationFailed(error_msg));
        }

        info!(
            "Adding file to sync: {} (profile: {})",
            relative_path, profile_name
        );
        debug!("Source path: {:?}", full_path);
        debug!("Repo destination: {:?}", repo_file_path);
        debug!("Symlink target: {:?}", target_path);

        // Create parent directories
        if let Some(parent) = repo_file_path.parent() {
            if !parent.exists() {
                debug!("Creating repo directory: {:?}", parent);
            }
            std::fs::create_dir_all(parent).context("Failed to create repo directory")?;
        }

        // Handle symlinks: resolve to original file
        let source_path = if file_manager.is_symlink(full_path) {
            debug!("Resolving symlink: {:?}", full_path);
            let resolved = file_manager.resolve_symlink(full_path)?;
            debug!("Resolved symlink to: {:?}", resolved);
            resolved
        } else {
            full_path.to_path_buf()
        };

        // Copy to repo FIRST (before deleting original)
        // This ensures we have a backup before any destructive operations
        info!("Copying file to repository...");
        file_manager
            .copy_to_repo(&source_path, &repo_file_path)
            .context("Failed to copy file to repo")?;
        info!("Successfully copied file to repository");

        // Create symlink using SymlinkManager
        info!("Creating symlink...");
        let mut symlink_mgr = SymlinkManager::new_with_backup(repo_path.clone(), backup_enabled)?;
        symlink_mgr
            .add_symlink_to_profile(profile_name, relative_path)
            .context("Failed to create symlink")?;
        info!("Successfully created symlink");

        // Update manifest
        info!("Updating profile manifest...");
        Self::add_file_to_manifest(repo_path, profile_name, relative_path)?;

        info!("Successfully added file to sync: {}", relative_path);
        Ok(AddFileResult::Success)
    }

    /// Remove a file from sync.
    ///
    /// This performs the following operations:
    /// 1. Removes the symlink from home
    /// 2. Restores the file from the repository
    /// 3. Removes the file from the repository
    /// 4. Updates the profile manifest
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration.
    /// * `relative_path` - Path relative to home directory.
    ///
    /// # Returns
    ///
    /// Result indicating success or that file was not synced.
    pub fn remove_file_from_sync(config: &Config, relative_path: &str) -> Result<RemoveFileResult> {
        let profile_name = &config.active_profile;
        let repo_path = &config.repo_path;
        let home_dir = get_home_dir();

        // Get previously synced files
        let previously_synced = Self::get_synced_files(repo_path, profile_name)?;

        if !previously_synced.contains(relative_path) {
            debug!("File not synced, skipping removal: {}", relative_path);
            return Ok(RemoveFileResult::NotSynced);
        }

        info!(
            "Removing file from sync: {} (profile: {})",
            relative_path, profile_name
        );

        let target_path = home_dir.join(relative_path);
        let repo_file_path = repo_path.join(profile_name).join(relative_path);

        // Restore file from repo if symlink exists
        if target_path.symlink_metadata().is_ok() {
            let metadata = target_path.symlink_metadata().unwrap();
            if metadata.is_symlink() {
                // Remove symlink
                std::fs::remove_file(&target_path).context("Failed to remove symlink")?;

                // Copy file from repo back to home
                if repo_file_path.exists() {
                    if repo_file_path.is_dir() {
                        copy_dir_all(&repo_file_path, &target_path)
                            .context("Failed to restore directory from repo")?;
                    } else {
                        std::fs::copy(&repo_file_path, &target_path)
                            .context("Failed to restore file from repo")?;
                    }
                }
            }
        }

        // Update symlink tracking - remove only the specific file
        let mut symlink_mgr = SymlinkManager::new(repo_path.clone())?;

        // Remove the specific symlink from tracking
        // Note: We already removed the actual symlink and restored the file above (lines 227-244)
        // This just updates the tracking data without touching other symlinks
        symlink_mgr.remove_symlink_from_tracking(profile_name, relative_path)?;

        // Remove from repo
        if repo_file_path.exists() {
            if repo_file_path.is_dir() {
                std::fs::remove_dir_all(&repo_file_path)
                    .context("Failed to remove directory from repo")?;
            } else {
                std::fs::remove_file(&repo_file_path).context("Failed to remove file from repo")?;
            }
        }

        // Update manifest - remove the file from the synced files list
        let remaining_files: Vec<String> = previously_synced
            .iter()
            .filter(|f| *f != relative_path)
            .cloned()
            .collect();

        let mut manifest = ProfileManifest::load_or_backfill(repo_path)?;
        manifest.update_synced_files(profile_name, remaining_files)?;
        manifest.save(repo_path)?;

        info!("Successfully removed file from sync: {}", relative_path);
        Ok(RemoveFileResult::Success)
    }

    /// Get the set of synced files for a profile.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the repository.
    /// * `profile_name` - Name of the profile.
    ///
    /// # Returns
    ///
    /// Set of synced file paths.
    pub fn get_synced_files(repo_path: &Path, profile_name: &str) -> Result<HashSet<String>> {
        let manifest = ProfileManifest::load_or_backfill(repo_path)?;
        Ok(manifest
            .profiles
            .into_iter()
            .find(|p| p.name == profile_name)
            .map(|p| p.synced_files.into_iter().collect())
            .unwrap_or_default())
    }

    /// Add a file path to the manifest for a profile.
    fn add_file_to_manifest(
        repo_path: &Path,
        profile_name: &str,
        relative_path: &str,
    ) -> Result<()> {
        let mut manifest = ProfileManifest::load_or_backfill(repo_path)?;
        let current_files = manifest
            .profiles
            .iter()
            .find(|p| p.name == profile_name)
            .map(|p| p.synced_files.clone())
            .unwrap_or_default();

        if !current_files.contains(&relative_path.to_string()) {
            debug!(
                "Adding {} to manifest for profile {}",
                relative_path, profile_name
            );
            let mut new_files = current_files;
            new_files.push(relative_path.to_string());
            manifest.update_synced_files(profile_name, new_files)?;
            manifest.save(repo_path)?;
            info!("Updated manifest with new file: {}", relative_path);
        } else {
            debug!("File already in manifest, skipping update");
        }
        Ok(())
    }

    /// Scan for dotfiles and return the list.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration.
    ///
    /// # Returns
    ///
    /// List of dotfiles found, with sync status marked.
    pub fn scan_dotfiles(config: &Config) -> Result<Vec<Dotfile>> {
        use crate::dotfile_candidates::get_default_dotfile_paths;

        let file_manager = FileManager::new()?;
        let dotfile_names = get_default_dotfile_paths();
        let mut found = file_manager.scan_dotfiles(&dotfile_names);

        // Mark files that are already synced
        let synced_set =
            Self::get_synced_files(&config.repo_path, &config.active_profile).unwrap_or_default();

        for dotfile in &mut found {
            let rel = dotfile.relative_path.to_string_lossy().to_string();
            if synced_set.contains(&rel) {
                dotfile.synced = true;
            }
        }

        // Also add custom files from config
        let home_dir = get_home_dir();
        for custom_path in &config.custom_files {
            let full_path = home_dir.join(custom_path);
            let relative_path = PathBuf::from(custom_path);

            // Skip if not a valid path or if it doesn't exist
            if !full_path.exists() && !full_path.is_symlink() {
                continue;
            }

            // Check if already in the list
            if found
                .iter()
                .any(|d| d.relative_path.to_string_lossy() == *custom_path)
            {
                continue;
            }

            let is_synced = synced_set.contains(custom_path);

            found.push(Dotfile {
                original_path: full_path,
                relative_path,
                synced: is_synced,
                description: None,
            });
        }

        // Sort by relative path for consistent display
        found.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        Ok(found)
    }

    /// Check if a path is a custom file (not in default dotfile candidates).
    ///
    /// # Arguments
    ///
    /// * `relative_path` - Path relative to home directory.
    ///
    /// # Returns
    ///
    /// True if this is a custom file.
    pub fn is_custom_file(relative_path: &str) -> bool {
        use crate::dotfile_candidates::get_default_dotfile_paths;
        let default_paths = get_default_dotfile_paths();
        !default_paths.iter().any(|p| p == relative_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_custom_file() {
        // Common dotfiles should not be custom
        assert!(!SyncService::is_custom_file(".bashrc"));
        assert!(!SyncService::is_custom_file(".zshrc"));

        // Random files should be custom
        assert!(SyncService::is_custom_file("my_custom_config"));
        assert!(SyncService::is_custom_file(".my_app/config.toml"));
    }
}
