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

use anyhow::{Result, bail};
use serde::Serialize;
use treelog::{Tree, config::RenderConfig, renderer::write_tree_with_config};

use crate::engine::{execute_call_tool, execute_list_tools};
use crate::{AppContext, Commands, OutputFormat, ToolCommands};

pub async fn execute_cli_router(ctx: &AppContext<'_>, cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Tool { action } => match action {
            ToolCommands::List { print_opts } => {
                let tools = execute_list_tools(ctx).await?;
                print_response(&tools, &print_opts.output)?;
            }
            ToolCommands::Call {
                call_args,
                print_opts,
            } => {
                let result = execute_call_tool(ctx, call_args).await?;
                print_response(&result, &print_opts.output)?;
            }
        },
        Commands::Interactive => { /* Handled previously */ }
    }
    Ok(())
}

fn print_response<T: Serialize>(data: &T, format: &OutputFormat) -> Result<()> {
    match serde_json::to_string_pretty(data) {
        Ok(json_str) => match format {
            OutputFormat::Json => {
                println!("{}", json_str);
            }
            OutputFormat::Tree => match Tree::from_arbitrary_json(&json_str) {
                Ok(tree) => {
                    let mut output = String::new();
                    let config = RenderConfig::default().with_colors(true);
                    if write_tree_with_config(&mut output, &tree, &config).is_ok() {
                        println!("{}", output);
                    }
                }
                Err(e) => bail!("Error parsing arbitrary JSON into tree: {:?}", e),
            },
        },
        Err(e) => bail!("Error serializing data: {:?}", e),
    }
    Ok(())
}
