//! Utility functions for cargo-tako

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Get the size of a file in bytes
pub fn file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
    let metadata = fs::metadata(path)?;
    Ok(metadata.len())
}

/// Clean build artifacts
pub fn clean_build_artifacts() -> Result<()> {
    let target_dir = Path::new("target");
    if target_dir.exists() {
        fs::remove_dir_all(target_dir)?;
    }
    Ok(())
}

/// Find the contract binary in target directory
///
/// Searches for the built contract (.so file) in the target directory.
/// Tries multiple possible locations:
/// 1. target/{target}/{profile}/*.so (for cross-compilation)
/// 2. target/{profile}/*.so (for native builds)
///
/// # Arguments
/// * `release` - Whether to look in release or debug directory
///
/// # Returns
/// Path to the contract binary
pub fn find_contract_binary(release: bool) -> Result<PathBuf> {
    find_contract_binary_for_target(release, "tbpf-tos-tos")
}

/// Get the package name from Cargo.toml in current directory
fn get_package_name() -> Option<String> {
    let cargo_toml = fs::read_to_string("Cargo.toml").ok()?;
    // Simple parsing - look for name = "..." in [package] section
    for line in cargo_toml.lines() {
        let line = line.trim();
        if line.starts_with("name") && line.contains('=') {
            // Extract the value after '='
            if let Some(value) = line.split('=').nth(1) {
                let name = value.trim().trim_matches('"').trim_matches('\'');
                return Some(name.replace('-', "_")); // Rust converts - to _ in binary names
            }
        }
    }
    None
}

/// Find the contract binary for a specific target
///
/// # Arguments
/// * `release` - Whether to look in release or debug directory
/// * `target` - Target triple (e.g., "tbpfv3-tos-tos")
///
/// # Returns
/// Path to the contract binary
pub fn find_contract_binary_for_target(release: bool, target: &str) -> Result<PathBuf> {
    let profile = if release { "release" } else { "debug" };
    let package_name = get_package_name();

    // Try multiple possible locations:
    // 1. Local target directory (standalone project)
    // 2. Parent's target directory (workspace member)
    // 3. Grandparent's target directory (nested workspace)
    let target_dirs = vec![
        format!("target/{}/{}", target, profile),
        format!("target/{}", profile),
        format!("../target/{}/{}", target, profile),
        format!("../../target/{}/{}", target, profile),
    ];

    // First, try to find the specific package binary if we know the name
    if let Some(ref name) = package_name {
        for target_dir in &target_dirs {
            let specific_path = PathBuf::from(target_dir).join(format!("{}.so", name));
            if specific_path.exists() {
                return Ok(specific_path);
            }
        }
    }

    // Fall back to finding any .so file
    for target_dir in target_dirs {
        if let Ok(entries) = fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                // Accept .so (Linux/eBPF), .dylib (macOS), or .dll (Windows)
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if ext == "so" || ext == "dylib" || ext == "dll" {
                        // Prefer the main library (not in deps/)
                        if !path.to_string_lossy().contains("/deps/") {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }

    Err(Error::BuildFailed(format!(
        "Contract binary (.so/.dylib/.dll) not found in target/{target}/{profile}"
    )))
}

/// Show contract information
pub fn show_contract_info(contract_path: Option<&str>) -> Result<()> {
    let path = if let Some(p) = contract_path {
        PathBuf::from(p)
    } else {
        find_contract_binary(false)?
    };

    if !path.exists() {
        return Err(Error::Other(format!(
            "Contract not found: {}",
            path.display()
        )));
    }

    let size = file_size(&path)?;
    println!("Contract Information:");
    println!("  Path: {}", path.display());
    println!("  Size: {} bytes ({:.2} KB)", size, size as f64 / 1024.0);

    // Try to read ELF header
    let content = fs::read(&path)?;
    if content.len() >= 4 && &content[0..4] == b"\x7FELF" {
        println!("  Format: ELF (valid)");
        if content.len() >= 5 {
            let class = match content[4] {
                1 => "32-bit",
                2 => "64-bit",
                _ => "unknown",
            };
            println!("  Class: {class}");
        }
    } else {
        println!("  Format: Invalid (not ELF)");
    }

    Ok(())
}

/// Create directory if it doesn't exist
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Write file with content
pub fn write_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}

/// Check if cargo is available
#[allow(dead_code)]
pub fn check_cargo_available() -> Result<()> {
    which::which("cargo").map_err(|_| Error::Other("cargo not found in PATH".to_string()))?;
    Ok(())
}
