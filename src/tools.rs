use crate::catalog_view::{self, CategoryView, StyleView};
use crate::error::ClientError;
use crate::mvt_rs_client::{LayerInput, MvtRsClient};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};
use std::sync::Arc;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateStyleRequest {
    pub name: String,
    pub category_id: String,
    pub description: String,
    /// Typed as a JSON object map (not `serde_json::Value`) purely so the
    /// generated tool schema advertises `{"type":"object"}` instead of the
    /// permissive `true` schema `Value` produces — some MCP clients (e.g.
    /// Claude Code) reject a boolean JSON Schema for a tool input property.
    /// Values inside the object remain fully opaque JSON, so this changes
    /// nothing about mvt-server-mcp never inspecting style content.
    pub style_json: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetStyleRequest {
    pub id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateCategoryRequest {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateStyleRequest {
    pub id: String,
    pub name: String,
    pub category_id: String,
    pub description: String,
    pub style_json: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetLayerRequest {
    pub id: String,
}

/// Every field mvt-rs's own layer-create/update form accepts. `update_layer`
/// replaces the whole catalog entry (no partial patch), so a model intending
/// to change only one field (e.g. moving a layer to a different category)
/// must still supply every other field unchanged — call `get_layer` first to
/// see the layer's current values for all of them.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateLayerRequest {
    pub category_id: String,
    pub database_id: String,
    pub geometry: String,
    pub name: String,
    pub alias: String,
    pub description: String,
    pub schema: String,
    pub table_name: String,
    pub fields: Vec<String>,
    pub published: bool,
    pub filter: Option<String>,
    pub srid: Option<u32>,
    pub geom: Option<String>,
    pub label_layer: Option<bool>,
    pub sql_mode: Option<String>,
    pub buffer: Option<u32>,
    pub extent: Option<u32>,
    pub zmin: Option<u32>,
    pub zmax: Option<u32>,
    pub zmax_do_not_simplify: Option<u32>,
    pub buffer_do_not_simplify: Option<u32>,
    pub extent_do_not_simplify: Option<u32>,
    pub clip_geom: Option<bool>,
    pub delete_cache_on_start: Option<bool>,
    pub max_cache_age: Option<u64>,
    pub max_records: Option<u64>,
    pub groups: Option<Vec<String>>,
}

impl From<CreateLayerRequest> for LayerInput {
    fn from(r: CreateLayerRequest) -> Self {
        LayerInput {
            category_id: r.category_id,
            database_id: r.database_id,
            geometry: r.geometry,
            name: r.name,
            alias: r.alias,
            description: r.description,
            schema: r.schema,
            table_name: r.table_name,
            fields: r.fields,
            published: r.published,
            filter: r.filter,
            srid: r.srid,
            geom: r.geom,
            label_layer: r.label_layer,
            sql_mode: r.sql_mode,
            buffer: r.buffer,
            extent: r.extent,
            zmin: r.zmin,
            zmax: r.zmax,
            zmax_do_not_simplify: r.zmax_do_not_simplify,
            buffer_do_not_simplify: r.buffer_do_not_simplify,
            extent_do_not_simplify: r.extent_do_not_simplify,
            clip_geom: r.clip_geom,
            delete_cache_on_start: r.delete_cache_on_start,
            max_cache_age: r.max_cache_age,
            max_records: r.max_records,
            groups: r.groups,
        }
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateLayerRequest {
    pub id: String,
    pub category_id: String,
    pub database_id: String,
    pub geometry: String,
    pub name: String,
    pub alias: String,
    pub description: String,
    pub schema: String,
    pub table_name: String,
    pub fields: Vec<String>,
    pub published: bool,
    pub filter: Option<String>,
    pub srid: Option<u32>,
    pub geom: Option<String>,
    pub label_layer: Option<bool>,
    pub sql_mode: Option<String>,
    pub buffer: Option<u32>,
    pub extent: Option<u32>,
    pub zmin: Option<u32>,
    pub zmax: Option<u32>,
    pub zmax_do_not_simplify: Option<u32>,
    pub buffer_do_not_simplify: Option<u32>,
    pub extent_do_not_simplify: Option<u32>,
    pub clip_geom: Option<bool>,
    pub delete_cache_on_start: Option<bool>,
    pub max_cache_age: Option<u64>,
    pub max_records: Option<u64>,
    pub groups: Option<Vec<String>>,
}

impl From<UpdateLayerRequest> for LayerInput {
    fn from(r: UpdateLayerRequest) -> Self {
        LayerInput {
            category_id: r.category_id,
            database_id: r.database_id,
            geometry: r.geometry,
            name: r.name,
            alias: r.alias,
            description: r.description,
            schema: r.schema,
            table_name: r.table_name,
            fields: r.fields,
            published: r.published,
            filter: r.filter,
            srid: r.srid,
            geom: r.geom,
            label_layer: r.label_layer,
            sql_mode: r.sql_mode,
            buffer: r.buffer,
            extent: r.extent,
            zmin: r.zmin,
            zmax: r.zmax,
            zmax_do_not_simplify: r.zmax_do_not_simplify,
            buffer_do_not_simplify: r.buffer_do_not_simplify,
            extent_do_not_simplify: r.extent_do_not_simplify,
            clip_geom: r.clip_geom,
            delete_cache_on_start: r.delete_cache_on_start,
            max_cache_age: r.max_cache_age,
            max_records: r.max_records,
            groups: r.groups,
        }
    }
}

/// Renders a successful tool result as pretty-printed JSON text.
fn success_json<T: serde::Serialize>(value: &T) -> CallToolResult {
    let text = serde_json::to_string_pretty(value)
        .unwrap_or_else(|e| format!("{{\"serialization_error\":\"{e}\"}}"));
    CallToolResult::success(vec![ContentBlock::text(text)])
}

/// Surfaces every `ClientError` as a tool-level error (`CallToolResult::error`,
/// never `Err(McpError)`) — the model must see mvt-rs's own rejection reason
/// to self-correct.
fn client_error_to_tool_result(err: ClientError) -> CallToolResult {
    let message = match &err {
        ClientError::Api {
            status,
            message,
            kind,
        } => format!("mvt-rs rejected the request (HTTP {status}, {kind}): {message}"),
        ClientError::Http(e) => format!("could not reach mvt-rs: {e}"),
        ClientError::Decode(e) => format!("mvt-rs returned an unexpected response: {e}"),
    };
    CallToolResult::error(vec![ContentBlock::text(message)])
}

// `tool_router` is written by `#[tool_router]`-generated code and read by
// `#[tool_handler]`-generated dispatch, not by any code visible to rustc's
// dead-code analysis in this module — it's live at runtime (proven by the
// server actually serving tools), so the allow stays.
#[allow(dead_code)]
#[derive(Clone)]
pub struct MvtMcpServer {
    client: Arc<MvtRsClient>,
    tool_router: ToolRouter<MvtMcpServer>,
}

#[tool_router]
impl MvtMcpServer {
    pub fn new(client: Arc<MvtRsClient>) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List every layer in the mvt-rs catalog, with its id, name, category, geometry type, published state, and the name+type of every field it exposes."
    )]
    async fn list_layers(&self) -> Result<CallToolResult, McpError> {
        match self.client.list_layers_with_fields().await {
            Ok(views) => Ok(success_json(&views)),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "Get one catalog layer's complete raw configuration by id — every field mvt-rs stores (filter, srid, buffer, extent, zoom range, groups, etc.), not just the summary list_layers shows. Always call this before update_layer, since update_layer replaces the whole entry and needs every field's current value for anything you don't intend to change."
    )]
    async fn get_layer(
        &self,
        Parameters(req): Parameters<GetLayerRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.get_full_layer_json(&req.id).await {
            Ok(layer) => Ok(success_json(&layer)),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "Register a new layer in the mvt-rs catalog from an existing Postgres table, making it servable as vector tiles. The database/schema/table must already exist and be reachable from mvt-rs; this only creates the catalog entry, it never creates or alters any database table."
    )]
    async fn create_layer(
        &self,
        Parameters(req): Parameters<CreateLayerRequest>,
    ) -> Result<CallToolResult, McpError> {
        let input: LayerInput = req.into();
        match self.client.create_layer(&input).await {
            Ok(layer) => Ok(success_json(&layer)),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "Replace an existing layer's full catalog entry in mvt-rs by id — e.g. to move it to a different category, rename it, or change its published state. This replaces the whole entry, not a partial patch: call get_layer first and pass every field back with its current value except what you mean to change."
    )]
    async fn update_layer(
        &self,
        Parameters(req): Parameters<UpdateLayerRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = req.id.clone();
        let input: LayerInput = req.into();
        match self.client.update_layer(&id, &input).await {
            Ok(layer) => Ok(success_json(&layer)),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "List every style category known to mvt-rs (id, name, description). The id is required by create_style/update_style's category_id parameter."
    )]
    async fn list_categories(&self) -> Result<CallToolResult, McpError> {
        match self.client.list_categories().await {
            Ok(cats) => Ok(success_json(
                &cats.into_iter().map(CategoryView::from).collect::<Vec<_>>(),
            )),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "Create a new style category in mvt-rs. The returned id is required by create_style/update_style's category_id parameter."
    )]
    async fn create_category(
        &self,
        Parameters(req): Parameters<CreateCategoryRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .create_category(&req.name, &req.description)
            .await
        {
            Ok(category) => Ok(success_json(&CategoryView::from(category))),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "Rename or redescribe an existing style category in mvt-rs by id. Use list_categories first to find the id."
    )]
    async fn update_category(
        &self,
        Parameters(req): Parameters<UpdateCategoryRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .update_category(&req.id, &req.name, &req.description)
            .await
        {
            Ok(category) => Ok(success_json(&CategoryView::from(category))),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "List every existing MapLibre style in mvt-rs, with its id, name, category, and description (not the style body itself). Use this to find an existing style's id before calling update_style, or to see what already exists in a category before creating a new one."
    )]
    async fn list_styles(&self) -> Result<CallToolResult, McpError> {
        match self.client.list_styles().await {
            Ok(styles) => Ok(success_json(
                &styles.into_iter().map(StyleView::from).collect::<Vec<_>>(),
            )),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "Get the full detail of one existing MapLibre style in mvt-rs by id, including its complete style_json body (parsed, not escaped). Use list_styles first to find the id."
    )]
    async fn get_style(
        &self,
        Parameters(req): Parameters<GetStyleRequest>,
    ) -> Result<CallToolResult, McpError> {
        let result: Result<catalog_view::StyleDetailView, ClientError> = async {
            let styles = self.client.list_styles().await?;
            match styles.into_iter().find(|s| s.id == req.id) {
                Some(style) => {
                    catalog_view::build_style_detail_view(style).map_err(ClientError::from)
                }
                None => Err(ClientError::Api {
                    status: 404,
                    message: format!("No style found with id '{}'", req.id),
                    kind: "NotFound".to_string(),
                }),
            }
        }
        .await;

        match result {
            Ok(view) => Ok(success_json(&view)),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "Create a new MapLibre style in mvt-rs. style_json must be a complete, valid MapLibre style JSON object; mvt-server-mcp does not validate it semantically — only mvt-rs's own JSON-parse check applies."
    )]
    async fn create_style(
        &self,
        Parameters(req): Parameters<CreateStyleRequest>,
    ) -> Result<CallToolResult, McpError> {
        let style_json = serde_json::Value::Object(req.style_json);
        match self
            .client
            .create_style(&req.name, &req.category_id, &req.description, &style_json)
            .await
        {
            Ok(style) => Ok(success_json(&style)),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }

    #[tool(
        description = "Replace an existing MapLibre style in mvt-rs by id. This replaces the whole style, not a partial patch."
    )]
    async fn update_style(
        &self,
        Parameters(req): Parameters<UpdateStyleRequest>,
    ) -> Result<CallToolResult, McpError> {
        let style_json = serde_json::Value::Object(req.style_json);
        match self
            .client
            .update_style(
                &req.id,
                &req.name,
                &req.category_id,
                &req.description,
                &style_json,
            )
            .await
        {
            Ok(style) => Ok(success_json(&style)),
            Err(err) => Ok(client_error_to_tool_result(err)),
        }
    }
}

#[tool_handler]
impl ServerHandler for MvtMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "Discover mvt-rs catalog layers/categories and create or update MapLibre \
                 styles. You (the model) compose the MapLibre style JSON; this server only \
                 translates tool calls into mvt-rs REST calls and relays its responses, \
                 including rejections."
                    .to_string(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog_view::{FieldView, LayerView};

    #[test]
    fn success_json_renders_pretty_json_text_and_is_not_an_error() {
        let views = vec![LayerView {
            id: "layer-1".into(),
            name: "urban_centers".into(),
            alias: "Urban Centers".into(),
            description: "Major cities".into(),
            category_id: "cat-1".into(),
            category_name: "Cities".into(),
            geometry: "Point".into(),
            published: true,
            fields: vec![FieldView {
                name: "name".into(),
                r#type: "text".into(),
            }],
        }];

        let result = success_json(&views);

        assert_ne!(result.is_error, Some(true));
        let text = &result.content[0].as_text().unwrap().text;
        let round_tripped: serde_json::Value =
            serde_json::from_str(text).expect("success_json output must be valid JSON");
        let expected = serde_json::to_value(&views).expect("LayerView must serialize to JSON");

        assert_eq!(round_tripped, expected);
    }

    #[test]
    fn client_error_to_tool_result_marks_result_as_error() {
        let api_err = ClientError::Api {
            status: 401,
            message: "Unauthorized".into(),
            kind: "UnauthorizedAccess".into(),
        };

        let api_result = client_error_to_tool_result(api_err);

        assert_eq!(api_result.is_error, Some(true));
    }

    #[test]
    fn client_error_message_includes_the_mvt_rs_rejection_reason() {
        let err = ClientError::Api {
            status: 404,
            message: "Category abc not found".into(),
            kind: "NotFound".into(),
        };

        let result = client_error_to_tool_result(err);

        let text = &result.content[0].as_text().unwrap().text;
        assert!(
            text.contains("Category abc not found"),
            "expected rendered message to contain the mvt-rs rejection reason verbatim, got: {text}"
        );
    }
}
