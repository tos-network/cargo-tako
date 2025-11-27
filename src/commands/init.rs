//! Project initialization command

use crate::error::{Error, Result};
use crate::template::{get_template, process_template};
use crate::util::{ensure_dir, write_file};
use std::path::PathBuf;
use std::process::Command;

/// Create a new TAKO project
///
/// Creates a new directory with the specified name and initializes it with
/// a TAKO smart contract template.
///
/// # Arguments
/// * `name` - Project name (will be used as directory name)
/// * `path` - Optional parent directory path (defaults to current directory)
/// * `template` - Template name (e.g., "default", "erc20", "erc721")
pub fn create_new_project(name: &str, path: Option<&str>, template: &str) -> Result<()> {
    // Determine project root directory
    let project_root = if let Some(parent) = path {
        PathBuf::from(parent).join(name)
    } else {
        PathBuf::from(name)
    };

    // Check if directory already exists
    if project_root.exists() {
        return Err(Error::ProjectExists(name.to_string()));
    }

    // Create project directory
    ensure_dir(&project_root)?;

    // Create src directory
    let src_dir = project_root.join("src");
    ensure_dir(&src_dir)?;

    // Create .cargo directory
    let cargo_dir = project_root.join(".cargo");
    ensure_dir(&cargo_dir)?;

    // Get template
    let tmpl = get_template(template)?;

    // Process template placeholders
    let cargo_toml = process_template(&tmpl.cargo_toml, name);
    let lib_rs = process_template(&tmpl.lib_rs, name);
    let readme = process_template(&tmpl.readme, name);

    // Write files
    write_file(project_root.join("Cargo.toml"), &cargo_toml)?;
    write_file(src_dir.join("lib.rs"), &lib_rs)?;
    write_file(project_root.join("README.md"), &readme)?;

    // Create .cargo/config.toml for TBPF target
    // Note: We don't set a default target to allow native tests
    // Use `cargo tako build` or `cargo build --target tbpf-tos-tos` for TBPF builds
    let cargo_config = r#"# TAKO Contract Build Configuration
#
# For development and testing:
#   cargo test                    # Run tests with native target
#
# For TBPF deployment build:
#   cargo tako build --release    # Build with tbpf-tos-tos target
#   cargo build --target tbpf-tos-tos --release

[target.tbpf-tos-tos]
linker = "rust-lld"
"#;
    write_file(cargo_dir.join("config.toml"), cargo_config)?;

    // Initialize git repository
    let _ = Command::new("git")
        .args(["init"])
        .current_dir(&project_root)
        .output();

    // Create .gitignore
    let gitignore = "target/\n*.log\n*.so\nCargo.lock\n";
    write_file(project_root.join(".gitignore"), gitignore)?;

    // Run cargo check to verify project
    println!("Verifying project...");
    let check_result = Command::new("cargo")
        .args(["check"])
        .current_dir(&project_root)
        .output();

    match check_result {
        Ok(output) if output.status.success() => {
            println!("✓ Project verified successfully");
        }
        Ok(output) => {
            eprintln!("Warning: cargo check failed:");
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            eprintln!("Warning: Could not run cargo check: {e}");
        }
    }

    Ok(())
}

/// Initialize TAKO in an existing Rust project
///
/// Adds TAKO dependencies and creates a template contract in the current directory.
///
/// # Arguments
/// * `template` - Template name (e.g., "default", "erc20", "erc721")
pub fn init_current_project(template: &str) -> Result<()> {
    let current_dir = std::env::current_dir()?;

    // Check if Cargo.toml exists
    let cargo_toml_path = current_dir.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Err(Error::Other(
            "No Cargo.toml found in current directory. Use 'cargo tako new' instead.".to_string(),
        ));
    }

    // Check if src/lib.rs already exists
    let lib_rs_path = current_dir.join("src/lib.rs");
    if lib_rs_path.exists() {
        return Err(Error::Other(
            "src/lib.rs already exists. Remove it or use a fresh project.".to_string(),
        ));
    }

    // Get template
    let tmpl = get_template(template)?;

    // Get project name from directory
    let project_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-contract");

    // Process template
    let lib_rs = process_template(&tmpl.lib_rs, project_name);

    // Create src directory if it doesn't exist
    let src_dir = current_dir.join("src");
    ensure_dir(&src_dir)?;

    // Create .cargo directory and config
    let cargo_dir = current_dir.join(".cargo");
    ensure_dir(&cargo_dir)?;

    let cargo_config = r#"# TAKO Contract Build Configuration
#
# For development and testing:
#   cargo test                    # Run tests with native target
#
# For TBPF deployment build:
#   cargo tako build --release    # Build with tbpf-tos-tos target
#   cargo build --target tbpf-tos-tos --release

[target.tbpf-tos-tos]
linker = "rust-lld"
"#;
    write_file(cargo_dir.join("config.toml"), cargo_config)?;

    // Write lib.rs
    write_file(&lib_rs_path, &lib_rs)?;

    println!("✓ TAKO contract initialized");
    println!();
    println!("Next steps:");
    println!("  1. Add TAKO dependencies to Cargo.toml:");
    println!("     [dependencies]");
    println!("     tako-macros = {{ git = \"https://github.com/tos-network/tako\" }}");
    println!("     tako-storage = {{ git = \"https://github.com/tos-network/tako\" }}");
    println!();
    println!("  2. Set crate type to cdylib:");
    println!("     [lib]");
    println!("     crate-type = [\"cdylib\"]");
    println!();
    println!("  3. Run: cargo tako build");

    Ok(())
}
