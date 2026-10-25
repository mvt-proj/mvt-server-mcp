mod catalog_view;
mod config;
mod error;
mod mvt_rs_client;
mod tools;

use mvt_rs_client::MvtRsClient;
use rmcp::{ServiceExt, transport::stdio};
use std::sync::Arc;
use tools::MvtMcpServer;

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr) // stdout is the MCP JSON-RPC channel — never write logs there
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

#[tokio::main]
async fn main() {
    init_tracing();

    let settings = match config::Settings::load() {
        Ok(settings) => settings,
        Err(e) => {
            tracing::error!("failed to load configuration: {e}");
            std::process::exit(1);
        }
    };

    let client = Arc::new(MvtRsClient::new(
        settings.mvt_rs.base_url,
        settings.mvt_rs.email,
        settings.mvt_rs.password,
    ));
    let server = MvtMcpServer::new(client);

    let service = match server.serve(stdio()).await {
        Ok(service) => service,
        Err(e) => {
            tracing::error!("failed to start MCP stdio server: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = service.waiting().await {
        tracing::error!("MCP server exited with error: {e}");
        std::process::exit(1);
    }
}
