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

use crate::cli::{CliExecute, ClientCli};
use crate::engine::ClientEngine;
use crate::{CallArgs, ClientCommands, OutputFormat, PrintArgs, ToolCommands};
use anyhow::{Result, anyhow, bail};
use inquire::{Select, Text};
use pgmoneta_mcp::utils::SafeFileReader;
use std::collections::HashMap;

/// Gracefully handles Inquire cancellations such as Esc or Ctrl+C
macro_rules! prompt_or_cancel {
    ($prompt_result:expr, $err_msg:expr) => {
        match $prompt_result {
            Ok(val) => val,
            Err(inquire::InquireError::OperationCanceled)
            | Err(inquire::InquireError::OperationInterrupted) => return Ok(None),
            Err(e) => anyhow::bail!("{}: {}", $err_msg, e),
        }
    };
}

pub async fn run_interactive_router() -> Result<()> {
    let client_version = env!("CARGO_PKG_VERSION");
    println!(
        "Welcome to pgmoneta MCP Interactive Shell! v{}",
        client_version
    );

    let options = vec!["Client", "Exit"];
    let choice = Select::new("Select a module:", options).prompt();

    match choice {
        Ok("Client") => {
            let wizard = ClientWizard;
            wizard.run().await?;
        }
        Ok("Exit")
        | Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => {}
        Ok(_) => {}
        Err(e) => anyhow::bail!("An error occurred: {}", e),
    }
    Ok(())
}

pub trait InteractiveWizard {
    #[allow(async_fn_in_trait)]
    async fn run(&self) -> Result<()>;
}

pub trait ClientInterface {
    #[allow(async_fn_in_trait)]
    async fn client_command(engine: &ClientEngine) -> Result<Option<ClientCommands>>;
}

pub struct ClientWizard;
pub struct PageTool;

impl InteractiveWizard for ClientWizard {
    async fn run(&self) -> Result<()> {
        let url_prompt = Text::new("Enter MCP server URL:").prompt();
        let url = match url_prompt {
            Ok(val) => val,
            Err(inquire::InquireError::OperationCanceled)
            | Err(inquire::InquireError::OperationInterrupted) => return Ok(()),
            Err(e) => anyhow::bail!("URL input failed: {}", e),
        };
        if url.trim().is_empty() {
            bail!("URL is required to connect to the MCP server.");
        }

        let timeout_input = match Text::new("Connection timeout in seconds (default 30):").prompt()
        {
            Ok(val) => val,
            Err(inquire::InquireError::OperationCanceled)
            | Err(inquire::InquireError::OperationInterrupted) => return Ok(()),
            Err(e) => anyhow::bail!("Timeout input failed: {}", e),
        };
        let timeout_secs: u64 = timeout_input.trim().parse().unwrap_or(30);

        let client = ClientEngine::connect(&url, timeout_secs).await?;

        println!("  Connected to: {}", url);
        if let Some((server_name, server_version)) = client.server_info() {
            println!("  Server: {} v{}", server_name, server_version);
        }

        let options = vec!["Tools", "Exit"];
        loop {
            let choice = Select::new("What would you like to manage?", options.clone()).prompt();

            match choice {
                Ok("Tools") => match PageTool::client_command(&client).await {
                    Ok(Some(cmd)) => {
                        let router = ClientCli::new(url.to_string(), timeout_secs, cmd);
                        if let Err(e) = router.execute().await {
                            eprintln!("Error executing command: {}", e);
                        }
                    }
                    Ok(None) => continue,
                    Err(e) => eprintln!("Error preparing command: {}", e),
                },
                Ok("Exit")
                | Err(inquire::InquireError::OperationCanceled)
                | Err(inquire::InquireError::OperationInterrupted) => break,
                Ok(_) => continue,
                Err(e) => bail!("An error occurred: {}", e),
            }
        }
        client.cleanup().await?;
        Ok(())
    }
}

