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

pub mod cli;
pub mod engine;
pub mod interactive;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use pgmoneta_mcp::mcp_client::McpClient;

use cli::execute_cli_router;
use interactive::run_interactive_wizard;

#[derive(Debug, Parser)]
#[command(
    name = "pgmoneta-mcp-client",
    about = "Enterprise MCP client CLI for pgmoneta",
    version
)]
pub struct McpCli {
    /// URL of the MCP server
    #[arg(short = 'u', long)]
    pub url: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Manage and execute tools provided by the MCP Server
    Tool {
        #[command(subcommand)]
        action: ToolCommands,
    },

    /// Launch the interactive wizard (Fallback)
    Interactive,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ToolCommands {
    /// List all available tools on the MCP server
    List {
        #[command(flatten)]
        print_opts: PrintArgs,
    },

    /// Call a specific tool on the MCP server
    Call {
        #[command(flatten)]
        call_args: CallArgs,

        #[command(flatten)]
        print_opts: PrintArgs,
    },
}

#[derive(Args, Debug, Clone)]
pub struct PrintArgs {
    /// Output format for responses
    #[arg(short = 'o', long, value_enum, default_value_t = OutputFormat::Tree)]
    pub output: OutputFormat,
}

#[derive(Args, Debug, Clone)]
pub struct CallArgs {
    /// Name of the tool to call
    pub name: String,

    /// Optional path to a JSON file containing the arguments
    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,

    /// JSON arguments for the tool (Strict JSON format)
    #[arg(default_value = "{}")]
    pub args: String,
}

#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
pub enum OutputFormat {
    /// Print response as an ASCII tree
    Tree,
    /// Print response as raw JSON
    Json,
}

/// The AppContext carries connection state and global configuration
pub struct AppContext<'a> {
    pub client: &'a McpClient,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = McpCli::parse();

    let client = McpClient::connect(&args.url).await?;

    let ctx = AppContext { client: &client };

    match args.command {
        Some(Commands::Interactive) | None => {
            run_interactive_wizard(&ctx).await?;
        }
        Some(cmd) => {
            execute_cli_router(&ctx, cmd).await?;
        }
    }

    client.cleanup().await?;
    Ok(())
}
