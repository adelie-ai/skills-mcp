#![deny(warnings)]

// Binary crate for skills-mcp

use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use clap::{Parser, ValueEnum};
use skills_mcp::error::Result;
use skills_mcp::server::McpServer;
use skills_mcp::transport::StdioTransportHandler;
use serde_json::Value;
use std::fmt;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone, Debug, ValueEnum)]
enum TransportMode {
    /// STDIN/STDOUT transport (recommended for VS Code and local usage)
    Stdio,
    /// WebSocket transport (recommended for hosted MCP services)
    Websocket,
}

impl fmt::Display for TransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportMode::Stdio => write!(f, "stdio"),
            TransportMode::Websocket => write!(f, "websocket"),
        }
    }
}

#[derive(Parser)]
#[command(name = "skills-mcp")]
#[command(about = "Skills knowledge-base MCP Server")]
#[command(
    long_about = "skills-mcp stores and retrieves code snippets and how-to guides for LLM agents.\n\nUsage:\n  skills-mcp serve --mode stdio\n  skills-mcp serve --mode websocket --host 0.0.0.0 --port 8080"
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Run the MCP server
    Serve {
        /// Transport mode
        #[arg(short, long, default_value_t = TransportMode::Stdio)]
        mode: TransportMode,
        /// Port for WebSocket mode (ignored for stdio)
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
        /// Host for WebSocket mode (ignored for stdio)
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { mode, port, host } => {
            let server = McpServer::new()?;

            match mode {
                TransportMode::Stdio => run_stdio_server(server).await?,
                TransportMode::Websocket => run_websocket_server(server, &host, port).await?,
            }
        }
    }

    Ok(())
}

async fn run_stdio_server(server: McpServer) -> Result<()> {
    let server = Arc::new(server);
    let mut transport = StdioTransportHandler::new();

    loop {
        let message_str = match transport.read_message().await {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                break;
            }
        };

        if message_str.is_empty() {
            continue;
        }

        let message: Value = match serde_json::from_str(&message_str) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Error parsing JSON-RPC message: {}", e);
                let error_response = jsonrpc_error_response(None, -32700, "Parse error", None);
                if let Ok(resp_str) = serde_json::to_string(&error_response) {
                    let _ = transport.write_message(&resp_str).await;
                }
                continue;
            }
        };

        let response = handle_jsonrpc_message(Arc::clone(&server), message).await;

        if let Some(resp) = response {
            let resp_str = match serde_json::to_string(&resp) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error serializing response: {}", e);
                    continue;
                }
            };
            if let Err(e) = transport.write_message(&resp_str).await {
                eprintln!("Error writing response: {}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn run_websocket_server(server: McpServer, host: &str, port: u16) -> Result<()> {
    let server = Arc::new(server);

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(server);

    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    eprintln!("WebSocket server listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn websocket_handler(ws: WebSocketUpgrade, State(server): State<Arc<McpServer>>) -> Response {
    ws.on_upgrade(move |socket| handle_websocket_connection(socket, server))
}

async fn handle_websocket_connection(socket: axum::extract::ws::WebSocket, server: Arc<McpServer>) {
    use axum::extract::ws::Message;
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let message: Value = match serde_json::from_str(&text) {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("Error parsing JSON-RPC message: {}", e);
                        let error_response =
                            jsonrpc_error_response(None, -32700, "Parse error", None);
                        if let Ok(resp_str) = serde_json::to_string(&error_response) {
                            let _ = sender.send(Message::Text(resp_str.into())).await;
                        }
                        continue;
                    }
                };

                let response = handle_jsonrpc_message(Arc::clone(&server), message).await;

                if let Some(resp) = response
                    && let Ok(resp_str) = serde_json::to_string(&resp)
                        && let Err(e) = sender.send(Message::Text(resp_str.into())).await
                        {
                            eprintln!("Error sending WebSocket response: {}", e);
                            break;
                        }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

async fn handle_jsonrpc_message(server: Arc<McpServer>, message: Value) -> Option<Value> {
    if let Some(jsonrpc_version) = message.get("jsonrpc").and_then(|v| v.as_str())
        && jsonrpc_version != "2.0"
    {
        let id = message.get("id").cloned();
        let error_msg = format!("Invalid JSON-RPC version: {}", jsonrpc_version);
        return Some(jsonrpc_error_response(id, -32600, &error_msg, None));
    }

    let id = message.get("id").cloned();
    let method = message.get("method").and_then(|m| m.as_str());
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let is_notification = id.is_none();

    let result = match method {
        Some("initialize") => {
            let protocol_version = params
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("2024-11-05");
            let client_capabilities = params.get("capabilities").unwrap_or(&Value::Null);
            server
                .handle_initialize(protocol_version, client_capabilities)
                .await
        }
        Some("initialized") | Some("notifications/initialized") => {
            server.handle_initialized().await.map(|_| Value::Null)
        }
        Some("tools/list") => {
            if !server.is_initialized().await {
                return Some(jsonrpc_error_response(
                    id,
                    -32000,
                    "Server not initialized. Call 'initialize' first.",
                    None,
                ));
            }
            Ok(serde_json::json!({ "tools": server.list_tools() }))
        }
        Some("tools/call") => {
            if !server.is_initialized().await {
                return Some(jsonrpc_error_response(
                    id,
                    -32000,
                    "Server not initialized. Call 'initialize' first.",
                    None,
                ));
            }
            let tool_name = params.get("name").and_then(|n| n.as_str());
            let arguments = params.get("arguments").unwrap_or(&Value::Null);
            if let Some(name) = tool_name {
                server.handle_tool_call(name, arguments).await
            } else {
                return Some(jsonrpc_error_response(
                    id,
                    -32602,
                    "Invalid params: Missing tool name",
                    None,
                ));
            }
        }
        Some("shutdown") => {
            if !server.is_initialized().await {
                return Some(jsonrpc_error_response(
                    id,
                    -32000,
                    "Server not initialized. Call 'initialize' first.",
                    None,
                ));
            }
            server.handle_shutdown().await.map(|_| Value::Null)
        }
        Some(_) | None => {
            return Some(jsonrpc_error_response(
                id,
                -32601,
                &format!("Method not found: {:?}", method.unwrap_or("(missing)")),
                None,
            ));
        }
    };

    match result {
        Ok(result_value) => {
            if is_notification {
                None
            } else {
                Some(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": result_value,
                }))
            }
        }
        Err(e) => {
            if is_notification {
                None
            } else {
                Some(jsonrpc_error_response(id, -32000, &e.to_string(), None))
            }
        }
    }
}

fn jsonrpc_error_response(
    id: Option<Value>,
    code: i32,
    message: &str,
    data: Option<Value>,
) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
            "data": data,
        },
    })
}
