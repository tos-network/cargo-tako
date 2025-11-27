//! Project templates for cargo-tako

use crate::error::{Error, Result};

pub struct Template {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    pub cargo_toml: String,
    pub lib_rs: String,
    pub readme: String,
}

pub fn get_template(name: &str) -> Result<Template> {
    match name {
        "default" => Ok(default_template()),
        "erc20" => Ok(erc20_template()),
        "erc721" => Ok(erc721_template()),
        "empty" => Ok(empty_template()),
        _ => Err(Error::InvalidTemplate(name.to_string())),
    }
}

#[allow(dead_code)]
pub fn list_templates() -> Vec<&'static str> {
    vec!["default", "erc20", "erc721", "empty"]
}

fn default_template() -> Template {
    Template {
        name: "default".to_string(),
        description: "Basic counter contract".to_string(),
        cargo_toml: include_str!("../templates/default/Cargo.toml.template").to_string(),
        lib_rs: include_str!("../templates/default/lib.rs.template").to_string(),
        readme: include_str!("../templates/default/README.md.template").to_string(),
    }
}

fn erc20_template() -> Template {
    Template {
        name: "erc20".to_string(),
        description: "ERC-20 fungible token".to_string(),
        cargo_toml: include_str!("../templates/erc20/Cargo.toml.template").to_string(),
        lib_rs: include_str!("../templates/erc20/lib.rs.template").to_string(),
        readme: include_str!("../templates/erc20/README.md.template").to_string(),
    }
}

fn erc721_template() -> Template {
    Template {
        name: "erc721".to_string(),
        description: "ERC-721 non-fungible token (NFT)".to_string(),
        cargo_toml: include_str!("../templates/erc721/Cargo.toml.template").to_string(),
        lib_rs: include_str!("../templates/erc721/lib.rs.template").to_string(),
        readme: include_str!("../templates/erc721/README.md.template").to_string(),
    }
}

fn empty_template() -> Template {
    Template {
        name: "empty".to_string(),
        description: "Minimal boilerplate".to_string(),
        cargo_toml: include_str!("../templates/empty/Cargo.toml.template").to_string(),
        lib_rs: include_str!("../templates/empty/lib.rs.template").to_string(),
        readme: include_str!("../templates/empty/README.md.template").to_string(),
    }
}

/// Replace placeholders in template string
pub fn process_template(content: &str, project_name: &str) -> String {
    content
        .replace("{{project_name}}", &to_pascal_case(project_name))
        .replace("{{project_name_snake}}", &to_snake_case(project_name))
        .replace("{{project_name_kebab}}", &to_kebab_case(project_name))
}

fn to_pascal_case(s: &str) -> String {
    s.split(&['-', '_'][..])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

fn to_snake_case(s: &str) -> String {
    s.replace('-', "_").to_lowercase()
}

fn to_kebab_case(s: &str) -> String {
    s.replace('_', "-").to_lowercase()
}
