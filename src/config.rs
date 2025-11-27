//! Configuration management for cargo-tako

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct TakoConfig {
    #[serde(default)]
    pub package: PackageConfig,

    #[serde(default)]
    pub contract: ContractConfig,

    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractConfig {
    #[serde(default = "default_entry")]
    pub entry: String,

    #[serde(default = "default_abi_version")]
    pub abi_version: String,
}

impl Default for ContractConfig {
    fn default() -> Self {
        Self {
            entry: default_entry(),
            abi_version: default_abi_version(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_target")]
    pub target: String,

    #[serde(default = "default_opt_level")]
    pub opt_level: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: default_target(),
            opt_level: default_opt_level(),
        }
    }
}

fn default_entry() -> String {
    "entrypoint".to_string()
}

fn default_abi_version() -> String {
    "1.0".to_string()
}

fn default_target() -> String {
    "tbpf-tos-tos".to_string()
}

fn default_opt_level() -> String {
    "z".to_string()
}

impl TakoConfig {
    #[allow(dead_code)]
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: TakoConfig = toml::from_str(&content)?;
        Ok(config)
    }

    #[allow(dead_code)]
    pub fn load_from_current_dir() -> Result<Self> {
        Self::load_from_file("Tako.toml")
    }

    #[allow(dead_code)]
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| Error::Config(e.to_string()))?;
        fs::write(path, content)?;
        Ok(())
    }
}
