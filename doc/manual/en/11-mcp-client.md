# pgmoneta-mcp-client

## CLI Hierarchy Tree
The following tree visualizes the entire command-line structure.

```text
pgmoneta-mcp-client
│
├── client (Connect to MCP server)
│   ├── --url -u <URL>         (Required: MCP server address, e.g., http://localhost:8000/mcp)
│   ├── --timeout -t <SECONDS> (Optional: Maximum total timeout for requests. Default: 30s)
│   │
│   └── tool (Manage and execute MCP tools)
│       ├── list (List all available tools on the server)
│       │   └── --output -o <tree|json> (Default: tree)
│       │
│       └── call (Call a specific tool)
│           ├── <NAME>              (Position 1: Tool name, e.g., get_backup_info)
│           ├── <ARGS>              (Position 2: Strict JSON arguments. Default: "{}")
│           ├── --file -f <PATH>    (Optional: Path file containing JSON arguments)
│           └── --output -o <tree|json> (Default: tree)
│
└── interactive (Launch the interactive wizard/shell)
```

---

## Commands
 Execute commands directly.
<!--
  HOW TO ADD A NEW COMMAND:
  Each command represents a traditional CLI subcommand — a direct, explicit way to perform
  a specific action by typing the full command with its flags and arguments in the terminal.
  Use this approach when scripting, automating, or when you already know the exact syntax.

  ## <N>. <command-name>
  ### A. <Feature or Subcommand>
  ...
-->

### A. Client Example

#### 1. Listing Tools

```bash
./pgmoneta-mcp-client client --url <your_mcp_server_url> tool list
```

#### 2. Calling a Tool

```bash
./pgmoneta-mcp-client client --url <your_mcp_server_url> tool call <tool_name_with_args> '{"key": "value"}'
```
```bash
./pgmoneta-mcp-client client --url <your_mcp_server_url> tool call <tool_name_without_args>
```

> **Note 1:** The `-f` flag allows you to load data from any file. This is functionally identical to typing directly in the terminal.Max file size supported: **10 MB**
> ```bash
> ./pgmoneta-mcp-client client --url <your_mcp_server_url> tool call get_backup_info -f <path_to_args_file>
> ```

---

## Interactive

Built on the command line, that provides a smooth user experience and converting user input into commands and executing them. run it: 

```bash
./pgmoneta-mcp-client interactive
```

> **Note 1 (Strict JSON Inputs):** Because the wizard may assembles a JSON request from your inputs (like the `args` of the `call` tool), every value must be a valid JSON value:
>
> | Type | Rule | Example |
> | :--- | :--- | :--- |
> | String | Must be wrapped in double quotes | `"primary"` |
> | Number | Enter directly | `123` |
> | Boolean | Enter directly | `true` or `false` |
> | Object | Enter full JSON object | `{"key": "value"}` |
> | Array | Enter full JSON array | `["a", "b", "c"]` |
> | Null | Enter null keyword | `null` |
> | Empty (skip) | Leave blank — the key will not be sent | *(press Enter)* |

> **Note 2 (`@path` Injection):** In any argument prompt, type @ followed by a file path to inject the file's content as the value. Max file size supported: **10 MB**.
> ```
> @anypath/file.txt
> ```