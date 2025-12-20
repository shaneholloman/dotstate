use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crate::config::Config;
use crate::git::GitManager;
use std::path::PathBuf;

/// A friendly TUI tool for managing dotfiles with GitHub sync
#[derive(Parser, Debug)]
#[command(name = "dotstate", version, about = "A friendly TUI tool for managing dotfiles with GitHub sync", long_about = None, disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Push changes to GitHub
    Push,
    /// Pull changes from GitHub
    Pull,
    /// List all synced files
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
    /// Add a file to sync
    Add {
        /// Path to the file to add
        path: PathBuf,
    },
    /// Show help for a specific command
    Help {
        /// Command to show help for
        command: Option<String>,
    },
}

impl Cli {
    /// Execute the CLI command
    pub fn execute(self) -> Result<()> {
        match self.command {
            Some(Commands::Push) => Self::cmd_push(),
            Some(Commands::Pull) => Self::cmd_pull(),
            Some(Commands::List { verbose }) => Self::cmd_list(verbose),
            Some(Commands::Add { path }) => Self::cmd_add(path),
            Some(Commands::Help { command }) => Self::cmd_help(command),
            None => {
                // No command provided, launch TUI
                Ok(())
            }
        }
    }

    fn cmd_push() -> Result<()> {
        let config_path = crate::utils::get_config_path();

        let config = Config::load_or_create(&config_path)
            .context("Failed to load configuration")?;

        if config.github.is_none() {
            eprintln!("❌ GitHub not configured. Please run 'dotstate' to set up GitHub sync.");
            std::process::exit(1);
        }

        let repo_path = &config.repo_path;
        let git_mgr = GitManager::open_or_init(repo_path)
            .context("Failed to open repository")?;

        let branch = git_mgr.get_current_branch()
            .unwrap_or_else(|| config.default_branch.clone());

        println!("📝 Committing changes...");
        git_mgr.commit_all("Update dotfiles")
            .context("Failed to commit changes")?;

        println!("📤 Pushing to GitHub...");
        let token = config.github.as_ref()
            .and_then(|gh| gh.token.as_deref());
        git_mgr.push("origin", &branch, token)
            .context("Failed to push to remote")?;

        println!("✅ Successfully pushed changes to GitHub!");
        Ok(())
    }

    fn cmd_pull() -> Result<()> {
        let config_path = crate::utils::get_config_path();

        let config = Config::load_or_create(&config_path)
            .context("Failed to load configuration")?;

        if config.github.is_none() {
            eprintln!("❌ GitHub not configured. Please run 'dotstate' to set up GitHub sync.");
            std::process::exit(1);
        }

        let repo_path = &config.repo_path;
        let git_mgr = GitManager::open_or_init(repo_path)
            .context("Failed to open repository")?;

        let branch = git_mgr.get_current_branch()
            .unwrap_or_else(|| config.default_branch.clone());

        println!("📥 Pulling from GitHub...");
        git_mgr.pull("origin", &branch)
            .context("Failed to pull from remote")?;

        println!("✅ Successfully pulled changes from GitHub!");
        Ok(())
    }

    fn cmd_list(verbose: bool) -> Result<()> {
        let config_path = crate::utils::get_config_path();

        let config = Config::load_or_create(&config_path)
            .context("Failed to load configuration")?;

        if config.synced_files.is_empty() {
            println!("No files are currently synced.");
            return Ok(());
        }

        println!("Synced files ({}):", config.synced_files.len());
        for file in &config.synced_files {
            if verbose {
                let full_path = dirs::home_dir()
                    .unwrap_or_default()
                    .join(file);
                if full_path.exists() {
                    println!("  ✓ {}", file);
                } else {
                    println!("  ✗ {} (not found)", file);
                }
            } else {
                println!("  {}", file);
            }
        }

        Ok(())
    }

    fn cmd_add(path: PathBuf) -> Result<()> {
        let config_path = crate::utils::get_config_path();

        let mut config = Config::load_or_create(&config_path)
            .context("Failed to load configuration")?;

        // Resolve path relative to home directory
        let home = dirs::home_dir()
            .context("Failed to get home directory")?;

        let resolved_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(path)
        };

        if !resolved_path.exists() {
            eprintln!("❌ File not found: {:?}", resolved_path);
            std::process::exit(1);
        }

        // Get relative path from home
        let relative_path = resolved_path.strip_prefix(&home)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| resolved_path.clone());

        let relative_str = relative_path.to_string_lossy().to_string();

        if config.synced_files.contains(&relative_str) {
            println!("ℹ️  File is already synced: {}", relative_str);
            return Ok(());
        }

        config.synced_files.push(relative_str.clone());
        config.save(&config_path)
            .context("Failed to save configuration")?;

        println!("✅ Added {} to synced files", relative_str);
        println!("💡 Run 'dotstate' to sync this file to GitHub");
        Ok(())
    }

    fn cmd_help(command: Option<String>) -> Result<()> {
        // This will be handled by loading help files from help/ folder
        // For now, just show a message
        if let Some(cmd) = command {
            println!("Help for '{}' command:", cmd);
            println!("(Help system coming soon - help files will be loaded from help/ folder)");
        } else {
            println!("Available commands:");
            println!("  push    - Push changes to GitHub");
            println!("  pull    - Pull changes from GitHub");
            println!("  list    - List all synced files");
            println!("  add     - Add a file to sync");
            println!("  help    - Show help for a command");
        }
        Ok(())
    }
}

