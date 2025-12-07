//! Platform tools management module (aligned with Solana's cargo-build-sbf)
//!
//! This module handles the installation and management of TOS platform-tools,
//! following a similar pattern to Solana's toolchain management.
//!
//! ## Directory Structure
//!
//! ```text
//! ~/.cache/tos/<version>/platform-tools/
//! ├── rust/
//! │   ├── bin/
//! │   │   ├── cargo
//! │   │   └── rustc
//! │   └── lib/
//! │       └── rustlib/
//! │           └── tbpfv3-tos-tos/
//! └── llvm/
//!     └── bin/
//!         ├── clang
//!         ├── lld
//!         └── llvm-objdump
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;

/// Default platform-tools version
/// This should match the version of tos-platform-tools releases on GitHub
/// Format: v<major>.<minor> (e.g., v1.0, v1.52)
pub const DEFAULT_PLATFORM_TOOLS_VERSION: &str = "v1.52";

/// Default Rust version used in platform-tools
/// This is the rustc version bundled in platform-tools
#[allow(dead_code)]
pub const DEFAULT_RUST_VERSION: &str = "1.89.0";

/// Get home directory
pub fn home_dir() -> PathBuf {
    PathBuf::from(
        env::var_os("HOME")
            .or_else(|| {
                #[cfg(windows)]
                {
                    env::var_os("USERPROFILE")
                }
                #[cfg(not(windows))]
                {
                    None
                }
            })
            .expect("Cannot determine home directory"),
    )
}

/// Get the cache directory for TOS tools
/// Returns: ~/.cache/tos/
pub fn cache_dir() -> PathBuf {
    home_dir().join(".cache").join("tos")
}

/// Get the platform-tools path for a specific version
/// Returns: ~/.cache/tos/<version>/platform-tools/
pub fn platform_tools_path(version: &str) -> PathBuf {
    cache_dir().join(version).join("platform-tools")
}

/// Get the rust toolchain bin path for a specific version
/// Returns: ~/.cache/tos/<version>/platform-tools/rust/bin/
pub fn rust_bin_path(version: &str) -> PathBuf {
    platform_tools_path(version).join("rust").join("bin")
}

/// Get the llvm bin path for a specific version
/// Returns: ~/.cache/tos/<version>/platform-tools/llvm/bin/
pub fn llvm_bin_path(version: &str) -> PathBuf {
    platform_tools_path(version).join("llvm").join("bin")
}

/// Find all installed platform-tools versions
pub fn find_installed_versions() -> Vec<String> {
    let cache = cache_dir();
    if let Ok(dir) = fs::read_dir(&cache) {
        dir.filter_map(|e| match e {
            Err(_) => None,
            Ok(e) => {
                let path = e.path();
                if path.join("platform-tools").is_dir() {
                    Some(e.file_name().to_string_lossy().to_string())
                } else {
                    None
                }
            }
        })
        .collect()
    } else {
        Vec::new()
    }
}

/// Check if platform-tools is installed for a specific version
#[allow(dead_code)]
pub fn is_installed(version: &str) -> bool {
    let rustc = rust_bin_path(version).join("rustc");
    let cargo = rust_bin_path(version).join("cargo");
    rustc.exists() && cargo.exists()
}

/// Find the best available platform-tools installation
/// Search order:
/// 1. ~/.cache/tos/<version>/platform-tools/rust/bin/ (Solana-aligned, versioned)
/// 2. ~/tos-network/platform-tools/rust/bin/ (legacy, unversioned)
/// 3. ~/tos-network/platform-tools/out/rust/bin/ (build output)
/// 4. ~/.tos/platform-tools/rust/bin/ (user local)
/// 5. /usr/local/tos/platform-tools/rust/bin/ (system-wide)
pub fn find_platform_tools(version: Option<&str>) -> Option<PlatformTools> {
    let home = home_dir();

    // 1. Check versioned cache (Solana-aligned)
    if let Some(ver) = version {
        let versioned_path = rust_bin_path(ver);
        if versioned_path.join("rustc").exists() {
            return Some(PlatformTools {
                version: ver.to_string(),
                rust_bin: versioned_path,
                llvm_bin: llvm_bin_path(ver),
                source: ToolchainSource::VersionedCache,
            });
        }
    }

    // 2. Check for any installed version in cache
    let installed = find_installed_versions();
    if let Some(ver) = installed.first() {
        let versioned_path = rust_bin_path(ver);
        if versioned_path.join("rustc").exists() {
            return Some(PlatformTools {
                version: ver.clone(),
                rust_bin: versioned_path,
                llvm_bin: llvm_bin_path(ver),
                source: ToolchainSource::VersionedCache,
            });
        }
    }

    // 3. Legacy locations (unversioned)
    let legacy_candidates = [
        (
            home.join("tos-network/platform-tools/rust/bin"),
            home.join("tos-network/platform-tools/llvm/bin"),
        ),
        (
            home.join("tos-network/platform-tools/out/rust/bin"),
            home.join("tos-network/platform-tools/out/llvm/bin"),
        ),
        (
            home.join(".tos/platform-tools/rust/bin"),
            home.join(".tos/platform-tools/llvm/bin"),
        ),
        (
            PathBuf::from("/usr/local/tos/platform-tools/rust/bin"),
            PathBuf::from("/usr/local/tos/platform-tools/llvm/bin"),
        ),
    ];

    for (rust_bin, llvm_bin) in legacy_candidates {
        if rust_bin.join("rustc").exists() {
            return Some(PlatformTools {
                version: "unknown".to_string(),
                rust_bin,
                llvm_bin,
                source: ToolchainSource::Legacy,
            });
        }
    }

    None
}

