use crate::error::ClientError;
use serde::{Deserialize, Serialize};

// `Serialize` is added here (beyond the brief's literal `Deserialize`-only
// derive) because `RemoteStyle` below derives `Serialize` and embeds a
// `RemoteCategory` field; without this, `RemoteStyle`'s derive fails to
// compile (`RemoteCategory: Serialize` not satisfied).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCategory {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteLayer {
    pub id: String,
    pub category: RemoteCategory,
    pub geometry: String,
    pub name: String,
    pub alias: String,
    pub description: String,
    pub database_id: String,
    pub schema: String,
    pub table_name: String,
    pub fields: Vec<String>,
    pub published: bool,
}
// Deliberately no #[serde(deny_unknown_fields)] — mvt-rs's real Layer JSON
// has many more keys (srid, buffer, groups, source, ...) we don't need;
// tolerating unknown fields keeps this forward-compatible.

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteField {
    pub name: String,
    pub udt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStyle {
    pub id: String,
    pub name: String,
    pub category: RemoteCategory,
    pub description: String,
    pub style: String,
}

#[derive(Serialize)]
struct LoginBody<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

#[derive(Serialize)]
struct StyleWriteBody<'a> {
    name: &'a str,
    category: &'a str,
    description: &'a str,
    style: String,
}

#[derive(Serialize)]
struct CategoryWriteBody<'a> {
    name: &'a str,
    description: &'a str,
}

/// The fields mvt-server-mcp lets a model set when creating or replacing a catalog
/// layer, in mvt-server-mcp's own naming (`category_id`, `table_name`) — mapped to
/// mvt-rs's wire names (`category`, `table`) only inside `layer_write_body`.
/// mvt-rs's update endpoint replaces the whole layer record, not a partial
/// patch, so callers (`tools.rs`) are expected to populate every field with
/// its current value except whatever they mean to change — see `get_layer`.
#[derive(Debug, Clone)]
pub struct LayerInput {
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

#[derive(Serialize)]
struct LayerWriteBody<'a> {
    category: &'a str,
    database_id: &'a str,
    geometry: &'a str,
    name: &'a str,
    alias: &'a str,
    description: &'a str,
    schema: &'a str,
    table: &'a str,
    fields: &'a [String],
    published: bool,
    filter: Option<&'a str>,
    srid: Option<u32>,
    geom: Option<&'a str>,
    label_layer: Option<bool>,
    sql_mode: Option<&'a str>,
    buffer: Option<u32>,
    extent: Option<u32>,
    zmin: Option<u32>,
    zmax: Option<u32>,
    zmax_do_not_simplify: Option<u32>,
    buffer_do_not_simplify: Option<u32>,
    extent_do_not_simplify: Option<u32>,
    clip_geom: Option<bool>,
    delete_cache_on_start: Option<bool>,
    max_cache_age: Option<u64>,
    max_records: Option<u64>,
    groups: Option<&'a [String]>,
}

fn layer_write_body(input: &LayerInput) -> LayerWriteBody<'_> {
    LayerWriteBody {
        category: &input.category_id,
        database_id: &input.database_id,
        geometry: &input.geometry,
        name: &input.name,
        alias: &input.alias,
        description: &input.description,
        schema: &input.schema,
        table: &input.table_name,
        fields: &input.fields,
        published: input.published,
        filter: input.filter.as_deref(),
        srid: input.srid,
        geom: input.geom.as_deref(),
        label_layer: input.label_layer,
        sql_mode: input.sql_mode.as_deref(),
        buffer: input.buffer,
        extent: input.extent,
        zmin: input.zmin,
        zmax: input.zmax,
        zmax_do_not_simplify: input.zmax_do_not_simplify,
        buffer_do_not_simplify: input.buffer_do_not_simplify,
        extent_do_not_simplify: input.extent_do_not_simplify,
        clip_geom: input.clip_geom,
        delete_cache_on_start: input.delete_cache_on_start,
        max_cache_age: input.max_cache_age,
        max_records: input.max_records,
        groups: input.groups.as_deref(),
    }
}

pub struct MvtRsClient {
    http: reqwest::Client,
    base_url: String,
    email: String,
    password: String,
    token: tokio::sync::Mutex<Option<String>>,
}

