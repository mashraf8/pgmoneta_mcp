// Copyright (C) 2026 The pgmoneta community
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use pgmoneta_mcp::configuration::{self, UserConf};
use pgmoneta_mcp::security::SecurityUtil;
use rand::RngCore;
use rpassword::prompt_password;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(
    name = "pgmoneta-mcp-admin",
    about = "Administration utility for pgmoneta-mcp",
    version
)]
struct Args {
    /// The user configuration file
    #[arg(short = 'f', long)]
    file: Option<String>,

    /// The user name
    #[arg(short = 'U', long)]
    user: Option<String>,

    /// The password for the user
    #[arg(short = 'P', long)]
    password: Option<String>,

    /// Generate a password
    #[arg(short = 'g', long)]
    generate: bool,

    /// Password length (default: 64, ignored when --generate is false)
    #[arg(short = 'l', long, default_value = "64")]
    length: usize,

    /// Output format
    #[arg(short = 'F', long, value_enum, default_value = "text")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create or update the master key
    MasterKey,
    /// Manage a specific user
    User {
        #[command(subcommand)]
        action: UserAction,
    },
}

#[derive(Subcommand, Debug)]
enum UserAction {
    /// Add a new user to configuration file
    Add,
    /// Remove an existing user
    Del,
    /// Change the password for an existing user
    Edit,
    /// List all available users
    Ls,
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Serialize, Deserialize)]
struct AdminResponse {
    command: String,
    outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    users: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generated_password: Option<String>,
}
fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::MasterKey => {
            MasterKey::set_master_key(
                args.password.as_deref(),
                args.generate,
                args.length,
                args.format,
            )?;
        }
        Commands::User { action } => {
            let file = args
                .file
                .as_ref()
                .ok_or_else(|| anyhow!("Missing required argument: -f, --file <FILE>"))?;

            match action {
                UserAction::Add => {
                    let user = args
                        .user
                        .as_ref()
                        .ok_or_else(|| anyhow!("Missing required argument: -U, --user <USER>"))?;
                    let password = User::get_or_generate_password(
                        args.password.as_deref(),
                        args.generate,
                        args.length,
                    )?;
                    User::add_user(file, user, &password, args.format)?;
                }
                UserAction::Del => {
                    let user = args
                        .user
                        .as_ref()
                        .ok_or_else(|| anyhow!("Missing required argument: -U, --user <USER>"))?;
                    User::remove_user(file, user, args.format)?;
                }
                UserAction::Edit => {
                    let user = args
                        .user
                        .as_ref()
                        .ok_or_else(|| anyhow!("Missing required argument: -U, --user <USER>"))?;
                    let password = User::get_or_generate_password(
                        args.password.as_deref(),
                        args.generate,
                        args.length,
                    )?;
                    User::edit_user(file, user, &password, args.format)?;
                }
                UserAction::Ls => {
                    User::list_users(file, args.format)?;
                }
            }
        }
    }

    Ok(())
}

