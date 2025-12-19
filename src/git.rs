use anyhow::{Context, Result};
use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks, Repository, Signature};
use std::path::Path;

/// Git operations for managing the dotfiles repository
pub struct GitManager {
    repo: Repository,
}

impl GitManager {
    /// Open or initialize a repository
    pub fn open_or_init(repo_path: &Path) -> Result<Self> {
        let repo = if repo_path.join(".git").exists() {
            Repository::open(repo_path)
                .with_context(|| format!("Failed to open repository: {:?}", repo_path))?
        } else {
            // Initialize as a normal (non-bare) repository so it has a working directory
            let repo = Repository::init(repo_path)
                .with_context(|| format!("Failed to initialize repository: {:?}", repo_path))?;

            // Create .gitignore with common patterns for frequently changing files
            Self::ensure_gitignore(repo_path)?;

            repo
        };

        // Ensure .gitignore exists even for existing repos (won't overwrite if it exists)
        let _ = Self::ensure_gitignore(repo_path);

        // Verify the repository has a working directory (not bare)
        if repo.is_bare() {
            return Err(anyhow::anyhow!(
                "Repository at {:?} is a bare repository and has no working directory. \
                Cannot add files to index.",
                repo_path
            ));
        }

        Ok(Self { repo })
    }

    /// Ensure .gitignore exists with common patterns for frequently changing files
    fn ensure_gitignore(repo_path: &Path) -> Result<()> {
        use std::fs;
        use std::io::Write;

        let gitignore_path = repo_path.join(".gitignore");

        // If .gitignore already exists, don't overwrite it
        if gitignore_path.exists() {
            return Ok(());
        }

        let mut file = fs::File::create(&gitignore_path)
            .with_context(|| format!("Failed to create .gitignore at {:?}", gitignore_path))?;

        // Write common patterns for files that shouldn't be tracked
        writeln!(file, "# OS files")?;
        writeln!(file, ".DS_Store")?;
        writeln!(file, "Thumbs.db")?;
        writeln!(file, "")?;
        writeln!(file, "# Backup files")?;
        writeln!(file, "*.bak")?;
        writeln!(file, "*.swp")?;
        writeln!(file, "*.swo")?;
        writeln!(file, "*~")?;

        Ok(())
    }

    /// Add all changes and commit
    pub fn commit_all(&self, message: &str) -> Result<()> {
        let mut index = self.repo.index()
            .context("Failed to get repository index")?;

        // Refresh the index to ensure it's up to date
        index.read(true)
            .context("Failed to refresh index")?;

        // Use add_all with "." to add all files (equivalent to "git add .")
        // Skip vim bundles since they are git repos themselves and vimrc will install them
        index.add_all(
            &["."],
            git2::IndexAddOption::DEFAULT,
            Some(&mut |path: &Path, _matched_spec: &[u8]| {
                // Skip vim bundle directories (they are git repos and will be installed by vimrc)
                let path_str = path.to_string_lossy();
                if path_str.contains(".vim/bundle/") || path_str.contains(".vim/plugged/") {
                    1 // Skip vim bundles
                } else {
                    0 // Accept everything else
                }
            })
        )
        .context("Failed to add files to index (git add .)")?;

        index.write()
            .context("Failed to write index")?;

        let tree_id = index.write_tree()
            .context("Failed to write tree")?;
        let tree = self.repo.find_tree(tree_id)
            .context("Failed to find tree")?;

        let signature = Self::get_signature()?;
        let head = self.repo.head();

        let parent_commit = if let Ok(head) = head {
            Some(head.peel_to_commit()
                .context("Failed to peel HEAD to commit")?)
        } else {
            None
        };

        let parents: Vec<&git2::Commit> = parent_commit.iter().collect();
        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .context("Failed to create commit")?;

        Ok(())
    }

