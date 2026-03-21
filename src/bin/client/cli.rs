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

use crate::engine::ClientEngine;
use crate::{ClientCommands, McpCli, OutputFormat, ToolCommands};
use anyhow::{Result, bail};
use serde::Serialize;
use treelog::{Tree, config::RenderConfig, renderer::write_tree_with_config};

pub async fn execute_cli_router(cmd: McpCli) -> Result<()> {
    match cmd {
        McpCli::Client { conn, action } => {
            let router = ClientCli::new(conn.url, conn.timeout, action);
            router.execute().await?;
        }
        McpCli::Interactive => {
            // Interactive mode is handled entirely in main.rs routing,
            // so we shouldn't hit this, but it matches exhaustively.
        }
    }
    Ok(())
}

pub trait CliExecute {
    #[allow(async_fn_in_trait)]
    async fn execute(&self) -> Result<()>;
}

pub struct ClientCli {
    pub url: String,
    pub timeout_secs: u64,
    pub cmd: ClientCommands,
}

impl ClientCli {
    pub fn new(url: String, timeout_secs: u64, cmd: ClientCommands) -> Self {
        Self {
            url,
            timeout_secs,
            cmd,
        }
    }
    fn format_response<T: Serialize>(data: &T, format: &OutputFormat) -> Result<String> {
        match serde_json::to_string_pretty(data) {
            Ok(json_str) => match format {
                OutputFormat::Json => Ok(json_str),
                OutputFormat::Tree => match Tree::from_arbitrary_json(&json_str) {
                    Ok(tree) => {
                        let mut output = String::new();
                        let config = RenderConfig::default().with_colors(true);
                        if write_tree_with_config(&mut output, &tree, &config).is_ok() {
                            Ok(output)
                        } else {
                            bail!("Failed to format tree output")
                        }
                    }
                    Err(e) => bail!("Error parsing arbitrary JSON into tree: {:?}", e),
                },
            },
            Err(e) => bail!("Error serializing data: {:?}", e),
        }
    }
}

impl CliExecute for ClientCli {
    async fn execute(&self) -> Result<()> {
        let client = ClientEngine::connect(&self.url, self.timeout_secs).await?;

        match &self.cmd {
            ClientCommands::Tool { action } => match action {
                ToolCommands::List { print_opts } => {
                    let tools = client.execute_list_tools().await?;
                    let output = Self::format_response(&tools, &print_opts.output)?;
                    println!("{}", output);
                }
                ToolCommands::Call {
                    call_args,
                    print_opts,
                } => {
                    let result = client.execute_call_tool(call_args.clone()).await?;
                    let output = Self::format_response(&result, &print_opts.output)?;
                    println!("{}", output);
                }
            },
        }

        client.cleanup().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_response_formats() {
        let data = json!({"name": "test_tool", "status": "ok"});

        // Test JSON format
        let result_json = ClientCli::format_response(&data, &OutputFormat::Json);
        assert!(result_json.is_ok());

        // Test Tree format
        let result_tree = ClientCli::format_response(&data, &OutputFormat::Tree);
        assert!(result_tree.is_ok());
    }

    #[test]
    fn test_format_response_data_types() {
        // Nested JSON
        let nested_data = json!({
            "tool": {
                "name": "get_backup_info",
                "params": {
                    "server": "primary",
                    "backup": "latest"
                }
            },
            "status": "success"
        });
        assert!(ClientCli::format_response(&nested_data, &OutputFormat::Json).is_ok());
        assert!(ClientCli::format_response(&nested_data, &OutputFormat::Tree).is_ok());

        // Empty array
        let empty_data: Vec<String> = vec![];
        assert!(ClientCli::format_response(&empty_data, &OutputFormat::Json).is_ok());
        assert!(ClientCli::format_response(&empty_data, &OutputFormat::Tree).is_ok());

        // Large payload
        let large_data = json!({
            "field1": "value1",
            "field2": "value2",
            "field3": "value3",
            "field4": 12345,
            "field5": true,
            "field6": null,
            "field7": [1, 2, 3],
            "field8": {"nested": "object"}
        });
        assert!(ClientCli::format_response(&large_data, &OutputFormat::Json).is_ok());
        assert!(ClientCli::format_response(&large_data, &OutputFormat::Tree).is_ok());
    }
}
