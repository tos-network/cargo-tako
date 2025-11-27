//! Build command implementation

use crate::error::{Error, Result};
use crate::util::find_contract_binary_for_target;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get target triple from architecture version (aligned with Solana's cargo-build-sbf)
fn get_target_triple(arch: &str) -> String {
    if arch == "v0" {
        "tbpf-tos-tos".to_string()
    } else {
        format!("tbpf{}-tos-tos", arch) // tbpfv3-tos-tos
    }
}

/// Get expected e_flags for architecture version
fn get_expected_flags(arch: &str) -> u32 {
    match arch {
        "v0" => 0x0,
        "v1" => 0x1,
        "v2" => 0x2,
        "v3" => 0x3,
        "v4" => 0x4,
        _ => 0x0,
    }
}

/// Find the TOS platform-tools toolchain directory
fn find_platform_tools() -> Option<PathBuf> {
    // Try to find platform-tools relative to the tos-network directory
    // Check common locations
    let home = std::env::var("HOME").ok()?;
    let candidates = [
        format!("{}/tos-network/platform-tools/out/rust/bin", home),
        format!("{}/.tos/platform-tools/rust/bin", home),
        "/usr/local/tos/platform-tools/rust/bin".to_string(),
    ];

    for path in candidates {
        let rustc_path = PathBuf::from(&path).join("rustc");
        if rustc_path.exists() {
            return Some(PathBuf::from(path));
        }
    }

    None
}

/// Build a TAKO smart contract
///
/// Compiles the contract for the specified target architecture.
/// Default architecture is V3 (aligned with Solana production).
///
/// # Arguments
/// * `release` - Whether to build in release mode (optimized)
/// * `arch` - Architecture version (v0, v1, v2, v3, v4)
/// * `target` - Optional target override (auto-detected from arch if not specified)
///
/// # Returns
/// Path to the built contract binary (.so file)
pub fn build_contract(release: bool, arch: &str, target: Option<&str>) -> Result<PathBuf> {
    // Determine target from arch or use override
    let target = target
        .map(|t| t.to_string())
        .unwrap_or_else(|| get_target_triple(arch));

    // Determine build profile
    let profile = if release { "release" } else { "debug" };

    println!("  Arch: {arch}");
    println!("  Target: {target}");
    println!("  Profile: {profile}");

    // Find TOS platform-tools
    let platform_tools = find_platform_tools();
    if let Some(ref tools_path) = platform_tools {
        println!("  Toolchain: {}", tools_path.display());
    } else {
        println!("  Toolchain: system (TOS platform-tools not found)");
        eprintln!("Warning: TOS platform-tools not found. TBPF targets may not be available.");
        eprintln!("Expected location: ~/tos-network/platform-tools/out/rust/bin/");
    }

    // Build cargo command - use TOS platform-tools cargo if available
    let cargo_bin = if let Some(ref tools_path) = platform_tools {
        let cargo_path = tools_path.join("cargo");
        if cargo_path.exists() {
            cargo_path.to_string_lossy().to_string()
        } else {
            "cargo".to_string()
        }
    } else {
        "cargo".to_string()
    };

    let mut cmd = Command::new(&cargo_bin);
    cmd.arg("build");

    if release {
        cmd.arg("--release");
    }

    cmd.arg("--target").arg(&target);

    // Add -Zbuild-std=core,alloc for building core and alloc libraries from source
    // This is required for TBPF V3+ targets as they don't have pre-built libraries
    // - core: basic types and traits (required)
    // - alloc: Vec, String, Box etc. (needed by most contracts)
    cmd.arg("-Zbuild-std=core,alloc");

    // Set TOS platform-tools as the Rust compiler if found
    if let Some(ref tools_path) = platform_tools {
        let rustc_path = tools_path.join("rustc");
        cmd.env("RUSTC", &rustc_path);
    }

    // Execute build
    println!(
        "Running: cargo build {} --target {} -Zbuild-std=core,alloc",
        if release { "--release" } else { "" },
        target
    );

    let output = cmd
        .output()
        .map_err(|e| Error::BuildFailed(format!("Failed to execute cargo: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::BuildFailed(format!("Build failed:\n{stderr}")));
    }

    // Find the built binary
    let binary_path = find_contract_binary_for_target(release, &target)?;

    println!("✓ Build successful");

    Ok(binary_path)
}

/// Verify a built contract binary
///
/// Checks that the contract is a valid ELF file and
/// meets requirements for the specified TBPF version.
///
/// # Arguments
/// * `path` - Path to the contract binary
/// * `arch` - Expected architecture version
///
/// # Checks
/// - File exists
/// - Valid ELF magic bytes (0x7F 'E' 'L' 'F')
/// - Correct e_flags for the architecture
/// - File size is reasonable
/// - 64-bit ELF format
pub fn verify_contract(path: &Path, arch: &str) -> Result<()> {
    use std::fs;

    println!("Verifying contract...");

    // Check file exists
    if !path.exists() {
        return Err(Error::BuildFailed(format!(
            "Contract binary not found: {}",
            path.display()
        )));
    }

    // Read file
    let contents =
        fs::read(path).map_err(|e| Error::BuildFailed(format!("Failed to read contract: {e}")))?;

    // Check minimum size
    if contents.len() < 64 {
        return Err(Error::BuildFailed(format!(
            "Contract file too small ({} bytes)",
            contents.len()
        )));
    }

    // Verify ELF magic
    if &contents[0..4] != b"\x7FELF" {
        return Err(Error::BuildFailed(
            "Invalid ELF file: wrong magic bytes".to_string(),
        ));
    }

    // Verify ELF class (64-bit)
    let elf_class = contents[4];
    if elf_class != 2 {
        return Err(Error::BuildFailed(format!(
            "Invalid ELF class: expected 64-bit (2), got {}",
            elf_class
        )));
    }

    // Verify e_flags (at offset 48 for ELF64)
    let e_flags = u32::from_le_bytes([contents[48], contents[49], contents[50], contents[51]]);
    let expected_flags = get_expected_flags(arch);

    if e_flags != expected_flags {
        return Err(Error::BuildFailed(format!(
            "Wrong e_flags: expected 0x{:x} for {}, got 0x{:x}",
            expected_flags, arch, e_flags
        )));
    }

    // Check file size (warn if too large)
    const MAX_REASONABLE_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if contents.len() > MAX_REASONABLE_SIZE {
        eprintln!("Warning: Contract is very large ({} bytes)", contents.len());
        eprintln!("Consider optimizing with --release flag");
    }

    println!("✓ Contract verified");
    println!("  Format: ELF 64-bit");
    println!("  e_flags: 0x{:x} ({})", e_flags, arch.to_uppercase());
    println!(
        "  Size: {} bytes ({:.2} KB)",
        contents.len(),
        contents.len() as f64 / 1024.0
    );
    println!(
        "  Type: TBPF {} contract (ready for deployment)",
        arch.to_uppercase()
    );

    Ok(())
}

/// Dump ELF information using llvm-readelf
pub fn dump_elf(path: &Path) -> Result<()> {
    println!("ELF dump for {}", path.display());
    println!();

    // Try llvm-readelf first, fall back to readelf
    let output = Command::new("llvm-readelf")
        .args(["-h", "-l", path.to_str().unwrap_or("")])
        .output()
        .or_else(|_| {
            Command::new("readelf")
                .args(["-h", "-l", path.to_str().unwrap_or("")])
                .output()
        })
        .map_err(|e| Error::BuildFailed(format!("Failed to run readelf: {e}")))?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Warning: readelf failed: {stderr}");
    }

    Ok(())
}