    /// Push to remote
    /// If token is provided, it will be used for authentication.
    /// Otherwise, attempts to extract token from remote URL.
    pub fn push(&self, remote_name: &str, branch: &str, token: Option<&str>) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)
            .with_context(|| format!("Remote '{}' not found", remote_name))?;

        let remote_url = remote.url()
            .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no URL", remote_name))?;

        let mut callbacks = RemoteCallbacks::new();

        // Try to get token from parameter first, then from URL
        let token_to_use = token
            .map(|t| t.to_string())
            .or_else(|| Self::extract_token_from_url(remote_url));

        if let Some(token) = token_to_use {
            // Set up credentials callback with the token
            // For GitHub PAT authentication, use token as username and password
            let token_clone = token.clone();
            callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
                // GitHub PAT: use token as both username and password
                Cred::userpass_plaintext(&token_clone, &token_clone)
            });
        } else {
            // If no token found, provide a helpful error
            callbacks.credentials(|_url, _username_from_url, _allowed_types| {
                Err(git2::Error::from_str(
                    "No credentials found. Please provide a GitHub token."
                ))
            });
        }

        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);

        // Check if branch exists locally
        let branch_ref = format!("refs/heads/{}", branch);
        if self.repo.find_reference(&branch_ref).is_err() {
            // Branch doesn't exist, try to get current branch
            if let Some(current_branch) = self.get_current_branch() {
                let refspec = format!("refs/heads/{}:refs/heads/{}", current_branch, branch);
                remote.push(&[&refspec], Some(&mut push_options))
                    .with_context(|| format!("Failed to push to remote '{}'", remote_name))?;
                return Ok(());
            }
            return Err(anyhow::anyhow!("No branch '{}' exists and no current branch found", branch));
        }

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
        remote.push(&[&refspec], Some(&mut push_options))
            .with_context(|| {
                // Get more detailed error information
                let remote_url = remote.url().unwrap_or("unknown");
                format!(
                    "Failed to push to remote '{}' (URL: {}). \
                    Check that:\n  - Your GitHub token has 'repo' scope\n  - The remote branch exists\n  - You have push permissions",
                    remote_name, remote_url
                )
            })?;

        Ok(())
    }

    /// Extract token from a GitHub URL (format: https://token@github.com/...)
    fn extract_token_from_url(url: &str) -> Option<String> {
        // Parse URL like https://token@github.com/user/repo.git
        if let Some(at_pos) = url.find('@') {
            if url.starts_with("https://") {
                let start = 8; // Skip "https://"
                if at_pos > start {
                    let token_part = &url[start..at_pos];
                    if !token_part.is_empty() {
                        return Some(token_part.to_string());
                    }
                }
            }
        }
        None
    }

    /// Pull from remote
    pub fn pull(&self, remote_name: &str, branch: &str) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)
            .with_context(|| format!("Remote '{}' not found", remote_name))?;

        remote.fetch(&[branch], None, None)
            .with_context(|| format!("Failed to fetch from remote '{}'", remote_name))?;

        let fetch_head = self.repo.find_reference("FETCH_HEAD")
            .context("Failed to find FETCH_HEAD")?;
        let fetch_commit = fetch_head.peel_to_commit()
            .context("Failed to peel FETCH_HEAD to commit")?;

        // Convert commit to annotated commit for merge
        let annotated_commit = self.repo.find_annotated_commit(fetch_commit.id())
            .context("Failed to create annotated commit")?;

        self.repo.merge(&[&annotated_commit], None, None)
            .context("Failed to merge")?;

        Ok(())
    }

    /// Add a remote (or update if it exists)
    pub fn add_remote(&mut self, name: &str, url: &str) -> Result<()> {
        // remote_set_url doesn't exist in git2, so we delete and recreate
        if self.repo.find_remote(name).is_ok() {
            self.repo.remote_delete(name)
                .with_context(|| format!("Failed to delete existing remote '{}'", name))?;
        }
        self.repo.remote(name, url)
            .with_context(|| format!("Failed to add remote '{}'", name))?;
        Ok(())
    }

    /// Get signature for commits
    fn get_signature() -> Result<Signature<'static>> {
        // Try to get from git config, fallback to defaults
        let config = git2::Config::open_default().ok();

        let name = config
            .as_ref()
            .and_then(|c| c.get_string("user.name").ok())
            .unwrap_or_else(|| "dotzz".to_string());

        let email = config
            .as_ref()
            .and_then(|c| c.get_string("user.email").ok())
            .unwrap_or_else(|| "dotzz@localhost".to_string());

        Ok(Signature::now(&name, &email)?)
    }

    /// Get the repository reference
    #[allow(dead_code)]
    pub fn repo(&self) -> &Repository {
        &self.repo
    }

    /// Check if there are uncommitted changes
    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let mut index = self.repo.index()
            .context("Failed to get repository index")?;

        // Refresh the index to get current state
        index.read(true)
            .context("Failed to read index")?;

        // Check if index differs from HEAD
        let head = match self.repo.head() {
            Ok(head) => Some(head.peel_to_tree()
                .context("Failed to peel HEAD to tree")?),
            Err(_) => None,
        };

        if let Some(head_tree) = head {
            let diff = self.repo.diff_tree_to_index(
                Some(&head_tree),
                Some(&index),
                None,
            )
            .context("Failed to create diff")?;

            // Check if there are any differences
            let has_changes = diff.deltas().next().is_some();

            // Also check for untracked files
            let mut status_opts = git2::StatusOptions::new();
            status_opts.include_untracked(true);
            status_opts.include_ignored(false);

            let statuses = self.repo.statuses(Some(&mut status_opts))
                .context("Failed to get status")?;

            let has_untracked = statuses.iter().any(|s| {
                s.status().contains(git2::Status::WT_NEW)
            });

            Ok(has_changes || has_untracked)
        } else {
            // No HEAD, check if index has any entries
            Ok(index.len() > 0)
        }
    }

    /// Check if there are unpushed commits
    pub fn has_unpushed_commits(&self, remote_name: &str, branch: &str) -> Result<bool> {
        // Check if remote exists
        let mut remote = match self.repo.find_remote(remote_name) {
            Ok(r) => r,
            Err(_) => return Ok(false), // No remote, so no unpushed commits
        };

        // Get local branch
        let branch_ref = format!("refs/heads/{}", branch);
        let local_branch = match self.repo.find_reference(&branch_ref) {
            Ok(r) => r,
            Err(_) => return Ok(false), // No local branch
        };

        let local_oid = local_branch.target()
            .context("Failed to get local branch OID")?;

        // Fetch from remote to update remote refs
        let mut remote_callbacks = RemoteCallbacks::new();
        remote_callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            // For now, we'll just fail if credentials are needed
            // In the future, we could use stored credentials
            Err(git2::Error::from_str("Credentials required"))
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(remote_callbacks);

        // Try to fetch (ignore errors - remote might not be accessible)
        let _ = remote.fetch(&[branch], Some(&mut fetch_opts), None);

        // Get remote branch reference
        let remote_ref = format!("refs/remotes/{}/{}", remote_name, branch);
        let remote_branch = match self.repo.find_reference(&remote_ref) {
            Ok(r) => r,
            Err(_) => return Ok(true), // No remote branch, so we have unpushed commits
        };

        let remote_oid = remote_branch.target()
            .context("Failed to get remote branch OID")?;

        // Check if local is ahead of remote (local commit is reachable from remote)
        match self.repo.graph_ahead_behind(local_oid, remote_oid) {
            Ok((ahead, _behind)) => Ok(ahead > 0),
            Err(_) => Ok(true), // Can't determine, assume there are unpushed commits
        }
    }

    /// Get the current branch name
    pub fn get_current_branch(&self) -> Option<String> {
        let head = self.repo.head().ok()?;
        let name = head.name()?;
        // Remove 'refs/heads/' prefix
        name.strip_prefix("refs/heads/").map(|s| s.to_string())
    }

    /// Get list of changed files (modified, added, deleted)
    pub fn get_changed_files(&self) -> Result<Vec<String>> {
        let mut status_opts = git2::StatusOptions::new();
        status_opts.include_untracked(true);
        // Show all untracked files including those in subdirectories (equivalent to -uall)
        // This is done by setting recurse_untracked_dirs to true
        status_opts.recurse_untracked_dirs(true);
        status_opts.include_ignored(false);
        status_opts.include_unmodified(false);

        let statuses = self.repo.statuses(Some(&mut status_opts))
            .context("Failed to get repository status")?;

        let mut changed_files = Vec::new();
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();
                let prefix = if status.contains(git2::Status::WT_NEW) {
                    "A " // Added
                } else if status.contains(git2::Status::WT_MODIFIED) {
                    "M " // Modified
                } else if status.contains(git2::Status::WT_DELETED) {
                    "D " // Deleted
                } else if status.contains(git2::Status::INDEX_NEW) {
                    "A " // Staged new
                } else if status.contains(git2::Status::INDEX_MODIFIED) {
                    "M " // Staged modified
                } else if status.contains(git2::Status::INDEX_DELETED) {
                    "D " // Staged deleted
                } else {
                    "? " // Unknown
                };
                changed_files.push(format!("{}{}", prefix, path));
            }
        }

        Ok(changed_files)
    }

    /// Clone a repository from a remote URL
    pub fn clone(url: &str, path: &Path, token: Option<&str>) -> Result<Self> {
        let mut fetch_options = FetchOptions::new();

        // Set up authentication if token is provided
        if let Some(token) = token {
            let mut callbacks = RemoteCallbacks::new();
            let token_clone = token.to_string();
            callbacks.credentials(move |_url, username, _allowed_types| {
                Cred::userpass_plaintext(username.unwrap_or("git"), &token_clone)
            });
            fetch_options.remote_callbacks(callbacks);
        }

        let mut builder = RepoBuilder::new();
        builder.fetch_options(fetch_options);

        let repo = builder.clone(url, path)
            .with_context(|| format!("Failed to clone repository from {} to {:?}", url, path))?;

        Ok(Self { repo })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_git_init() {
        let temp_dir = TempDir::new().unwrap();
        let git_mgr = GitManager::open_or_init(temp_dir.path()).unwrap();
        assert!(git_mgr.repo().is_empty().unwrap_or(false));
    }
}


