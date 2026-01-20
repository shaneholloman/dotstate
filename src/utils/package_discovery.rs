//! Package discovery module for detecting installed system packages.
//!
//! This module provides functionality to discover packages installed via
//! various package managers, focusing on explicitly installed packages
//! (not dependencies).

use anyhow::{Context, Result};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use tracing::{info, warn};

/// A discovered package from a system package manager.
#[derive(Debug, Clone)]
pub struct DiscoveredPackage {
    /// The package name as known by the package manager
    pub package_name: String,
    /// The binary name (if detectable)
    pub binary_name: Option<String>,
    /// Description (if available)
    pub description: Option<String>,
    /// The package manager this was discovered from
    pub manager: DiscoverySource,
}

/// Source package manager for discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySource {
    Homebrew,
    Pacman,
    Apt,
    Dnf,
    // Add more as needed
}

impl DiscoverySource {
    /// Get display name for the source.
    pub fn display_name(&self) -> &'static str {
        match self {
            DiscoverySource::Homebrew => "Homebrew",
            DiscoverySource::Pacman => "Pacman",
            DiscoverySource::Apt => "APT",
            DiscoverySource::Dnf => "DNF",
        }
    }

    /// Convert to the profile manifest PackageManager type.
    pub fn to_package_manager(&self) -> crate::utils::profile_manifest::PackageManager {
        match self {
            DiscoverySource::Homebrew => crate::utils::profile_manifest::PackageManager::Brew,
            DiscoverySource::Pacman => crate::utils::profile_manifest::PackageManager::Pacman,
            DiscoverySource::Apt => crate::utils::profile_manifest::PackageManager::Apt,
            DiscoverySource::Dnf => crate::utils::profile_manifest::PackageManager::Dnf,
        }
    }
}

/// Status updates from async discovery
#[derive(Debug, Clone)]
pub enum DiscoveryStatus {
    /// Discovery started for a source
    Started(DiscoverySource),
    /// Discovery completed successfully
    Complete {
        source: DiscoverySource,
        packages: Vec<DiscoveredPackage>,
    },
    /// Discovery failed
    Failed {
        source: DiscoverySource,
        error: String,
    },
    /// No package managers available
    NoSourcesAvailable,
}

/// Trait for package manager discovery implementations.
pub trait PackageDiscoverer: Send + Sync {
    /// Check if this package manager is available on the system.
    fn is_available(&self) -> bool;

    /// Get the discovery source type.
    fn source(&self) -> DiscoverySource;

    /// Discover explicitly installed packages.
    fn discover_packages(&self) -> Result<Vec<DiscoveredPackage>>;

    /// Try to detect the binary name for a package.
    fn detect_binary_name(&self, package_name: &str) -> Option<String>;
}

/// Homebrew package discoverer.
pub struct HomebrewDiscoverer;

impl PackageDiscoverer for HomebrewDiscoverer {
    fn is_available(&self) -> bool {
        Command::new("brew")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn source(&self) -> DiscoverySource {
        DiscoverySource::Homebrew
    }

    fn discover_packages(&self) -> Result<Vec<DiscoveredPackage>> {
        info!("Discovering Homebrew packages...");

        // Use `brew leaves` to get only explicitly installed packages (not dependencies)
        let output = Command::new("brew")
            .args(["leaves", "--installed-on-request"])
            .output()
            .context("Failed to run brew leaves")?;

        if !output.status.success() {
            // Fallback to just `brew leaves` if --installed-on-request isn't supported
            let output = Command::new("brew")
                .arg("leaves")
                .output()
                .context("Failed to run brew leaves")?;

            if !output.status.success() {
                anyhow::bail!("brew leaves failed");
            }

            return self.parse_leaves_output(&output.stdout);
        }

        self.parse_leaves_output(&output.stdout)
    }

    fn detect_binary_name(&self, package_name: &str) -> Option<String> {
        // Try to get the binary name from brew
        // First, check if a binary with the same name exists
        let check = Command::new("which").arg(package_name).output().ok()?;

        if check.status.success() {
            return Some(package_name.to_string());
        }

        // Try to list files installed by the package and find executables
        let output = Command::new("brew")
            .args(["list", "--formula", package_name])
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Look for files in bin/ directories
            for line in stdout.lines() {
                if line.contains("/bin/") {
                    if let Some(binary) = line.split('/').next_back() {
                        if !binary.is_empty() {
                            return Some(binary.to_string());
                        }
                    }
                }
            }
        }

        // Default to package name
        Some(package_name.to_string())
    }
}

