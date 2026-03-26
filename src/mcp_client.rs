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

use crate::configuration;
use rmcp::model::Tool;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransport;
use rmcp::{RoleClient, ServiceExt};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

/// The main client interface for communicating with the pgmoneta MCP server.
pub struct McpClient {
    session: RunningService<RoleClient, ()>,
    timeout: Duration,
    url: String,
}

impl McpClient {
    /// Connects to the MCP server by parsing the configuration file at `conf_path`
    pub async fn connect(conf_path: &str) -> anyhow::Result<Self> {
        let config = configuration::load_client_configuration(conf_path).map_err(|e| {
            anyhow::anyhow!("Failed to load client config from '{}': {}", conf_path, e)
        })?;
        Self::connect_raw(&config.url, config.timeout).await
    }

    /// Lists all available tools from the MCP server
    pub async fn list_tools(&self) -> anyhow::Result<Vec<Tool>> {
        let result = timeout(self.timeout, self.session.list_tools(None))
            .await
            .map_err(|_| anyhow::anyhow!("list_tools timed out"))??;
        Ok(result.tools)
    }

    /// Calls a specific tool with the provided arguments
    pub async fn call_tool(
        &self,
        name: String,
        args: HashMap<String, serde_json::Value>,
    ) -> anyhow::Result<CallToolResult> {
        let request = CallToolRequestParams::new(name).with_arguments(args.into_iter().collect());
        let result = timeout(self.timeout, self.session.call_tool(request))
            .await
            .map_err(|_| anyhow::anyhow!("call_tool timed out"))??;
        Ok(result)
    }

    /// Returns the server's name, version, and the connected URL.
    pub fn server_info(&self) -> Option<(&str, &str, &str)> {
        self.session.peer_info().map(|info| {
            (
                info.server_info.name.as_str(),
                info.server_info.version.as_str(),
                self.url.as_str(),
            )
        })
    }

    /// Cleanly closes the MCP session
    pub async fn cleanup(self) -> anyhow::Result<()> {
        self.session.cancel().await?;
        Ok(())
    }

    /// Connects to the MCP server at the given URL with a specified timeout
    async fn connect_raw(url: &str, timeout_secs: u64) -> anyhow::Result<Self> {
        let timeout_duration = Duration::from_secs(timeout_secs);
        let transport = StreamableHttpClientTransport::from_uri(url);
        let session = timeout(timeout_duration, ().serve(transport))
            .await
            .map_err(|_| {
                anyhow::anyhow!("Connection timed out after {} seconds", timeout_secs)
            })??;
        Ok(Self {
            session,
            timeout: timeout_duration,
            url: url.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect_scenarios() {
        // Test invalid URL
        let result = McpClient::connect_raw("not-a-url", 1).await;
        assert!(result.is_err());

        // Test timeout zero
        let result = McpClient::connect_raw("http://192.0.2.1:1234", 0).await;
        match result {
            Err(e) => assert_eq!(e.to_string(), "Connection timed out after 0 seconds"),
            Ok(_) => panic!("Expected an error, but got Ok"),
        }

        // Test empty URL
        let result = McpClient::connect_raw("", 1).await;
        assert!(result.is_err());
    }
}
