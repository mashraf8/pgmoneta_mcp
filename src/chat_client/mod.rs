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

pub mod chat_hook;
pub mod commands;

use crate::chat_client::chat_hook::ChatHook;
use crate::configuration::{LlmConfiguration, PgmonetaMcpConfiguration};
use anyhow::anyhow;
use rig::agent::Agent as RigAgent;
use rig::client::CompletionClient;
use rig::client::Nothing;
use rig::completion::{Message, Prompt};
use rig::providers::{llamafile, ollama, openai};
use rig::tool::rmcp::McpClientHandler;
use rig::tool::server::ToolServer;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
use rmcp::service::{RoleClient, RunningService};

/// Default system prompt for the pgmoneta backup management assistant.
pub const SYSTEM_PROMPT: &str = "\
You are a PostgreSQL backup management assistant powered by pgmoneta. \
Help users with questions about backups, server status, and management operations. \
When presenting information, format it in a clear, human-readable way.";

/// Wraps different rig `Agent` types for runtime provider selection.
pub enum AnyAgent {
    Ollama(RigAgent<ollama::CompletionModel, ChatHook>),
    Llamafile(RigAgent<llamafile::CompletionModel, ChatHook>),
    OpenAiCompat(RigAgent<openai::completion::CompletionModel, ChatHook>),
}

/// Top-level agent managing conversation history and provider dispatch.
pub struct Agent {
    inner: AnyAgent,
    history: Vec<Message>,
    hook: ChatHook,
    _mcp_service: RunningService<RoleClient, McpClientHandler>,
}

impl Agent {
    /// Send a user prompt. Rig handles the multi-turn tool loop internally,
    /// and manages the history.
    pub async fn prompt(&mut self, input: &str) -> anyhow::Result<String> {
        // Reset hook state for new prompt
        self.hook.reset();

        let prompt_future = async {
            match &self.inner {
                AnyAgent::Ollama(a) => a.prompt(input).with_history(self.history.clone()).await,
                AnyAgent::Llamafile(a) => a.prompt(input).with_history(self.history.clone()).await,
                AnyAgent::OpenAiCompat(a) => {
                    a.prompt(input).with_history(self.history.clone()).await
                }
            }
        };

        // Run the agent prompt concurrently with a Ctrl+C listener
        let result: Result<String, anyhow::Error> = tokio::select! {
            res = prompt_future => res.map_err(|e| anyhow!("{e}")),
            _ = tokio::signal::ctrl_c() => {
                self.hook.clear_chain().await;
                Err(anyhow!("Cancelled"))
            }
        };

        match result {
            Ok(response) => {
                self.history.push(Message::user(input));
                self.history.push(Message::assistant(&response));
                self.hook.print_final_summary().await;
                Ok(response)
            }
            Err(e) => {
                self.hook.print_error_summary(&e.to_string()).await;
                Err(e)
            }
        }
    }

    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

/// Build an Agent from configuration.
pub async fn build_agent(
    config: &LlmConfiguration,
    mcp_config: &PgmonetaMcpConfiguration,
) -> anyhow::Result<Agent> {
    let hook = ChatHook::new(&config.provider, &config.model, &config.endpoint);

    let mcp_url = format!("http://127.0.0.1:{}/mcp", mcp_config.port);

    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
    );

    let tool_server_handle = ToolServer::new().run();
    let handler = McpClientHandler::new(client_info, tool_server_handle.clone());

    let transport = rmcp::transport::StreamableHttpClientTransport::from_uri(&*mcp_url);
    let mcp_service = handler
        .connect(transport)
        .await
        .map_err(|e| anyhow!("Failed to connect to MCP server: {:?}", e))?;

    // 2. Build the LLM provider
    let inner = match config.provider.as_str() {
        "ollama" => {
            let client = ollama::Client::builder()
                .api_key(Nothing)
                .base_url(&config.endpoint)
                .build()
                .map_err(|e| anyhow!("Failed to create Ollama client: {e}"))?;

            let mut builder = client
                .agent(&config.model)
                .preamble(SYSTEM_PROMPT)
                .hook(hook.clone())
                .tool_server_handle(tool_server_handle.clone())
                .default_max_turns(config.max_tool_rounds);

            if let Some(t) = config.temperature {
                builder = builder.temperature(t);
            }
            if let Some(m) = config.max_tokens {
                builder = builder.max_tokens(m);
            }

            AnyAgent::Ollama(builder.build())
        }

        "llama.cpp" => {
            let client = llamafile::Client::from_url(&config.endpoint);

            let mut builder = client
                .agent(&config.model)
                .preamble(SYSTEM_PROMPT)
                .hook(hook.clone())
                .tool_server_handle(tool_server_handle.clone())
                .default_max_turns(config.max_tool_rounds);

            if let Some(t) = config.temperature {
                builder = builder.temperature(t);
            }
            if let Some(m) = config.max_tokens {
                builder = builder.max_tokens(m);
            }

            AnyAgent::Llamafile(builder.build())
        }

        // Fallback for providers not supported by rig but that
        // expose an OpenAI-compatible API.
        // To add a new provider, append its name to the match pattern below.
        "ramalama" | "vllm" => {
            // openai::CompletionsClient appends "/chat/completions" to the base URL,
            // so we need to ensure the base URL includes the "/v1" prefix.
            let base_url = format!("{}/v1", config.endpoint.trim_end_matches('/'));

            let client = openai::CompletionsClient::builder()
                .api_key("no-key")
                .base_url(&base_url)
                .build()
                .map_err(|e| anyhow!("Failed to create OpenAI-compatible client: {e}"))?;

            let mut builder = client
                .agent(&config.model)
                .preamble(SYSTEM_PROMPT)
                .hook(hook.clone())
                .tool_server_handle(tool_server_handle.clone())
                .default_max_turns(config.max_tool_rounds);

            if let Some(t) = config.temperature {
                builder = builder.temperature(t);
            }
            if let Some(m) = config.max_tokens {
                builder = builder.max_tokens(m);
            }

            AnyAgent::OpenAiCompat(builder.build())
        }

        other => return Err(anyhow!("Unsupported LLM provider '{}'", other)),
    };

    Ok(Agent {
        inner,
        history: Vec::new(),
        hook,
        _mcp_service: mcp_service,
    })
}
