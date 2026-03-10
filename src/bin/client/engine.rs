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

use crate::{AppContext, CallArgs};
use anyhow::{Result, anyhow, bail};
use rmcp::model::{CallToolResult, Tool};
use std::collections::HashMap;

pub async fn execute_list_tools(ctx: &AppContext<'_>) -> Result<Vec<Tool>> {
    let tools = ctx.client.list_tools().await?;
    Ok(tools)
}

pub async fn execute_call_tool(
    ctx: &AppContext<'_>,
    call_args: CallArgs,
) -> Result<CallToolResult> {
    let parsed_args: HashMap<String, serde_json::Value> = if let Some(file_path) = call_args.file {
        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Could not read file '{}': {}", file_path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| anyhow!("Invalid JSON in file '{}': {}", file_path, e))?
    } else {
        let args_trimmed = call_args.args.trim();
        if args_trimmed.is_empty() || args_trimmed == "{}" {
            HashMap::new()
        } else if args_trimmed.starts_with('{') {
            serde_json::from_str(args_trimmed)
                .map_err(|e| anyhow!("Invalid JSON arguments provided: {}", e))?
        } else {
            bail!("Invalid format. Use strict JSON '{{\"key\": \"val\"}}' or -f <PATH>");
        }
    };

    let result = ctx.client.call_tool(call_args.name, parsed_args).await?;
    Ok(result)
}
