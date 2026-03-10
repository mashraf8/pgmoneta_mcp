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

use anyhow::{Result, anyhow, bail};
use inquire::{Select, Text};
use std::collections::HashMap;

use crate::cli::execute_cli_router;
use crate::engine::execute_list_tools;
use crate::{AppContext, CallArgs, Commands, OutputFormat, PrintArgs, ToolCommands};

/// Gracefully handles Inquire cancellations such as Esc or Ctrl+C
/// by returning `Ok(None)` from the current function.
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

pub async fn run_interactive_wizard(ctx: &AppContext<'_>) -> Result<()> {
    println!("Welcome to pgmoneta MCP Interactive Shell!");

    let options = vec!["Tools", "Exit"];
    loop {
        let choice = Select::new("What would you like to manage?", options.clone()).prompt();

        match choice {
            Ok("Tools") => match interactive_tool_router(ctx).await {
                Ok(Some(cmd)) => {
                    if let Err(e) = execute_cli_router(ctx, cmd).await {
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
    Ok(())
}

async fn interactive_tool_router(ctx: &AppContext<'_>) -> Result<Option<Commands>> {
    let options = vec!["List Tools", "Call Tool"];
    let choice = prompt_or_cancel!(
        Select::new("What would you like to do with tools?", options).prompt(),
        "Selection failed"
    );

    match choice {
        "List Tools" => Ok(Some(Commands::Tool {
            action: ToolCommands::List {
                print_opts: PrintArgs {
                    output: OutputFormat::Tree,
                },
            },
        })),
        "Call Tool" => {
            let tools = execute_list_tools(ctx).await?;
            let tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();

            if tool_names.is_empty() {
                bail!("Notice: No tools are currently available on the server.");
            }

            let name = prompt_or_cancel!(
                Select::new("Select a tool to call:", tool_names).prompt(),
                "Tool selection failed"
            );

            let tool = tools.iter().find(|t| t.name == name).unwrap();

            match build_calltool_args(&tool.input_schema)? {
                Some(args) => Ok(Some(Commands::Tool {
                    action: ToolCommands::Call {
                        call_args: CallArgs {
                            name: name.to_string(),
                            file: None,
                            args: serde_json::to_string(&args).unwrap_or_default(),
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

fn build_calltool_args(
    schema: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<HashMap<String, serde_json::Value>>> {
    let mut args = HashMap::new();
    let props = schema
        .get("properties")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("Tool schema missing properties"))?;

    for (prop_name, prop_schema) in props {
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

        let v = prompt_or_cancel!(Text::new(&prompt).prompt(), "Input failed");

        if !v.trim().is_empty() {
            let content = if let Some(path) = v.trim().strip_prefix('@') {
                std::fs::read_to_string(path)
                    .map_err(|e| anyhow!("Failed to read file '{}': {}", path, e))?
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
