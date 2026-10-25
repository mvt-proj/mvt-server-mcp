# mvt-server-mcp

An MCP server that bridges Claude to [mvt-rs](https://github.com/mvt-proj/mvt-rs), a PostgreSQL-backed vector tile and style management system. This server discovers catalog layers and style categories, and allows Claude to create or update MapLibre styles on the connected mvt-rs instance. Non-goals: this server makes no AI calls itself, performs no semantic style validation, and communicates via stdio only.

## Build

```bash
cargo build --release
```

The compiled binary is at `target/release/mvt-server-mcp`.

## Configure

The server loads configuration from a YAML file (default `config.yaml`), with environment variables overlaid on top. Configuration is required for three mvt-rs connection parameters:

- `mvt_rs.base_url`: HTTP(S) URL of the mvt-rs server instance (e.g., `http://localhost:5887`).
- `mvt_rs.email`: Admin email for authentication.
- `mvt_rs.password`: Admin password for authentication.

### Configuration File

Create a `config.yaml` at your working directory or specify one via the `MVT_SERVER_MCP_CONFIG_FILE` environment variable:

```yaml
mvt_rs:
  base_url: "http://localhost:5887"
  email: "admin@example.com"
  password: "change-me"
```

See `config.example.yaml` for a template.

### Environment Variable Overrides

Each configuration field can be overridden by an environment variable with the prefix `MVT_SERVER_MCP` and segment separator `__`:

- `MVT_SERVER_MCP__MVT_RS__BASE_URL` → `mvt_rs.base_url`
- `MVT_SERVER_MCP__MVT_RS__EMAIL` → `mvt_rs.email`
- `MVT_SERVER_MCP__MVT_RS__PASSWORD` → `mvt_rs.password`

Environment variables take precedence over the configuration file.

### Configuration File Escape Hatch

If you need to store the configuration file at a non-default location (important since MCP hosts control the launched process's working directory), set `MVT_SERVER_MCP_CONFIG_FILE` to its absolute path:

```bash
export MVT_SERVER_MCP_CONFIG_FILE=/etc/mvt-server-mcp/config.yaml
./mvt-server-mcp
```

## Register with Claude Desktop

Add the server to your `claude_desktop_config.json` (typically `~/.config/Claude/claude_desktop_config.json` on Linux/macOS or `%APPDATA%\Claude\claude_desktop_config.json` on Windows):

```json
{
  "mcpServers": {
    "mvt-server-mcp": {
      "command": "/absolute/path/to/mvt-server-mcp",
      "env": {
        "MVT_SERVER_MCP__MVT_RS__BASE_URL": "http://localhost:5887",
        "MVT_SERVER_MCP__MVT_RS__EMAIL": "admin@example.com",
        "MVT_SERVER_MCP__MVT_RS__PASSWORD": "change-me"
      }
    }
  }
}
```

Replace the path, URL, email, and password with your actual values. Restart Claude Desktop for the changes to take effect.

## Register with Claude Code

Register the server using the `claude mcp add` command:

```bash
claude mcp add mvt-server-mcp --env MVT_SERVER_MCP__MVT_RS__BASE_URL=http://localhost:5887 --env MVT_SERVER_MCP__MVT_RS__EMAIL=admin@example.com --env MVT_SERVER_MCP__MVT_RS__PASSWORD=change-me -- /absolute/path/to/mvt-server-mcp
```

Adjust the environment variables and path as needed for your setup.

## Tools

The server exposes eleven tools:

- **`list_layers`**: List every layer in the mvt-rs catalog, with its id, name, category, geometry type, published state, and the name+type of every field it exposes.

- **`get_layer`**: Get one catalog layer's complete raw configuration by id — every field mvt-rs stores (filter, srid, buffer, extent, zoom range, groups, etc.), not just the summary `list_layers` shows. Always call this before `update_layer`.

- **`create_layer`**: Register a new layer in the mvt-rs catalog from an existing Postgres table. The database/schema/table must already exist and be reachable from mvt-rs; this only creates the catalog entry.

- **`update_layer`**: Replace an existing layer's full catalog entry in mvt-rs by id — e.g. to move it to a different category, rename it, or change its published state. This replaces the whole entry, not a partial patch: call `get_layer` first and pass every field back with its current value except what you mean to change.

- **`list_categories`**: List every style category known to mvt-rs (id, name, description). The id is required by create_style/update_style's category_id parameter.

- **`create_category`**: Create a new style category in mvt-rs. The returned id is required by create_style/update_style's category_id parameter.

- **`update_category`**: Rename or redescribe an existing style category in mvt-rs by id. Use list_categories first to find the id.

- **`list_styles`**: List every existing MapLibre style in mvt-rs, with its id, name, category, and description (not the style body itself). Use this to find an existing style's id before calling update_style or get_style, or to see what already exists in a category before creating a new one.

- **`get_style`**: Get the full detail of one existing MapLibre style in mvt-rs by id, including its complete style_json body (parsed, not escaped). Use list_styles first to find the id.

- **`create_style`**: Create a new MapLibre style in mvt-rs. style_json must be a complete, valid MapLibre style JSON object; mvt-server-mcp does not validate it semantically — only mvt-rs's own JSON-parse check applies.

- **`update_style`**: Replace an existing MapLibre style in mvt-rs by id. This replaces the whole style, not a partial patch.

## Manual Smoke-Test Checklist

To verify the setup works end-to-end against a real mvt-rs instance:

1. **Start mvt-rs** at the configured URL (e.g., `http://localhost:5887`).
2. **Configure mvt-server-mcp** to point at that instance (via `config.yaml` or env vars).
3. **Register mvt-server-mcp** with Claude Desktop or Claude Code using the steps above.
4. **List layers**: Ask Claude to call `list_layers` and confirm that at least one layer is returned with id, name, category, fields, etc.
5. **List categories**: Ask Claude to call `list_categories` and confirm categories are listed with id, name, description.
6. **Create a style**: Ask Claude to create a trivial MapLibre style (e.g., a plain light background) and assign it to one of the returned categories; confirm it appears in mvt-rs's own admin UI.
7. **Verify rejection handling**: Ask Claude to reference a bogus category id (e.g., `nonexistent-category`) and confirm that the rejection message from mvt-rs surfaces back through the MCP host to the conversation.

## Security Note

The stored mvt-rs credential (email and password) grants full admin privilege; there is no scoped "styles only" role available today, so mvt-server-mcp must use a full admin account to do its job. This is a deliberate, accepted v1 trade-off, not an oversight. The password sits in plaintext in whatever configuration file or environment variable holds it — the same trust model as any local MCP server configuration. Ensure the configuration file and process environment are protected accordingly, and do not expose credentials in public repositories or logs.
