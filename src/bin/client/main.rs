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
use cli::execute_cli_router;
use interactive::run_interactive_router;

#[derive(Debug, Parser)]
#[command(
    name = "pgmoneta-mcp-client",
    about = "Enterprise MCP client CLI for pgmoneta",
    version
)]
pub enum McpCli {
    /// Client operations (connect to MCP server)
    Client {
        #[command(flatten)]
        conn: ConnectionArgs,

        #[command(subcommand)]
        action: ClientCommands,
    },

    /// Launch the interactive wizard
    Interactive,
}

#[derive(Args, Debug, Clone)]
pub struct ConnectionArgs {
    /// URL of the MCP server
    #[arg(short = 'u', long)]
    pub url: String,

    /// Connection timeout in seconds
    #[arg(short = 't', long, default_value_t = 30)]
    pub timeout: u64,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ClientCommands {
    /// Manage and execute tools provided by the MCP Server
    Tool {
        #[command(subcommand)]
        action: ToolCommands,
    },
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

#[derive(Args, Debug, Clone)]
pub struct PrintArgs {
    /// Output format for responses
    #[arg(short = 'o', long, value_enum, default_value_t = OutputFormat::Tree)]
    pub output: OutputFormat,
}

#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
pub enum OutputFormat {
    /// Print response as an ASCII tree
    Tree,
    /// Print response as raw JSON
    Json,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = McpCli::parse();

    match args {
        McpCli::Interactive => {
            run_interactive_router().await?;
        }
        client_cmd @ McpCli::Client { .. } => {
            execute_cli_router(client_cmd).await?;
        }
    }

    Ok(())
}
