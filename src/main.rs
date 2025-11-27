//! cargo-tako - Command-line tool for developing TAKO smart contracts
//!
//! This tool provides a streamlined workflow for creating, building, testing,
//! and deploying TAKO smart contracts.

use clap::{Parser, Subcommand};
use colored::Colorize;

mod commands;
mod config;
mod error;
mod template;
mod util;

use commands::{build, init, test};
use error::Result;

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// TAKO smart contract development tool
    Tako(TakoArgs),
}

#[derive(Parser)]
struct TakoArgs {
    #[command(subcommand)]
    command: TakoCommands,
}

#[derive(Subcommand)]
enum TakoCommands {
    /// Create a new TAKO smart contract project
    New {
        /// Name of the project
        name: String,

        /// Path where to create the project (defaults to current directory)
        #[arg(long)]
        path: Option<String>,

        /// Use a specific template (default, erc20, erc721, empty)
        #[arg(long, default_value = "default")]
        template: String,
    },

    /// Initialize TAKO in an existing Rust project
    Init {
        /// Use a specific template (default, erc20, erc721, empty)
        #[arg(long, default_value = "default")]
        template: String,
    },

    /// Build the TAKO smart contract
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,

        /// TBPF architecture version (v0, v1, v2, v3, v4)
        #[arg(long, default_value = "v3", value_parser = ["v0", "v1", "v2", "v3", "v4"])]
        arch: String,

        /// Target to build for (auto-detected from arch if not specified)
        #[arg(long)]
        target: Option<String>,

        /// Verify the built contract
        #[arg(long)]
        verify: bool,

        /// Dump ELF information after build
        #[arg(long)]
        dump: bool,
    },

    /// Run tests for the smart contract
    Test {
        /// Run only tests matching this filter
        filter: Option<String>,

        /// Run tests in release mode
        #[arg(long)]
        release: bool,
    },

    /// Clean build artifacts
    Clean,

    /// Display contract information
    Info {
        /// Path to the contract binary
        #[arg(long)]
        contract: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Tako(args) => match args.command {
            TakoCommands::New {
                name,
                path,
                template,
            } => {
                println!("{} TAKO contract project...", "Creating".green().bold());
                init::create_new_project(&name, path.as_deref(), &template)?;
                println!();
                println!(
                    "{} Created contract project: {}",
                    "✓".green().bold(),
                    name.bold()
                );
                println!();
                println!("Next steps:");
                println!("  cd {name}");
                println!("  cargo tako build");
                println!("  cargo tako test");
            }

            TakoCommands::Init { template } => {
                println!(
                    "{} TAKO in current project...",
                    "Initializing".green().bold()
                );
                init::init_current_project(&template)?;
                println!();
                println!("{} TAKO initialized", "✓".green().bold());
            }

            TakoCommands::Build {
                release,
                arch,
                target,
                verify,
                dump,
            } => {
                println!("{} TAKO contract...", "Building".green().bold());
                let output = build::build_contract(release, &arch, target.as_deref())?;
                println!();
                println!("{} Built contract:", "✓".green().bold());
                println!("  Binary: {}", output.display());
                println!("  Size: {} bytes", util::file_size(&output)?);
                println!("  Arch: {}", arch);

                if verify {
                    println!();
                    println!("{} contract...", "Verifying".cyan().bold());
                    build::verify_contract(&output, &arch)?;
                    println!("{} Contract verified", "✓".green().bold());
                }

                if dump {
                    println!();
                    println!("{} ELF information...", "Dumping".cyan().bold());
                    build::dump_elf(&output)?;
                }
            }

            TakoCommands::Test { filter, release } => {
                println!("{} tests...", "Running".green().bold());
                test::run_tests(filter.as_deref(), release)?;
            }

            TakoCommands::Clean => {
                println!("{} build artifacts...", "Cleaning".green().bold());
                util::clean_build_artifacts()?;
                println!("{} Build artifacts removed", "✓".green().bold());
            }

            TakoCommands::Info { contract } => {
                println!("{} contract information...", "Reading".cyan().bold());
                util::show_contract_info(contract.as_deref())?;
            }
        },
    }

    Ok(())
}
