## Chat Client

The chat client provides an interactive conversational interface to manage PostgreSQL backups using natural language.

### Configuration

The chat client requires both `[pgmoneta_mcp]` and `[llm]` sections in your `pgmoneta-mcp.conf`.

#### pgmoneta_mcp

| Parameter | Required | Default | Description |
| :--- | :--- | :--- | :--- |
| port | Yes | - | MCP server port |

#### llm

| Parameter | Required | Default | Description |
| :--- | :--- | :--- | :--- |
| provider | Yes | - | LLM backend: `ollama`, `llama.cpp`, `ramalama`, `vllm` |
| endpoint | Yes | - | LLM server URL |
| model | No | Provider default | Model name |
| max_tool_rounds | No | 10 | Max tool-calling rounds |
| temperature | No | - | Sampling temperature (0.0-2.0) |
| max_tokens | No | - | Token limit |

Example:

``` ini
[pgmoneta_mcp]
port = 8000

[pgmoneta]

[admins]

[llm]
provider = ollama
endpoint = http://localhost:11434
model = llama3.1
```

### Usage

``` sh
./pgmoneta-mcp-client -c /etc/pgmoneta-mcp/pgmoneta-mcp.conf
```

### Commands

| Command | Description |
| :--- | :--- |
| /help | Show available commands |
| /clear | Clear conversation history |
| /model \<name\> | Switch LLM model |
| /provider \<name\> | Switch provider |
| /endpoint \<url\> | Change server endpoint |
| /temperature \<n\> | Set temperature |
| /max-tokens \<n\> | Set token limit |
| /config | Show current configuration |
| /exit | Exit chat |
| /quit | Exit chat |

### Keyboard Shortcuts

- **Ctrl+C** - Cancel current request
