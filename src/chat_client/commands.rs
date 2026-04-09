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

use crate::chat_client::{self, Agent};
use crate::configuration::{LlmConfiguration, PgmonetaMcpConfiguration};
use console::style;

// ── Helper display functions ──────────────────────────────────────

fn print_section(title: &str) {
    eprintln!();
    eprintln!("  {}", style(title).bold().underlined());
    eprintln!();
}

fn print_help_row(cmd: &str, desc: &str) {
    eprintln!("    {}  {}", style(cmd).cyan().bold(), style(desc).dim());
}

fn print_config_row(key: &str, value: &str) {
    eprintln!("    {} {}", style(key).dim(), style(value).cyan().bold());
}

fn print_success(msg: &str) {
    eprintln!("  {} {}", style("✔").green(), msg);
}

fn print_error(msg: &str) {
    eprintln!("  {} {}", style("✖").red(), msg);
}

// ── Slash command handler ─────────────────────────────────────────

/// Process a slash command. Called only when input starts with '/'.
pub async fn handle(
    input: &str,
    agent: &mut Agent,
    config: &mut LlmConfiguration,
    mcp_config: &PgmonetaMcpConfiguration,
) {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0];
    let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match cmd {
        "/help" => {
            print_section("Available Commands");
            print_help_row("/clear", "Clear conversation history");
            print_help_row("/model <name>", "Switch the LLM model");
            print_help_row(
                "/provider <name>",
                "Switch provider (ollama/llama.cpp/ramalama/vllm)",
            );
            print_help_row("/endpoint <url>", "Change backend endpoint");
            print_help_row("/temperature <n>", "Set temperature (0.0 - 2.0)");
            print_help_row("/max-tokens <n>", "Set token limit");
            print_help_row("/config", "Show current configuration");
            print_help_row("/exit, /quit", "Exit the chat");
        }

        "/config" => {
            print_section("Current Configuration");
            print_config_row("Provider:", &config.provider);
            print_config_row("Model:", &config.model);
            print_config_row("Endpoint:", &config.endpoint);
            print_config_row(
                "Temperature:",
                &config
                    .temperature
                    .map_or("default".into(), |t| t.to_string()),
            );
            print_config_row(
                "Max tokens:",
                &config
                    .max_tokens
                    .map_or("default".into(), |t| t.to_string()),
            );
            print_config_row("Max rounds:", &config.max_tool_rounds.to_string());
        }

        "/clear" => {
            agent.clear_history();
            print_success(&format!("{}", style("History cleared").dim()));
        }

        "/model" => {
            if arg.is_empty() {
                print_config_row("Current model:", &config.model);
                eprintln!("  {} /model <name>", style("Usage:").dim());
            } else {
                rebuild_with(
                    agent,
                    config,
                    mcp_config,
                    |c| c.model = arg.to_string(),
                    &format!("Model changed to {}", style(arg).cyan().bold()),
                )
                .await;
            }
        }

        "/provider" => {
            if arg.is_empty() {
                print_config_row("Current provider:", &config.provider);
                eprintln!(
                    "  {} /provider <ollama|llama.cpp|ramalama|vllm>",
                    style("Usage:").dim()
                );
            } else {
                rebuild_with(
                    agent,
                    config,
                    mcp_config,
                    |c| {
                        c.provider = arg.to_string();
                        c.model = String::new();
                    },
                    &format!("Provider changed to {}", style(arg).cyan().bold()),
                )
                .await;
            }
        }

        "/endpoint" => {
            if arg.is_empty() {
                print_config_row("Current endpoint:", &config.endpoint);
                eprintln!("  {} /endpoint <url>", style("Usage:").dim());
            } else {
                rebuild_with(
                    agent,
                    config,
                    mcp_config,
                    |c| c.endpoint = arg.to_string(),
                    &format!("Endpoint changed to {}", style(arg).cyan()),
                )
                .await;
            }
        }

        "/temperature" => {
            if arg.is_empty() {
                print_config_row(
                    "Current temperature:",
                    &config
                        .temperature
                        .map_or("default".into(), |t| t.to_string()),
                );
                eprintln!("  {} /temperature <0.0-2.0>", style("Usage:").dim());
            } else {
                match arg.parse::<f64>() {
                    Ok(t) => {
                        rebuild_with(
                            agent,
                            config,
                            mcp_config,
                            |c| c.temperature = Some(t),
                            &format!("Temperature set to {}", style(t).bold()),
                        )
                        .await
                    }
                    Err(_) => print_error("Invalid temperature"),
                }
            }
        }

        "/max-tokens" => {
            if arg.is_empty() {
                print_config_row(
                    "Current max tokens:",
                    &config
                        .max_tokens
                        .map_or("default".into(), |t| t.to_string()),
                );
                eprintln!("  {} /max-tokens <num>", style("Usage:").dim());
            } else {
                match arg.parse::<u64>() {
                    Ok(m) => {
                        rebuild_with(
                            agent,
                            config,
                            mcp_config,
                            |c| c.max_tokens = Some(m),
                            &format!("Max tokens set to {}", style(m).bold()),
                        )
                        .await
                    }
                    Err(_) => print_error("Invalid token limit"),
                }
            }
        }

        _ => eprintln!(
            "  {} Unknown command: {}. Type {} for available commands.",
            style("✖").red(),
            style(cmd).yellow(),
            style("/help").cyan().bold()
        ),
    }

    eprintln!();
}

/// Apply a config mutation, rebuild the agent, and update both in place.
async fn rebuild_with(
    agent: &mut Agent,
    config: &mut LlmConfiguration,
    mcp_config: &PgmonetaMcpConfiguration,
    mutate: impl FnOnce(&mut LlmConfiguration),
    success_msg: &str,
) {
    let mut new_config = config.clone();
    mutate(&mut new_config);
    match chat_client::build_agent(&new_config, mcp_config).await {
        Ok(new_agent) => {
            *agent = new_agent;
            *config = new_config;
            print_success(success_msg);
        }
        Err(e) => print_error(&format!("Failed: {e}")),
    }
}