impl ClientInterface for PageTool {
    async fn client_command(engine: &ClientEngine) -> Result<Option<ClientCommands>> {
        let options = vec!["List Tools", "Call Tool"];
        let choice = prompt_or_cancel!(
            Select::new("What would you like to do with tools?", options).prompt(),
            "Selection failed"
        );

        match choice {
            "List Tools" => Ok(Some(ClientCommands::Tool {
                action: ToolCommands::List {
                    print_opts: PrintArgs {
                        output: OutputFormat::Tree,
                    },
                },
            })),
            "Call Tool" => {
                let tools = engine.execute_list_tools().await?;

                if tools.is_empty() {
                    bail!("Notice: No tools are currently available on the server.");
                }

                let tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();

                let name = prompt_or_cancel!(
                    Select::new("Select a tool to call:", tool_names).prompt(),
                    "Tool selection failed"
                );

                let tool = tools
                    .iter()
                    .find(|t| t.name == name)
                    .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;

                match Self::build_calltool_args(&tool.input_schema)? {
                    Some(args) => Ok(Some(ClientCommands::Tool {
                        action: ToolCommands::Call {
                            call_args: CallArgs {
                                name: name.to_string(),
                                file: None,
                                args: serde_json::to_string(&args)
                                    .map_err(|e| anyhow!("Failed to serialize args: {}", e))?,
                            },
                            print_opts: PrintArgs {
                                output: OutputFormat::Tree,
                            },
                        },
                    })),
                    None => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }
}

impl PageTool {
    fn extract_property_info(
        prop_name: &str,
        prop_schema: &serde_json::Value,
    ) -> (String, String, String) {
        let prop_type = prop_schema
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown type");
        let desc = prop_schema
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("unknown description");

        let short_desc = desc.chars().take(30).collect::<String>();
        let prompt = format!("{} [{}] [{}] =", prop_name, prop_type, short_desc);

        (prop_type.to_string(), desc.to_string(), prompt)
    }

    fn build_calltool_args(
        schema: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Option<HashMap<String, serde_json::Value>>> {
        let mut args = HashMap::new();
        let props = schema
            .get("properties")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow!("Tool schema missing properties"))?;

        for (prop_name, prop_schema) in props {
            let (_, _, prompt) = Self::extract_property_info(prop_name, prop_schema);

            let v = prompt_or_cancel!(Text::new(&prompt).prompt(), "Input failed");

            if !v.trim().is_empty() {
                let content = if let Some(path) = v.trim().strip_prefix('@') {
                    SafeFileReader::new()
                        .max_size(10 * 1024 * 1024)
                        .read(path)?
                } else {
                    v
                };

                let parsed =
                    serde_json::from_str(&content).map_err(|e| anyhow!("Invalid JSON: {}", e))?;
                args.insert(prop_name.clone(), parsed);
            }
        }
        Ok(Some(args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_property_info_scenarios() {
        // Test with type and description
        let schema_full = json!({"type": "string", "description": "The server name"});
        let (ptype, desc, prompt) = PageTool::extract_property_info("server", &schema_full);
        assert_eq!(ptype, "string");
        assert_eq!(desc, "The server name");
        assert!(prompt.contains("server"));
        assert!(prompt.contains("[string]"));
        assert!(prompt.contains("[The server name]"));

        // Test missing type
        let schema_no_type = json!({"description": "Some description"});
        let (ptype, _, _) = PageTool::extract_property_info("field", &schema_no_type);
        assert_eq!(ptype, "unknown type");

        // Test missing description
        let schema_no_desc = json!({"type": "integer"});
        let (_, desc, _) = PageTool::extract_property_info("field", &schema_no_desc);
        assert_eq!(desc, "unknown description");

        // Test long description truncated
        let long_desc =
            "This is a very long description that should be truncated to thirty characters";
        let schema_long = json!({"type": "string", "description": long_desc});
        let (_, _, prompt) = PageTool::extract_property_info("field", &schema_long);
        let short: String = long_desc.chars().take(30).collect();
        assert!(prompt.contains(&short));
        assert!(!prompt.contains(long_desc));
    }

    #[test]
    fn test_build_calltool_args_scenarios() {
        // Test missing properties
        let schema_missing: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        let result_missing = PageTool::build_calltool_args(&schema_missing);
        assert!(result_missing.is_err());
        assert!(format!("{:?}", result_missing.unwrap_err()).contains("missing properties"));

        // Test empty properties
        let mut schema_empty = serde_json::Map::new();
        schema_empty.insert("properties".to_string(), json!({}));
        let result_empty = PageTool::build_calltool_args(&schema_empty);
        assert!(result_empty.is_ok());
        let args = result_empty.unwrap().unwrap();
        assert!(args.is_empty());
    }
}
