//! Build command implementation

use crate::error::{Error, Result};
use crate::toolchain::{find_platform_tools, PlatformTools, DEFAULT_PLATFORM_TOOLS_VERSION};
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

    // Find TOS platform-tools (Solana-aligned search)
    let platform_tools = find_platform_tools(Some(DEFAULT_PLATFORM_TOOLS_VERSION));

    if let Some(ref tools) = platform_tools {
        println!("  Toolchain: {} ({})", tools.display_path(), tools.version);
    } else {
        println!("  Toolchain: system (TOS platform-tools not found)");
        eprintln!("Warning: TOS platform-tools not found. TBPF targets may not be available.");
        eprintln!("Expected locations:");
        eprintln!("  1. ~/.cache/tos/<version>/platform-tools/rust/bin/");
        eprintln!("  2. ~/tos-network/platform-tools/rust/bin/");
        eprintln!("  3. ~/.tos/platform-tools/rust/bin/");
    }

    // Build cargo command
    let (cargo_bin, rustc_env) = get_cargo_and_rustc(&platform_tools);

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
    if let Some(rustc) = rustc_env {
        cmd.env("RUSTC", &rustc);
    }

    // Set LLVM tools environment variables if available
    if let Some(ref tools) = platform_tools {
        if tools.llvm_bin.exists() {
            cmd.env("CC", tools.clang());
            cmd.env("AR", tools.llvm_ar());
            cmd.env("OBJDUMP", tools.llvm_objdump());
            cmd.env("OBJCOPY", tools.llvm_objcopy());
        }
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

/// Get cargo binary path and optional RUSTC environment variable
fn get_cargo_and_rustc(platform_tools: &Option<PlatformTools>) -> (String, Option<PathBuf>) {
    if let Some(ref tools) = platform_tools {
        let cargo = tools.cargo();
        let rustc = tools.rustc();

        if cargo.exists() {
            (cargo.to_string_lossy().to_string(), Some(rustc))
        } else if rustc.exists() {
            // Use system cargo but TOS rustc
            ("cargo".to_string(), Some(rustc))
        } else {
            ("cargo".to_string(), None)
        }
    } else {
        ("cargo".to_string(), None)
    }
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

    // Try to use platform-tools llvm-readelf first
    let platform_tools = find_platform_tools(None);

    let output = if let Some(ref tools) = platform_tools {
        let llvm_readelf = tools.llvm_bin.join("llvm-readelf");
        if llvm_readelf.exists() {
            Command::new(&llvm_readelf)
                .args(["-h", "-l", path.to_str().unwrap_or("")])
                .output()
                .ok()
        } else {
            None
        }
    } else {
        None
    };

    // Fall back to system tools
    let output = output.or_else(|| {
        Command::new("llvm-readelf")
            .args(["-h", "-l", path.to_str().unwrap_or("")])
            .output()
            .ok()
    }).or_else(|| {
        Command::new("readelf")
            .args(["-h", "-l", path.to_str().unwrap_or("")])
            .output()
            .ok()
    });

    match output {
        Some(out) if out.status.success() => {
            println!("{}", String::from_utf8_lossy(&out.stdout));
        }
        Some(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!("Warning: readelf failed: {stderr}");
        }
        None => {
            return Err(Error::BuildFailed("Failed to run readelf or llvm-readelf".to_string()));
        }
    }

    Ok(())
}
