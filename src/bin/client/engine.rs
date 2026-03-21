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

use std::collections::HashMap;
use anyhow::{Result, anyhow, bail};
use rmcp::model::{CallToolResult, Tool};
use crate::CallArgs;
use pgmoneta_mcp::mcp_client::McpClient;
use pgmoneta_mcp::utils::SafeFileReader;

pub struct ClientEngine {
    client: McpClient,
}

impl ClientEngine {
    pub async fn connect(url: &str, timeout_secs: u64) -> Result<Self> {
        let client = McpClient::connect(url, timeout_secs).await?;
        Ok(Self { client })
    }

    pub async fn cleanup(self) -> Result<()> {
        self.client.cleanup().await?;
        Ok(())
    }

    pub async fn execute_list_tools(&self) -> Result<Vec<Tool>> {
        let tools = self.client.list_tools().await?;
        Ok(tools)
    }

    pub fn server_info(&self) -> Option<(&str, &str)> {
        self.client.server_info()
    }

    pub async fn execute_call_tool(&self, call_args: CallArgs) -> Result<CallToolResult> {
        let parsed_args = Self::parse_call_args(&call_args)?;
        let result = self.client.call_tool(call_args.name, parsed_args).await?;
        Ok(result)
    }

    fn parse_call_args(call_args: &CallArgs) -> Result<HashMap<String, serde_json::Value>> {
        if let Some(file_path) = &call_args.file {
            let content = SafeFileReader::new()
                .max_size(10 * 1024 * 1024)
                .read(file_path)?;
            serde_json::from_str(&content)
                .map_err(|e| anyhow!("Invalid JSON in file '{}': {}", file_path, e))
        } else {
            let args_trimmed = call_args.args.trim();
            if args_trimmed.is_empty() || args_trimmed == "{}" {
                Ok(HashMap::new())
            } else if args_trimmed.starts_with('{') {
                serde_json::from_str(args_trimmed)
                    .map_err(|e| anyhow!("Invalid JSON arguments provided: {}", e))
            } else {
                bail!("Invalid format. Use strict JSON '{{\"key\": \"val\"}}' or -f <PATH>");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_call_args(name: &str, args: &str, file: Option<&str>) -> CallArgs {
        CallArgs {
            name: name.to_string(),
            args: args.to_string(),
            file: file.map(|f| f.to_string()),
        }
    }

    #[test]
    fn test_parse_empty_or_whitespace() {
        // Test empty braces
        let call_args = make_call_args("tool", "{}", None);
        let result = ClientEngine::parse_call_args(&call_args).unwrap();
        assert!(result.is_empty());

        // Test empty string
        let call_args = make_call_args("tool", "", None);
        let result = ClientEngine::parse_call_args(&call_args).unwrap();
        assert!(result.is_empty());

        // Test whitespace only
        let call_args = make_call_args("tool", "   ", None);
        let result = ClientEngine::parse_call_args(&call_args).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_valid_json() {
        // Test single key
        let call_args = make_call_args("tool", r#"{"server":"s1"}"#, None);
        let result = ClientEngine::parse_call_args(&call_args).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("server").unwrap(), "s1");

        // Test multiple keys
        let call_args = make_call_args("tool", r#"{"server":"s1","backup":"b1"}"#, None);
        let result = ClientEngine::parse_call_args(&call_args).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result.get("server").unwrap(), "s1");
        assert_eq!(result.get("backup").unwrap(), "b1");
    }

    #[test]
    fn test_parse_invalid_json() {
        // Test not json
        let call_args = make_call_args("tool", "not json", None);
        let result = ClientEngine::parse_call_args(&call_args);
        assert!(result.is_err());

        // Test missing brace
        let call_args = make_call_args("tool", r#"{"key": "val""#, None);
        let result = ClientEngine::parse_call_args(&call_args);
        assert!(result.is_err());

        // Test invalid inside JSON
        let call_args = make_call_args("tool", r#"{"key: unquoted_value}"#, None);
        let result = ClientEngine::parse_call_args(&call_args);
        assert!(result.is_err());
    }
}
