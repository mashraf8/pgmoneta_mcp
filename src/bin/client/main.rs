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

use anyhow::{Result, anyhow};
use clap::Parser;
use console::style;
use dialoguer::Input;
use pgmoneta_mcp::chat_client::{self, commands};
use pgmoneta_mcp::configuration;

/// FIGlet ASCII art banner for pgmoneta (ANSI Shadow font from patorjk.com)
const LOGO: &str = r#"
██████╗  ██████╗ ███╗   ███╗ ██████╗ ███╗   ██╗███████╗████████╗ █████╗
██╔══██╗██╔════╝ ████╗ ████║██╔═══██╗████╗  ██║██╔════╝╚══██╔══╝██╔══██╗
██████╔╝██║  ███╗██╔████╔██║██║   ██║██╔██╗ ██║█████╗     ██║   ███████║
██╔═══╝ ██║   ██║██║╚██╔╝██║██║   ██║██║╚██╗██║██╔══╝     ██║   ██╔══██║
██║     ╚██████╔╝██║ ╚═╝ ██║╚██████╔╝██║ ╚████║███████╗   ██║   ██║  ██║
╚═╝      ╚═════╝ ╚═╝     ╚═╝ ╚═════╝ ╚═╝  ╚═══╝╚══════╝   ╚═╝   ╚═╝  ╚═╝
"#;

#[derive(Parser, Debug)]
#[command(
    name = "pgmoneta-mcp-client",
    about = "Interactive LLM chat client for pgmoneta-mcp"
)]
struct Args {
    /// Path to main configuration file
    #[arg(
        short = 'c',
        long,
        default_value = "/etc/pgmoneta-mcp/pgmoneta-mcp.conf"
    )]
    config: String,
}

fn print_banner(provider: &str, model: &str, endpoint: &str) {
    eprintln!();
    // Print FIGlet logo in orange (closest ANSI: yellow/208)
    for line in LOGO.lines() {
        if !line.is_empty() {
            eprintln!("{}", style(line).color256(208).bold());
        }
    }

    // Info line
    eprintln!(
        "  {} {} · {} {} · {} {}",
        style("Model:").dim(),
        style(model).cyan().bold(),
        style("Provider:").dim(),
        style(provider).cyan().bold(),
        style("Endpoint:").dim(),
        style(endpoint).cyan(),
    );
    eprintln!();
    eprintln!(
        "  {} · {} · {}",
        style("Ctrl+C cancel").dim(),
        style("/help commands").dim(),
        style("/quit exit").dim(),
    );
    eprintln!();
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load configurations
    let config = configuration::load_base_configuration(&args.config)?;
    let mcp_config = config.pgmoneta_mcp.clone();
    let mut llm_config = config
        .llm
        .clone()
        .ok_or_else(|| anyhow!("No [llm] section in configuration file"))?;

    // Build agent with MCP Tools
    let mut chat_agent = chat_client::build_agent(&llm_config, &mcp_config).await?;

    // Print banner
    print_banner(
        &llm_config.provider,
        &llm_config.model,
        &llm_config.endpoint,
    );

    loop {
        // Styled prompt: you@provider>
        let prompt_text = format!(
            "{}@{}",
            style("you").green().bold(),
            style(&llm_config.provider).cyan()
        );

        let input: String = match Input::new()
            .with_prompt(&prompt_text)
            .allow_empty(true)
            .interact_text()
        {
            Ok(input) => input,
            Err(_) => {
                eprintln!("\n{}", style("Goodbye! 👋").dim());
                break;
            }
        };

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "/exit" || input == "/quit" {
            eprintln!("{}", style("Goodbye! 👋").dim());
            break;
        }

        // Slash command → delegate entirely to commands module
        if input.starts_with('/') {
            commands::handle(input, &mut chat_agent, &mut llm_config, &mcp_config).await;
            continue;
        }

        // Send to agent and render the response with markdown
        match chat_agent.prompt(input).await {
            Ok(output) => {
                eprintln!();
                // Render LLM response as terminal markdown
                termimad::print_text(&output);
                eprintln!();
            }
            Err(e) => {
                // If the error was just "Cancelled", don't spam stack traces
                if e.to_string() != "Cancelled" {
                    eprintln!("\n  {} {}\n", style("Error:").red().bold(), style(e).red());
                }
            }
        }
    }

    Ok(())
}
