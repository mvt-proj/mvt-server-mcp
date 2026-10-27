# mvt-server-mcp

An MCP server that bridges AI assistants — Claude, Antigravity, or any other MCP-compatible client — to [MVT Server](https://github.com/mvt-proj/mvt-rs) (repo name `mvt-rs`), a PostgreSQL-backed vector tile and style management system. This server discovers catalog layers and style categories, and allows the connected AI assistant to create or update MapLibre styles on the connected MVT Server instance. Non-goals: this server makes no AI calls itself, performs no semantic style validation, and communicates via stdio only.

## Build

```bash
cargo build --release
```

The compiled binary is at `target/release/mvt-server-mcp`.

## Configure

The server loads configuration from a YAML file (default `config.yaml`), with environment variables overlaid on top. Configuration is required for three MVT Server connection parameters:

- `mvt_rs.base_url`: HTTP(S) URL of the MVT Server instance (e.g., `http://localhost:5887`).
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

## Register with an MCP Client

This server speaks standard MCP over stdio, so it works with any MCP-compatible client — Claude Desktop, Claude Code, Antigravity CLI, and others. The sections below cover the clients with their own registration flow; for other MCP hosts, consult their docs for how to add a stdio server with a command/args/env configuration.

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

Register the server using the `claude mcp add` command. Either pass the three connection fields as separate env vars:

```bash
claude mcp add mvt-server-mcp --env MVT_SERVER_MCP__MVT_RS__BASE_URL=http://localhost:5887 --env MVT_SERVER_MCP__MVT_RS__EMAIL=admin@example.com --env MVT_SERVER_MCP__MVT_RS__PASSWORD=change-me -- /absolute/path/to/mvt-server-mcp
```

or point at an existing `config.yaml` with the escape-hatch variable instead:

```bash
claude mcp add mvt-server-mcp --env MVT_SERVER_MCP_CONFIG_FILE=/absolute/path/to/config.yaml -- /absolute/path/to/mvt-server-mcp
```

Adjust the environment variables and path as needed for your setup. This writes to local scope (this project, this machine only) by default; add `--scope project` to share the entry via a checked-in `.mcp.json`, or `--scope user` to make it available in every project.

## Register with Antigravity CLI

Register the server using the `agy mcp add` command. Note the argument order differs from `claude mcp add`: name and binary path come after the flags, with no `--` separator.

```bash
agy mcp add \
  --env MVT_SERVER_MCP_CONFIG_FILE=/absolute/path/to/config.yaml \
  mvt-server-mcp \
  /absolute/path/to/mvt-server-mcp
```

As with Claude Code, you can instead pass the three `MVT_SERVER_MCP__MVT_RS__*` env vars directly if you don't want a config file on disk.

## Tools

The server exposes eighteen tools:

- **`list_layers`**: List every layer in the MVT Server catalog, with its id, name, category, geometry type, published state, and the name+type of every field it exposes.

- **`get_layer`**: Get one catalog layer's complete raw configuration by id — every field MVT Server stores (filter, srid, buffer, extent, zoom range, groups, etc.), not just the summary `list_layers` shows. Always call this before `update_layer`.

- **`create_layer`**: Register a new layer in the MVT Server catalog from an existing Postgres table. The database/schema/table must already exist and be reachable from MVT Server; this only creates the catalog entry.

- **`update_layer`**: Replace an existing layer's full catalog entry in MVT Server by id — e.g. to move it to a different category, rename it, or change its published state. This replaces the whole entry, not a partial patch: call `get_layer` first and pass every field back with its current value except what you mean to change.

- **`list_schemas`**: List every schema in a database reachable from MVT Server. Use this before `list_tables` to find the schema to look in.

- **`list_tables`**: List every table, view, and materialized view in a database schema reachable from MVT Server, with the name of its geometry column. Only relations with a registered geometry column are returned; the three kinds come back combined, with nothing distinguishing which is which. Use `list_schemas` first to find the schema.

- **`list_table_fields`**: List every column (name and Postgres type) of a database table reachable from MVT Server, whether or not it is already a catalog layer. Use it before `create_layer` to pick the table's real field names; leave the geometry column out of `create_layer`'s fields, since it goes in `geom`.

- **`clear_layer_cache`**: Clear the cached tiles for one catalog layer in MVT Server by id, so the next request for any of its tiles is rendered fresh from the database. Use `list_layers` first to find the id.

- **`list_groups`**: List every user group known to MVT Server (id, name, description). `create_layer`/`update_layer`'s `groups` parameter takes group names, matched exactly; MVT Server silently drops an unknown name, and a layer left with no groups is public. Check names here first, then confirm with `get_layer`.

- **`create_group`**: Create a new user group in MVT Server. MVT Server normalizes the name (lowercased, accents stripped, spaces to underscores), so use the returned name when assigning the group to a layer.

- **`update_group`**: Rename or redescribe an existing user group in MVT Server by id. Use `list_groups` first to find the id. Layers reference ordinary groups by id, so renaming keeps their access. `admin` and `admin_metadata` are roles MVT Server checks by exact name, so renaming them, or renaming another group to those names, is refused; their description can still change.

- **`list_categories`**: List every style category known to MVT Server (id, name, description). The id is required by create_style/update_style's category_id parameter.

- **`create_category`**: Create a new style category in MVT Server. The returned id is required by create_style/update_style's category_id parameter.

- **`update_category`**: Rename or redescribe an existing style category in MVT Server by id. Use list_categories first to find the id.

- **`list_styles`**: List every existing MapLibre style in MVT Server, with its id, name, category, and description (not the style body itself). Use this to find an existing style's id before calling update_style or get_style, or to see what already exists in a category before creating a new one.

- **`get_style`**: Get the full detail of one existing MapLibre style in MVT Server by id, including its complete style_json body (parsed, not escaped). Use list_styles first to find the id.

- **`create_style`**: Create a new MapLibre style in MVT Server. style_json must be a complete, valid MapLibre style JSON object; mvt-server-mcp does not validate it semantically — only MVT Server's own JSON-parse check applies.

- **`update_style`**: Replace an existing MapLibre style in MVT Server by id. This replaces the whole style, not a partial patch.

## Manual Smoke-Test Checklist

To verify the setup works end-to-end against a real MVT Server instance:

1. **Start MVT Server** at the configured URL (e.g., `http://localhost:5887`).
2. **Configure mvt-server-mcp** to point at that instance (via `config.yaml` or env vars).
3. **Register mvt-server-mcp** with your MCP client (Claude Desktop, Claude Code, Antigravity CLI, or another MCP host) using the steps above.
4. **List layers**: Ask the AI assistant to call `list_layers` and confirm that at least one layer is returned with id, name, category, fields, etc.
5. **List categories**: Ask the AI assistant to call `list_categories` and confirm categories are listed with id, name, description.
6. **Create a style**: Ask the AI assistant to create a trivial MapLibre style (e.g., a plain light background) and assign it to one of the returned categories; confirm it appears in MVT Server's own admin UI.
7. **Verify rejection handling**: Ask the AI assistant to reference a bogus category id (e.g., `nonexistent-category`) and confirm that the rejection message from MVT Server surfaces back through the MCP host to the conversation.

## Security Note

The stored MVT Server credential (email and password) grants full admin privilege; there is no scoped "styles only" role available today, so mvt-server-mcp must use a full admin account to do its job. This is a deliberate, accepted v1 trade-off, not an oversight. The password sits in plaintext in whatever configuration file or environment variable holds it — the same trust model as any local MCP server configuration. Ensure the configuration file and process environment are protected accordingly, and do not expose credentials in public repositories or logs.
