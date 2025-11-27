//! Test command implementation

use crate::error::{Error, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

/// Run tests for the TAKO smart contract
///
/// Executes `cargo test` with optional filtering and displays results.
/// Captures and forwards test output in real-time with colored formatting.
///
/// # Arguments
/// * `filter` - Optional test name filter (e.g., "test_counter_increment")
/// * `release` - Whether to run tests in release mode
///
/// # Examples
/// ```bash
/// cargo tako test                    # Run all tests
/// cargo tako test test_increment     # Run tests matching "test_increment"
/// cargo tako test --release          # Run tests in release mode
/// ```
pub fn run_tests(filter: Option<&str>, release: bool) -> Result<()> {
    println!("Running tests...");

    // Build cargo test command
    let mut cmd = Command::new("cargo");
    cmd.arg("test");

    if release {
        cmd.arg("--release");
    }

    // Add filter if specified
    if let Some(f) = filter {
        cmd.arg(f);
        println!("Filter: {f}");
    }

    // Configure command to capture output
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    // Execute the command
    let mut child = cmd
        .spawn()
        .map_err(|e| Error::TestFailed(format!("Failed to execute cargo test: {e}")))?;

    // Capture and display stdout in real-time
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(|r| r.ok()) {
            println!("{line}");
        }
    }

    // Wait for the process to complete
    let status = child
        .wait()
        .map_err(|e| Error::TestFailed(format!("Failed to wait for tests: {e}")))?;

    if !status.success() {
        // Capture stderr if available
        if let Some(stderr) = child.stderr {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(|r| r.ok()) {
                eprintln!("{line}");
            }
        }

        return Err(Error::TestFailed(format!(
            "Tests failed with exit code: {:?}",
            status.code()
        )));
    }

    println!();
    println!("✓ All tests passed");

    Ok(())
}
