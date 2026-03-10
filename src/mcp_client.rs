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

use rmcp::model::Tool;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransport;
use rmcp::{RoleClient, ServiceExt};
use std::collections::HashMap;

/// A shared MCP client that encapsulates the connection and tool execution logic.
/// This can be used by both the REPL and future LLM clients.
pub struct McpClient {
    session: RunningService<RoleClient, ()>,
}

impl McpClient {
    /// Connects to the MCP server at the given URL
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let transport = StreamableHttpClientTransport::from_uri(url);
        let session = ().serve(transport).await?;
        Ok(Self { session })
    }

    /// Lists all available tools from the MCP server
    pub async fn list_tools(&self) -> anyhow::Result<Vec<Tool>> {
        let result = self.session.list_tools(None).await?;
        Ok(result.tools)
    }

    /// Calls a specific tool with the provided arguments
    pub async fn call_tool(
        &self,
        name: String,
        args: HashMap<String, serde_json::Value>,
    ) -> anyhow::Result<CallToolResult> {
        let request = CallToolRequestParams::new(name).with_arguments(args.into_iter().collect());
        let result = self.session.call_tool(request).await?;
        Ok(result)
    }

    /// Cleanly closes the MCP session
    pub async fn cleanup(self) -> anyhow::Result<()> {
        self.session.cancel().await?;
        Ok(())
    }
}