/// Get the download filename for the current platform
#[allow(dead_code)]
pub fn get_download_filename() -> String {
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };

    if cfg!(target_os = "windows") {
        format!("tos-platform-tools-windows-{arch}.tar.bz2")
    } else if cfg!(target_os = "macos") {
        format!("tos-platform-tools-osx-{arch}.tar.bz2")
    } else {
        format!("tos-platform-tools-linux-{arch}.tar.bz2")
    }
}

/// Get the download URL for platform-tools
#[allow(dead_code)]
pub fn get_download_url(version: &str) -> String {
    let filename = get_download_filename();
    format!(
        "https://github.com/tos-network/platform-tools/releases/download/{version}/{filename}"
    )
}

/// Platform tools information
#[derive(Debug, Clone)]
pub struct PlatformTools {
    /// Version string (e.g., "v1.0.0")
    pub version: String,
    /// Path to rust/bin directory
    pub rust_bin: PathBuf,
    /// Path to llvm/bin directory
    pub llvm_bin: PathBuf,
    /// Source of the toolchain
    pub source: ToolchainSource,
}

impl PlatformTools {
    /// Get path to rustc
    pub fn rustc(&self) -> PathBuf {
        self.rust_bin.join("rustc")
    }

    /// Get path to cargo
    pub fn cargo(&self) -> PathBuf {
        self.rust_bin.join("cargo")
    }

    /// Get path to clang
    pub fn clang(&self) -> PathBuf {
        self.llvm_bin.join("clang")
    }

    /// Get path to lld
    #[allow(dead_code)]
    pub fn lld(&self) -> PathBuf {
        self.llvm_bin.join("lld")
    }

    /// Get path to llvm-objdump
    pub fn llvm_objdump(&self) -> PathBuf {
        self.llvm_bin.join("llvm-objdump")
    }

    /// Get path to llvm-objcopy
    pub fn llvm_objcopy(&self) -> PathBuf {
        self.llvm_bin.join("llvm-objcopy")
    }

    /// Get path to llvm-ar
    pub fn llvm_ar(&self) -> PathBuf {
        self.llvm_bin.join("llvm-ar")
    }

    /// Check if this toolchain is valid (all required binaries exist)
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        self.rustc().exists() && self.cargo().exists()
    }

    /// Get display string for the toolchain location
    pub fn display_path(&self) -> String {
        match self.source {
            ToolchainSource::VersionedCache => {
                format!("~/.cache/tos/{}/platform-tools", self.version)
            }
            ToolchainSource::Legacy => self.rust_bin.display().to_string(),
        }
    }
}

/// Source of the toolchain installation
#[derive(Debug, Clone, PartialEq)]
pub enum ToolchainSource {
    /// Installed in ~/.cache/tos/<version>/ (Solana-aligned)
    VersionedCache,
    /// Legacy location (unversioned)
    Legacy,
}

/// Install platform-tools from a local archive
#[allow(dead_code)]
pub fn install_from_archive(archive_path: &PathBuf, version: &str) -> Result<PathBuf, String> {
    use std::process::Command;

    let target_dir = cache_dir().join(version);
    let platform_tools_dir = target_dir.join("platform-tools");

    // Create target directory
    fs::create_dir_all(&target_dir).map_err(|e| format!("Failed to create directory: {e}"))?;

    // Check if already installed
    if platform_tools_dir.join("rust").join("bin").join("rustc").exists() {
        println!("Platform-tools {} already installed", version);
        return Ok(platform_tools_dir);
    }

    // Remove existing incomplete installation
    if platform_tools_dir.exists() {
        fs::remove_dir_all(&platform_tools_dir)
            .map_err(|e| format!("Failed to remove existing directory: {e}"))?;
    }

    println!("Installing platform-tools {} from {}", version, archive_path.display());

    // Extract archive using tar command (more reliable than Rust libraries)
    let status = Command::new("tar")
        .args(["-xjf", archive_path.to_str().unwrap(), "-C", target_dir.to_str().unwrap()])
        .status()
        .map_err(|e| format!("Failed to run tar: {e}"))?;

    if !status.success() {
        return Err("Failed to extract archive".to_string());
    }

    // Verify installation
    if !platform_tools_dir.join("rust").join("bin").join("rustc").exists() {
        return Err("Installation verification failed: rustc not found".to_string());
    }

    println!("✓ Platform-tools {} installed successfully", version);
    Ok(platform_tools_dir)
}

/// Print toolchain information
#[allow(dead_code)]
pub fn print_toolchain_info(tools: &PlatformTools) {
    println!("Platform-tools Information:");
    println!("  Version: {}", tools.version);
    println!("  Location: {}", tools.display_path());
    println!("  Source: {:?}", tools.source);
    println!("  Rustc: {}", tools.rustc().display());
    println!("  Cargo: {}", tools.cargo().display());
    if tools.llvm_bin.exists() {
        println!("  LLVM: {}", tools.llvm_bin.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_dir() {
        let home = home_dir();
        assert!(home.exists());
    }

    #[test]
    fn test_cache_dir() {
        let cache = cache_dir();
        assert!(cache.to_string_lossy().contains(".cache/tos"));
    }

    #[test]
    fn test_platform_tools_path() {
        let path = platform_tools_path("v1.0.0");
        assert!(path.to_string_lossy().contains("v1.0.0"));
        assert!(path.to_string_lossy().contains("platform-tools"));
    }

    #[test]
    fn test_get_download_filename() {
        let filename = get_download_filename();
        assert!(filename.starts_with("tos-platform-tools-"));
        assert!(filename.ends_with(".tar.bz2"));
    }
}