impl MvtRsClient {
    pub fn new(
        base_url: impl Into<String>,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client configuration must be valid");
        Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            email: email.into(),
            password: password.into(),
            token: tokio::sync::Mutex::new(None),
        }
    }

    async fn login(&self) -> Result<String, ClientError> {
        let url = format!("{}/api/users/login", self.base_url);
        let body = LoginBody {
            email: &self.email,
            password: &self.password,
        };
        let resp = self.http.post(&url).json(&body).send().await?;
        let login_resp: LoginResponse = Self::into_result(resp).await?;
        *self.token.lock().await = Some(login_resp.token.clone());
        Ok(login_resp.token)
    }

    async fn token(&self) -> Result<String, ClientError> {
        if let Some(t) = self.token.lock().await.as_ref() {
            return Ok(t.clone());
        }
        self.login().await
    }

    async fn into_result<T: serde::de::DeserializeOwned>(
        resp: reqwest::Response,
    ) -> Result<T, ClientError> {
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if status.is_success() {
            Ok(serde_json::from_slice(&bytes)?)
        } else {
            Err(crate::error::parse_error_body(status, &bytes))
        }
    }

    async fn get_authorized<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, ClientError> {
        let token = self.token().await?;
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .query(query)
            .bearer_auth(&token)
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            let fresh = self.login().await?;
            let retry = self
                .http
                .get(&url)
                .query(query)
                .bearer_auth(&fresh)
                .send()
                .await?;
            return Self::into_result(retry).await;
        }
        Self::into_result(resp).await
    }

    pub async fn list_categories(&self) -> Result<Vec<RemoteCategory>, ClientError> {
        self.get_authorized("/api/admin/categories", &[]).await
    }

    pub async fn create_category(
        &self,
        name: &str,
        description: &str,
    ) -> Result<RemoteCategory, ClientError> {
        let body = CategoryWriteBody { name, description };
        self.send_authorized(reqwest::Method::POST, "/api/admin/categories", &body)
            .await
    }

    pub async fn update_category(
        &self,
        id: &str,
        name: &str,
        description: &str,
    ) -> Result<RemoteCategory, ClientError> {
        let body = CategoryWriteBody { name, description };
        self.send_authorized(
            reqwest::Method::PUT,
            &format!("/api/admin/categories/{id}"),
            &body,
        )
        .await
    }

    pub async fn list_styles(&self) -> Result<Vec<RemoteStyle>, ClientError> {
        self.get_authorized("/api/admin/styles", &[]).await
    }

    pub async fn list_catalog_layers(&self) -> Result<Vec<RemoteLayer>, ClientError> {
        let url = format!("{}/api/catalog/layer", self.base_url);
        let resp = self.http.get(&url).send().await?;
        Self::into_result(resp).await
    }

    /// Fetches one layer's complete raw catalog JSON (every field mvt-rs
    /// stores — filter, srid, buffer, extent, zmin/zmax, groups, etc., not
    /// just the curated subset `RemoteLayer`/`LayerView` expose) from the
    /// same public, unauthenticated listing `list_catalog_layers` uses. A
    /// model needs this before `update_layer`, since that endpoint replaces
    /// the whole layer record rather than patching it.
    pub async fn get_full_layer_json(&self, id: &str) -> Result<serde_json::Value, ClientError> {
        let url = format!("{}/api/catalog/layer", self.base_url);
        let resp = self.http.get(&url).send().await?;
        let layers: Vec<serde_json::Value> = Self::into_result(resp).await?;
        layers
            .into_iter()
            .find(|layer| layer.get("id").and_then(|v| v.as_str()) == Some(id))
            .ok_or_else(|| ClientError::Api {
                status: 404,
                message: format!("No layer found with id '{id}'"),
                kind: "NotFound".to_string(),
            })
    }

    pub async fn create_layer(&self, input: &LayerInput) -> Result<serde_json::Value, ClientError> {
        let body = layer_write_body(input);
        self.send_authorized(reqwest::Method::POST, "/api/admin/catalog/layer", &body)
            .await
    }

    pub async fn update_layer(
        &self,
        id: &str,
        input: &LayerInput,
    ) -> Result<serde_json::Value, ClientError> {
        let body = layer_write_body(input);
        self.send_authorized(
            reqwest::Method::PUT,
            &format!("/api/admin/catalog/layer/{id}"),
            &body,
        )
        .await
    }

    /// Drops every cached tile for this layer so the next request for any
    /// of its tiles is rendered fresh from the database. mvt-rs takes no
    /// body for this call.
    pub async fn delete_layer_cache(&self, id: &str) -> Result<serde_json::Value, ClientError> {
        self.send_authorized(
            reqwest::Method::DELETE,
            &format!("/api/admin/catalog/layer/{id}/cache"),
            &(),
        )
        .await
    }

    /// `schema`/`table` are URL path segments (mvt-rs mounts this route as
    /// `fields/{schema}/{table}`) — only `database_id` is a query param.
    pub async fn list_database_fields(
        &self,
        database_id: &str,
        schema: &str,
        table: &str,
    ) -> Result<Vec<RemoteField>, ClientError> {
        let path = format!("/api/admin/database/fields/{schema}/{table}");
        self.get_authorized(&path, &[("database_id", database_id)])
            .await
    }

    async fn send_authorized<B: Serialize, T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &B,
    ) -> Result<T, ClientError> {
        let token = self.token().await?;
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .request(method.clone(), &url)
            .bearer_auth(&token)
            .json(body)
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            let fresh = self.login().await?;
            let retry = self
                .http
                .request(method, &url)
                .bearer_auth(&fresh)
                .json(body)
                .send()
                .await?;
            return Self::into_result(retry).await;
        }
        Self::into_result(resp).await
    }

    pub async fn create_style(
        &self,
        name: &str,
        category_id: &str,
        description: &str,
        style_json: &serde_json::Value,
    ) -> Result<RemoteStyle, ClientError> {
        let body = StyleWriteBody {
            name,
            category: category_id,
            description,
            style: serde_json::to_string(style_json)?,
        };
        self.send_authorized(reqwest::Method::POST, "/api/admin/styles", &body)
            .await
    }

    pub async fn update_style(
        &self,
        id: &str,
        name: &str,
        category_id: &str,
        description: &str,
        style_json: &serde_json::Value,
    ) -> Result<RemoteStyle, ClientError> {
        let body = StyleWriteBody {
            name,
            category: category_id,
            description,
            style: serde_json::to_string(style_json)?,
        };
        self.send_authorized(
            reqwest::Method::PUT,
            &format!("/api/admin/styles/{id}"),
            &body,
        )
        .await
    }

    /// Combines the public catalog listing with one field-introspection call
    /// per layer, then reshapes each via `catalog_view::build_layer_view`. A
    /// field-introspection failure for any single layer fails the whole call
    /// (simple "all or nothing" semantics for v1) rather than silently
    /// dropping that layer or returning partial results.
    pub async fn list_layers_with_fields(
        &self,
    ) -> Result<Vec<crate::catalog_view::LayerView>, ClientError> {
        let layers = self.list_catalog_layers().await?;
        let mut views = Vec::with_capacity(layers.len());
        for layer in &layers {
            let table_fields = self
                .list_database_fields(&layer.database_id, &layer.schema, &layer.table_name)
                .await?;
            views.push(crate::catalog_view::build_layer_view(layer, &table_fields));
        }
        Ok(views)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{bearer_token, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mount_login_success(server: &MockServer, token: &str) {
        Mock::given(method("POST"))
            .and(path("/api/users/login"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"token": token})),
            )
            .mount(server)
            .await;
    }

    fn style_response_body(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": "roads-style",
            "category": {"id": "c1", "name": "Roads", "description": "d"},
            "description": "d",
            "style": "{}"
        })
    }

    fn category_response_body(id: &str, name: &str, description: &str) -> serde_json::Value {
        serde_json::json!({"id": id, "name": name, "description": description})
    }

    fn full_layer_json(id: &str, srid: u32) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "category": {"id": "c1", "name": "Roads", "description": "d"},
            "geometry": "lines",
            "name": "roads",
            "alias": "Roads",
            "description": "d",
            "database_id": "default",
            "schema": "public",
            "table_name": "roads",
            "fields": ["name"],
            "filter": "",
            "srid": srid,
            "published": true,
            "groups": []
        })
    }

    fn layer_json_with_table(
        id: &str,
        name: &str,
        table_name: &str,
        fields: &[&str],
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "category": {"id": "cat-1", "name": "Category 1", "description": "desc1"},
            "geometry": "Point",
            "name": name,
            "alias": format!("{name} Alias"),
            "description": format!("{name} description"),
            "database_id": "db-1",
            "schema": "public",
            "table_name": table_name,
            "fields": fields,
            "published": true
        })
    }

    fn sample_layer_input() -> LayerInput {
        LayerInput {
            category_id: "cat-1".into(),
            database_id: "default".into(),
            geometry: "lines".into(),
            name: "roads".into(),
            alias: "Roads".into(),
            description: "Road network".into(),
            schema: "public".into(),
            table_name: "roads".into(),
            fields: vec!["name".into()],
            published: true,
            filter: None,
            srid: Some(4326),
            geom: None,
            label_layer: None,
            sql_mode: None,
            buffer: None,
            extent: None,
            zmin: None,
            zmax: None,
            zmax_do_not_simplify: None,
            buffer_do_not_simplify: None,
            extent_do_not_simplify: None,
            clip_geom: None,
            delete_cache_on_start: None,
            max_cache_age: None,
            max_records: None,
            groups: None,
        }
    }

    #[tokio::test]
    async fn login_success_and_token_is_cached_across_calls() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("GET"))
            .and(path("/api/admin/categories"))
            .and(bearer_token("tok-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");

        let first = client.list_categories().await;
        let second = client.list_categories().await;

        assert!(first.is_ok());
        assert!(second.is_ok());

        let requests = server
            .received_requests()
            .await
            .expect("request recording must be enabled");
        let login_calls = requests
            .iter()
            .filter(|r| r.url.path() == "/api/users/login")
            .count();
        assert_eq!(login_calls, 1, "token should be cached and reused");
    }

    #[tokio::test]
    async fn login_failure_surfaces_mvt_rs_error_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/users/login"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "status": 401, "error": "Unauthorized", "type": "UnauthorizedAccess"
            })))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.list_categories().await;

        match result {
            Err(ClientError::Api { status, .. }) => assert_eq!(status, 401),
            other => panic!("expected ClientError::Api with status 401, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resource_401_triggers_single_reauth_and_retry_succeeds() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("GET"))
            .and(path("/api/admin/categories"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "status": 401, "error": "Unauthorized", "type": "UnauthorizedAccess"
            })))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/admin/categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .with_priority(2)
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.list_categories().await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");

        let requests = server
            .received_requests()
            .await
            .expect("request recording must be enabled");
        let login_calls = requests
            .iter()
            .filter(|r| r.url.path() == "/api/users/login")
            .count();
        let categories_calls = requests
            .iter()
            .filter(|r| r.url.path() == "/api/admin/categories")
            .count();
        assert_eq!(
            login_calls, 2,
            "expected the initial login plus one re-login after 401"
        );
        assert_eq!(
            categories_calls, 2,
            "expected the initial 401 plus one retry"
        );
    }

    #[tokio::test]
    async fn resource_double_401_surfaces_as_api_error() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("GET"))
            .and(path("/api/admin/categories"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "status": 401, "error": "Unauthorized", "type": "UnauthorizedAccess"
            })))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.list_categories().await;

        match result {
            Err(ClientError::Api { status, .. }) => assert_eq!(status, 401),
            other => panic!("expected ClientError::Api with status 401, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn non_2xx_error_body_is_parsed_into_client_error() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("GET"))
            .and(path("/api/admin/categories"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "status": 404, "error": "Category abc not found", "type": "NotFound"
            })))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.list_categories().await;

        match result {
            Err(ClientError::Api {
                status,
                message,
                kind,
            }) => {
                assert_eq!(status, 404);
                assert_eq!(message, "Category abc not found");
                assert_eq!(kind, "NotFound");
            }
            other => panic!("expected ClientError::Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_catalog_layers_sends_no_authorization_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/catalog/layer"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([full_layer_json("layer-1", 4326)])),
            )
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.list_catalog_layers().await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let requests = server
            .received_requests()
            .await
            .expect("request recording must be enabled");
        assert_eq!(
            requests.len(),
            1,
            "no login call should ever happen for this public endpoint"
        );
        assert!(requests[0].headers.get("authorization").is_none());
    }

    #[tokio::test]
    async fn list_database_fields_sends_schema_table_as_path_segments_and_database_id_as_query() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("GET"))
            .and(path("/api/admin/database/fields/public/roads"))
            .and(query_param("database_id", "default"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client
            .list_database_fields("default", "public", "roads")
            .await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    #[tokio::test]
    async fn create_style_sends_style_field_as_json_string() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("POST"))
            .and(path("/api/admin/styles"))
            .respond_with(ResponseTemplate::new(200).set_body_json(style_response_body("style-1")))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let style_json = serde_json::json!({"version": 8, "layers": []});
        let result = client
            .create_style("roads-style", "c1", "d", &style_json)
            .await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");

        let requests = server
            .received_requests()
            .await
            .expect("request recording must be enabled");
        let create_request = requests
            .iter()
            .find(|r| r.url.path() == "/api/admin/styles")
            .expect("styles POST must have been captured");
        let json_body: serde_json::Value = create_request.body_json().expect("body must be JSON");

        assert_eq!(json_body["category"], serde_json::json!("c1"));
        let style_field = json_body["style"]
            .as_str()
            .expect("style field must be a JSON string");
        let round_tripped: serde_json::Value =
            serde_json::from_str(style_field).expect("style field must parse as JSON");
        assert_eq!(round_tripped, style_json);
    }

    #[tokio::test]
    async fn update_style_puts_to_the_correct_id_path() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("PUT"))
            .and(path("/api/admin/styles/style-42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(style_response_body("style-42")))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let style_json = serde_json::json!({"version": 8, "layers": []});
        let result = client
            .update_style("style-42", "roads-style", "c1", "d", &style_json)
            .await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    #[tokio::test]
    async fn list_styles_returns_real_style_data() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("GET"))
            .and(path("/api/admin/styles"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([style_response_body("style-1")])),
            )
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.list_styles().await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let styles = result.unwrap();
        assert_eq!(styles.len(), 1);
        assert_eq!(styles[0].id, "style-1");
        assert_eq!(styles[0].category.name, "Roads");
    }

    #[tokio::test]
    async fn create_category_sends_name_and_description() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("POST"))
            .and(path("/api/admin/categories"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(category_response_body(
                    "cat-1",
                    "parks",
                    "Parks and recreation",
                )),
            )
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client
            .create_category("parks", "Parks and recreation")
            .await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let category = result.unwrap();
        assert_eq!(category.id, "cat-1");
        assert_eq!(category.name, "parks");

        let requests = server
            .received_requests()
            .await
            .expect("request recording must be enabled");
        let create_request = requests
            .iter()
            .find(|r| r.url.path() == "/api/admin/categories")
            .expect("categories POST must have been captured");
        let json_body: serde_json::Value = create_request.body_json().expect("body must be JSON");
        assert_eq!(json_body["name"], serde_json::json!("parks"));
        assert_eq!(
            json_body["description"],
            serde_json::json!("Parks and recreation")
        );
    }

    #[tokio::test]
    async fn update_category_puts_to_the_correct_id_path() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("PUT"))
            .and(path("/api/admin/categories/cat-42"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(category_response_body(
                    "cat-42",
                    "parks-renamed",
                    "Updated description",
                )),
            )
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client
            .update_category("cat-42", "parks-renamed", "Updated description")
            .await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let category = result.unwrap();
        assert_eq!(category.name, "parks-renamed");
    }

    #[tokio::test]
    async fn delete_layer_cache_deletes_to_the_correct_id_path() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("DELETE"))
            .and(path("/api/admin/catalog/layer/layer-42/cache"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"deleted": true})),
            )
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.delete_layer_cache("layer-42").await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), serde_json::json!({"deleted": true}));
    }

    #[tokio::test]
    async fn combines_catalog_and_per_layer_field_lookups_over_real_http() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("GET"))
            .and(path("/api/catalog/layer"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                layer_json_with_table("layer-1", "Layer 1", "table_1", &["name", "type"]),
                layer_json_with_table("layer-2", "Layer 2", "table_2", &["id", "geom"]),
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/admin/database/fields/public/table_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"name": "name", "udt": "text"},
                {"name": "type", "udt": "varchar"}
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/admin/database/fields/public/table_2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"name": "id", "udt": "integer"},
                {"name": "geom", "udt": "geometry"}
            ])))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.list_layers_with_fields().await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let views = result.unwrap();
        assert_eq!(views.len(), 2, "expected 2 layer views");

        assert_eq!(views[0].id, "layer-1");
        assert_eq!(views[0].name, "Layer 1");
        assert_eq!(views[0].fields.len(), 2);
        assert_eq!(views[0].fields[0].name, "name");
        assert_eq!(views[0].fields[0].r#type, "text");
        assert_eq!(views[0].fields[1].name, "type");
        assert_eq!(views[0].fields[1].r#type, "varchar");

        assert_eq!(views[1].id, "layer-2");
        assert_eq!(views[1].name, "Layer 2");
        assert_eq!(views[1].fields.len(), 2);
        assert_eq!(views[1].fields[0].name, "id");
        assert_eq!(views[1].fields[0].r#type, "integer");
        assert_eq!(views[1].fields[1].name, "geom");
        assert_eq!(views[1].fields[1].r#type, "geometry");
    }

    #[tokio::test]
    async fn propagates_a_field_lookup_failure_for_any_single_layer() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("GET"))
            .and(path("/api/catalog/layer"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                layer_json_with_table("layer-1", "Layer 1", "table_1", &["name"]),
                layer_json_with_table("layer-2", "Layer 2", "table_2", &["id"]),
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/admin/database/fields/public/table_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"name": "name", "udt": "text"}
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/admin/database/fields/public/table_2"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(serde_json::json!({"error": "internal error"})),
            )
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.list_layers_with_fields().await;

        match result {
            Err(ClientError::Api { status, .. }) => assert_eq!(status, 500),
            other => panic!("expected ClientError::Api with status 500, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn propagates_a_catalog_listing_failure_without_calling_fields() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/catalog/layer"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(serde_json::json!({"error": "internal error"})),
            )
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.list_layers_with_fields().await;

        assert!(result.is_err(), "expected Err but got Ok");

        let requests = server
            .received_requests()
            .await
            .expect("request recording must be enabled");
        let fields_calls = requests
            .iter()
            .filter(|r| r.url.path().starts_with("/api/admin/database/fields"))
            .count();
        assert_eq!(
            fields_calls, 0,
            "fields endpoint should not have been called"
        );
    }

    #[tokio::test]
    async fn get_full_layer_json_finds_the_matching_layer_by_id_without_auth() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/catalog/layer"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                full_layer_json("layer-1", 4326),
                full_layer_json("layer-2", 22174),
            ])))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.get_full_layer_json("layer-2").await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let layer = result.unwrap();
        assert_eq!(layer["id"], serde_json::json!("layer-2"));
        assert_eq!(layer["srid"], serde_json::json!(22174));
    }

    #[tokio::test]
    async fn get_full_layer_json_returns_not_found_for_a_missing_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/catalog/layer"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                full_layer_json("layer-1", 4326),
                full_layer_json("layer-2", 22174),
            ])))
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.get_full_layer_json("does-not-exist").await;

        match result {
            Err(ClientError::Api { status, .. }) => assert_eq!(status, 404),
            other => panic!("expected ClientError::Api with status 404, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_layer_sends_mvt_rs_wire_field_names() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("POST"))
            .and(path("/api/admin/catalog/layer"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(full_layer_json("layer-1", 4326)),
            )
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.create_layer(&sample_layer_input()).await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");

        let requests = server
            .received_requests()
            .await
            .expect("request recording must be enabled");
        let create_request = requests
            .iter()
            .find(|r| r.url.path() == "/api/admin/catalog/layer")
            .expect("catalog layer POST must have been captured");
        let json_body: serde_json::Value = create_request.body_json().expect("body must be JSON");

        // `category` (not `category_id`) and `table` (not `table_name`) are
        // mvt-rs's actual wire field names.
        assert_eq!(json_body["category"], serde_json::json!("cat-1"));
        assert_eq!(json_body["table"], serde_json::json!("roads"));
        assert!(json_body.get("category_id").is_none());
        assert!(json_body.get("table_name").is_none());
        assert_eq!(json_body["srid"], serde_json::json!(4326));
    }

    #[tokio::test]
    async fn update_layer_puts_to_the_correct_id_path() {
        let server = MockServer::start().await;
        mount_login_success(&server, "tok-123").await;
        Mock::given(method("PUT"))
            .and(path("/api/admin/catalog/layer/layer-42"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(full_layer_json("layer-42", 4326)),
            )
            .mount(&server)
            .await;

        let client = MvtRsClient::new(server.uri(), "user@example.com", "pw");
        let result = client.update_layer("layer-42", &sample_layer_input()).await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }
}