impl HomebrewDiscoverer {
    fn parse_leaves_output(&self, stdout: &[u8]) -> Result<Vec<DiscoveredPackage>> {
        let stdout = String::from_utf8_lossy(stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let package_name = line.trim();
            if package_name.is_empty() {
                continue;
            }

            // Use package name as binary name by default (fast)
            // Skip the slow detect_binary_name call - user can edit if needed
            packages.push(DiscoveredPackage {
                package_name: package_name.to_string(),
                binary_name: Some(package_name.to_string()),
                description: None,
                manager: DiscoverySource::Homebrew,
            });
        }

        info!("Discovered {} Homebrew packages", packages.len());
        Ok(packages)
    }
}

/// Pacman package discoverer (Arch Linux).
pub struct PacmanDiscoverer;

impl PackageDiscoverer for PacmanDiscoverer {
    fn is_available(&self) -> bool {
        Command::new("pacman")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn source(&self) -> DiscoverySource {
        DiscoverySource::Pacman
    }

    fn discover_packages(&self) -> Result<Vec<DiscoveredPackage>> {
        info!("Discovering Pacman packages...");

        // Use `pacman -Qe` to get explicitly installed packages
        let output = Command::new("pacman")
            .args(["-Qe", "-q"]) // -q for quiet (just names)
            .output()
            .context("Failed to run pacman -Qe")?;

        if !output.status.success() {
            anyhow::bail!("pacman -Qe failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let package_name = line.trim();
            if package_name.is_empty() {
                continue;
            }

            // Use package name as binary name by default (fast)
            packages.push(DiscoveredPackage {
                package_name: package_name.to_string(),
                binary_name: Some(package_name.to_string()),
                description: None,
                manager: DiscoverySource::Pacman,
            });
        }

        info!("Discovered {} Pacman packages", packages.len());
        Ok(packages)
    }

    #[allow(dead_code)]
    fn detect_binary_name(&self, package_name: &str) -> Option<String> {
        // Check if binary with same name exists
        let check = Command::new("which").arg(package_name).output().ok()?;

        if check.status.success() {
            return Some(package_name.to_string());
        }

        // Try pacman -Ql to list files
        let output = Command::new("pacman")
            .args(["-Ql", package_name])
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("/usr/bin/") || line.contains("/bin/") {
                    if let Some(binary) = line.split_whitespace().last() {
                        if let Some(name) = binary.split('/').next_back() {
                            if !name.is_empty() {
                                return Some(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        Some(package_name.to_string())
    }
}

/// APT package discoverer (Debian/Ubuntu).
pub struct AptDiscoverer;

impl PackageDiscoverer for AptDiscoverer {
    fn is_available(&self) -> bool {
        Command::new("apt-mark")
            .arg("--help")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn source(&self) -> DiscoverySource {
        DiscoverySource::Apt
    }

    fn discover_packages(&self) -> Result<Vec<DiscoveredPackage>> {
        info!("Discovering APT packages...");

        // Use `apt-mark showmanual` to get manually installed packages
        let output = Command::new("apt-mark")
            .arg("showmanual")
            .output()
            .context("Failed to run apt-mark showmanual")?;

        if !output.status.success() {
            anyhow::bail!("apt-mark showmanual failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let package_name = line.trim();
            if package_name.is_empty() {
                continue;
            }

            // Use package name as binary name by default (fast)
            packages.push(DiscoveredPackage {
                package_name: package_name.to_string(),
                binary_name: Some(package_name.to_string()),
                description: None,
                manager: DiscoverySource::Apt,
            });
        }

        info!("Discovered {} APT packages", packages.len());
        Ok(packages)
    }

    #[allow(dead_code)]
    fn detect_binary_name(&self, package_name: &str) -> Option<String> {
        // Check if binary with same name exists
        let check = Command::new("which").arg(package_name).output().ok()?;

        if check.status.success() {
            return Some(package_name.to_string());
        }

        // Try dpkg -L to list files
        let output = Command::new("dpkg")
            .args(["-L", package_name])
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("/usr/bin/") || line.contains("/bin/") {
                    if let Some(name) = line.split('/').next_back() {
                        if !name.is_empty() {
                            return Some(name.to_string());
                        }
                    }
                }
            }
        }

        Some(package_name.to_string())
    }
}

/// Main discovery service that aggregates all discoverers.
pub struct PackageDiscoveryService {
    discoverers: Vec<Box<dyn PackageDiscoverer>>,
}

impl Default for PackageDiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageDiscoveryService {
    pub fn new() -> Self {
        let discoverers: Vec<Box<dyn PackageDiscoverer>> = vec![
            Box::new(HomebrewDiscoverer),
            Box::new(PacmanDiscoverer),
            Box::new(AptDiscoverer),
        ];

        Self { discoverers }
    }

    /// Get available package managers on this system.
    pub fn available_sources(&self) -> Vec<DiscoverySource> {
        self.discoverers
            .iter()
            .filter(|d| d.is_available())
            .map(|d| d.source())
            .collect()
    }

    /// Discover packages from a specific source.
    pub fn discover_from(&self, source: DiscoverySource) -> Result<Vec<DiscoveredPackage>> {
        for discoverer in &self.discoverers {
            if discoverer.source() == source {
                return discoverer.discover_packages();
            }
        }
        anyhow::bail!("No discoverer for source {:?}", source)
    }

    /// Discover packages from all available sources.
    pub fn discover_all(&self) -> Vec<DiscoveredPackage> {
        let mut all_packages = Vec::new();

        for discoverer in &self.discoverers {
            if discoverer.is_available() {
                match discoverer.discover_packages() {
                    Ok(packages) => all_packages.extend(packages),
                    Err(e) => {
                        warn!(
                            "Failed to discover packages from {:?}: {}",
                            discoverer.source(),
                            e
                        );
                    }
                }
            }
        }

        all_packages
    }

    /// Start async discovery - returns a receiver for status updates.
    /// The discovery runs in a background thread and sends status updates.
    pub fn discover_async() -> mpsc::Receiver<DiscoveryStatus> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let service = PackageDiscoveryService::new();
            let sources = service.available_sources();

            if sources.is_empty() {
                let _ = tx.send(DiscoveryStatus::NoSourcesAvailable);
                return;
            }

            // Use the first available source
            let source = sources[0];
            let _ = tx.send(DiscoveryStatus::Started(source));

            match service.discover_from(source) {
                Ok(packages) => {
                    let _ = tx.send(DiscoveryStatus::Complete { source, packages });
                }
                Err(e) => {
                    let _ = tx.send(DiscoveryStatus::Failed {
                        source,
                        error: e.to_string(),
                    });
                }
            }
        });

        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_source_display_name() {
        assert_eq!(DiscoverySource::Homebrew.display_name(), "Homebrew");
        assert_eq!(DiscoverySource::Pacman.display_name(), "Pacman");
        assert_eq!(DiscoverySource::Apt.display_name(), "APT");
    }

    #[test]
    fn test_service_creation() {
        let service = PackageDiscoveryService::new();
        // Just verify it doesn't panic
        let _ = service.available_sources();
    }
}