struct User;
impl User {
    pub fn add_user(file: &str, user: &str, password: &str, format: OutputFormat) -> Result<()> {
        let path = Path::new(file);
        let sutil = SecurityUtil::new();
        let mut conf: UserConf;
        let (master_password, master_salt) = sutil.load_master_key().map_err(|e| {
            anyhow!(
                "Unable to load the master key, needed for adding user: {:?}",
                e
            )
        })?;
        let password_str =
            sutil.encrypt_to_base64_string(password.as_bytes(), &master_password, &master_salt)?;

        if !path.exists() || path.is_dir() {
            conf = HashMap::new();
            let mut user_conf: HashMap<String, String> = HashMap::new();
            user_conf.insert(user.to_string(), password_str);
            conf.insert("admins".to_string(), user_conf);
        } else {
            conf = configuration::load_user_configuration(file)?;
            if let Some(user_conf) = conf.get_mut("admins") {
                if user_conf.contains_key(user) {
                    return Err(anyhow!("User '{}' already exists", user));
                }
                user_conf.insert(user.to_string(), password_str);
            } else {
                return Err(anyhow!("Unable to find admins in user configuration"));
            }
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conf_str = serde_ini::to_string(&conf)?;
        fs::write(file, &conf_str)?;

        Self::print_response(
            format,
            AdminResponse {
                command: "user add".to_string(),
                outcome: "success".to_string(),
                users: Some(vec![user.to_string()]),
                generated_password: None,
            },
        );

        Ok(())
    }

    pub fn remove_user(file: &str, user: &str, format: OutputFormat) -> Result<()> {
        let path = Path::new(file);

        if !path.exists() {
            return Err(anyhow!("User file '{}' does not exist", file));
        }

        let mut conf = configuration::load_user_configuration(file)?;

        if let Some(user_conf) = conf.get_mut("admins") {
            if user_conf.remove(user).is_none() {
                return Err(anyhow!("User '{}' not found", user));
            }
        } else {
            return Err(anyhow!(
                "Unable to find admins section in user configuration"
            ));
        }

        let conf_str = serde_ini::to_string(&conf)?;
        fs::write(file, &conf_str)?;

        Self::print_response(
            format,
            AdminResponse {
                command: "user del".to_string(),
                outcome: "success".to_string(),
                users: Some(vec![user.to_string()]),
                generated_password: None,
            },
        );

        Ok(())
    }

    pub fn edit_user(file: &str, user: &str, password: &str, format: OutputFormat) -> Result<()> {
        let path = Path::new(file);
        let sutil = SecurityUtil::new();

        if !path.exists() {
            return Err(anyhow!("User file '{}' does not exist", file));
        }

        let (master_password, master_salt) = sutil.load_master_key().map_err(|e| {
            anyhow!(
                "Unable to load the master key, needed for editing user: {:?}",
                e
            )
        })?;

        let password_str =
            sutil.encrypt_to_base64_string(password.as_bytes(), &master_password, &master_salt)?;

        let mut conf = configuration::load_user_configuration(file)?;

        if let Some(user_conf) = conf.get_mut("admins") {
            if user_conf.get(user).is_none() {
                return Err(anyhow!("User '{}' not found", user));
            }
            user_conf.insert(user.to_string(), password_str);
        } else {
            return Err(anyhow!(
                "Unable to find admins section in user configuration"
            ));
        }

        let conf_str = serde_ini::to_string(&conf)?;
        fs::write(file, &conf_str)?;

        Self::print_response(
            format,
            AdminResponse {
                command: "user edit".to_string(),
                outcome: "success".to_string(),
                users: Some(vec![user.to_string()]),
                generated_password: None,
            },
        );

        Ok(())
    }

    pub fn list_users(file: &str, format: OutputFormat) -> Result<()> {
        let path = Path::new(file);

        if !path.exists() {
            Self::print_response(
                format,
                AdminResponse {
                    command: "user ls".to_string(),
                    outcome: "success".to_string(),
                    users: Some(vec![]),
                    generated_password: None,
                },
            );
            return Ok(());
        }

        let conf = configuration::load_user_configuration(file)?;
        let mut users: Vec<String> = conf
            .get("admins")
            .map(|user_conf| user_conf.keys().cloned().collect())
            .unwrap_or_default();
        users.sort_unstable();

        Self::print_response(
            format,
            AdminResponse {
                command: "user ls".to_string(),
                outcome: "success".to_string(),
                users: Some(users),
                generated_password: None,
            },
        );

        Ok(())
    }

    fn get_or_generate_password(
        password: Option<&str>,
        generate: bool,
        length: usize,
    ) -> Result<String> {
        if let Some(pwd) = password {
            return Ok(pwd.to_string());
        }

        if generate {
            let sutil = SecurityUtil::new();
            let generated = sutil.generate_password(length)?;
            println!("Generated password: {}", generated);
            return Ok(generated);
        }

        let pwd = prompt_password("Password: ")?;
        let verify = prompt_password("Verify password: ")?;

        if pwd != verify {
            return Err(anyhow!("Passwords do not match"));
        }

        Ok(pwd)
    }

    fn print_response(format: OutputFormat, response: AdminResponse) {
        match format {
            OutputFormat::Json => {
                if let Ok(json) = serde_json::to_string_pretty(&response) {
                    println!("{}", json);
                }
            }
            OutputFormat::Text => {
                println!("Command: {}", response.command);
                println!("Outcome: {}", response.outcome);
                if let Some(users) = &response.users
                    && !users.is_empty()
                {
                    println!("Users:");
                    for user in users {
                        println!("  - {}", user);
                    }
                }
                if let Some(pwd) = &response.generated_password {
                    println!("Generated password: {}", pwd);
                }
            }
        }
    }
}

struct MasterKey;

impl MasterKey {
    pub fn set_master_key(
        password: Option<&str>,
        generate: bool,
        length: usize,
        format: OutputFormat,
    ) -> Result<()> {
        let sutil = SecurityUtil::new();
        let final_password: String;

        let master_key = if let Some(pwd) = password {
            final_password = pwd.to_string();
            &final_password
        } else if generate {
            final_password = sutil.generate_password(length)?;
            println!("Generated master key: {}", final_password);
            &final_password
        } else {
            final_password = prompt_password("Please enter your master key: ")?;
            let m = prompt_password("Please enter your master key again: ")?;

            if final_password != m {
                return Err(anyhow!("Passwords do not match"));
            }
            &final_password
        };

        let mut salt = [0u8; 16];
        rand::rng().fill_bytes(&mut salt);
        sutil.write_master_key(master_key, &salt)?;

        User::print_response(
            format,
            AdminResponse {
                command: "master-key".to_string(),
                outcome: "success".to_string(),
                users: None,
                generated_password: if generate {
                    Some(final_password.clone())
                } else {
                    None
                },
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgmoneta_mcp::configuration::UserConf;
    use std::fs;
    use std::path::PathBuf;

    fn get_temp_file(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(name);
        path
    }

    #[test]
    fn test_add_edit_remove_user() {
        let temp_file = get_temp_file("pgmoneta_mcp_test_users.conf");
        let temp_file_str = temp_file.to_str().unwrap();

        // Ensure master key exists for testing
        let sutil = SecurityUtil::new();
        if sutil.load_master_key().is_err() {
            MasterKey::set_master_key(Some("test_master_pass"), false, 64, OutputFormat::Text)
                .unwrap();
        }

        // Clean up before test
        if temp_file.exists() {
            fs::remove_file(&temp_file).unwrap();
        }

        // Add a user
        User::add_user(temp_file_str, "test_user", "test_pass", OutputFormat::Text).unwrap();

        // Verify user was added
        let content_added = fs::read_to_string(&temp_file).unwrap();
        let users_added: UserConf = serde_ini::from_str(&content_added).unwrap();
        assert!(users_added.contains_key("admins"));
        assert!(users_added["admins"].contains_key("test_user"));
        assert!(!users_added["admins"]["test_user"].is_empty());

        // Edit the user
        User::edit_user(temp_file_str, "test_user", "new_pass", OutputFormat::Text).unwrap();

        // Verify user was edited
        let content_edited = fs::read_to_string(&temp_file).unwrap();
        let users_edited: UserConf = serde_ini::from_str(&content_edited).unwrap();
        assert_ne!(
            users_added["admins"]["test_user"],
            users_edited["admins"]["test_user"]
        );

        // Remove the user
        User::remove_user(temp_file_str, "test_user", OutputFormat::Text).unwrap();

        // Verify user was removed
        let content_removed = fs::read_to_string(&temp_file).unwrap();
        let users_removed: UserConf = serde_ini::from_str(&content_removed).unwrap();
        assert!(
            !users_removed
                .get("admins")
                .is_some_and(|a| a.contains_key("test_user"))
        );

        // Clean up after test
        if temp_file.exists() {
            fs::remove_file(&temp_file).unwrap();
        }
    }

    #[test]
    fn test_list_users() {
        let temp_file = get_temp_file("pgmoneta_mcp_test_users_list.conf");
        let temp_file_str = temp_file.to_str().unwrap();

        // Ensure master key exists for testing
        let sutil = SecurityUtil::new();
        if sutil.load_master_key().is_err() {
            MasterKey::set_master_key(Some("test_master_pass"), false, 64, OutputFormat::Text)
                .unwrap();
        }

        // Clean up before test
        if temp_file.exists() {
            fs::remove_file(&temp_file).unwrap();
        }

        User::add_user(temp_file_str, "user1", "pass1", OutputFormat::Text).unwrap();
        User::add_user(temp_file_str, "user2", "pass2", OutputFormat::Text).unwrap();

        // We can't easily capture stdout here, but we can ensure the function runs without error
        User::list_users(temp_file_str, OutputFormat::Text).unwrap();
        User::list_users(temp_file_str, OutputFormat::Json).unwrap();

        // Clean up after test
        if temp_file.exists() {
            fs::remove_file(&temp_file).unwrap();
        }
    }
}
