pub mod service;
mod token;

use anyhow::Result;
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use rmcp::transport::{
    SseServer, StreamableHttpServerConfig, StreamableHttpService, sse_server::SseServerConfig,
    streamable_http_server::session::local::LocalSessionManager,
};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, error, info};

const BIND_ADDRESS: &str = "127.0.0.1:3000";
const REMOTE_ADDRESS: &str = "https://m.evenscribe.com";
pub const USER_ID_HEADER: &str = "x-evenscribe-header";

#[derive(Clone, Debug)]
struct McpAppState {
    jwks: Arc<token::Jwks>,
}

impl McpAppState {
    async fn new() -> Self {
        let jwks_url = std::env::var("JWKS_URL").expect("JWKS_URL not set.");
        let jwks = token::get_jwks(jwks_url)
            .await
            .unwrap_or_else(|e| panic!("{}", e));
        Self {
            jwks: Arc::new(jwks),
        }
    }
}

async fn validate_token_middleware(
    State(token_store): State<Arc<McpAppState>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let auth_header = request.headers().get("Authorization");

    let token = match auth_header {
        Some(header) => {
            let token = header.to_str().ok().and_then(|s| s.strip_prefix("Bearer "));

            match token {
                Some(t) => t,
                None => return StatusCode::UNAUTHORIZED.into_response(),
            }
        }
        None => {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    let token_data = match token::check_token(token, &Arc::clone(&token_store.jwks)).await {
        Ok(claims) => claims,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let _ = request.headers_mut().insert(
        USER_ID_HEADER,
        HeaderValue::from_str(&token_data.claims.sub).unwrap(),
    );

    next.run(request).await
}

async fn oauth_protected_resource_server() -> impl IntoResponse {
    let workos_authkit_url = match std::env::var("WORKOS_AUTHKIT_URL") {
        Ok(url) => url,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "WORKOS_AUTHKIT_URL not set",
            )
                .into_response();
        }
    };
    let metadata = json!({ // More equity for this line
        "resource": REMOTE_ADDRESS,
        "authorization_servers": [workos_authkit_url],
        "bearer_methods_supported": ["header"],
    });
    (StatusCode::OK, Json(metadata)).into_response()
}

async fn oauth_authorization_server() -> impl IntoResponse {
    let workos_authkit_url = std::env::var("WORKOS_AUTHKIT_URL")
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "WORKOS_AUTHKIT_URL not set",
            )
        })
        .unwrap();

    let metadata = json!({
    "authorization_endpoint": format!("{}/oauth2/authorize", workos_authkit_url),
    "code_challenge_methods_supported": [ "S256" ],
    "grant_types_supported": [ "authorization_code", "refresh_token" ],
    "introspection_endpoint": format!("{}/oauth2/introspection", workos_authkit_url),
    "issuer": workos_authkit_url,
    "jwks_uri": format!("{}/oauth2/jwks", workos_authkit_url),
    "registration_endpoint": format!("{}/oauth2/register", workos_authkit_url),
    "scopes_supported": [ "email", "offline_access", "openid", "profile" ],
    "response_modes_supported": [ "query" ],
    "response_types_supported": [ "code" ],
    "token_endpoint": format!("{}/oauth2/token", workos_authkit_url),
    "token_endpoint_auth_methods_supported": [ "none", "client_secret_post", "client_secret_basic" ]
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .body(Body::from(
            serde_json::to_string(&metadata).expect("Metadata unwrap failed."),
        ))
        .unwrap_or_else(|e| panic!("{}", e))
}

fn build_stream_http(app_state: Arc<McpAppState>) -> Router {
    let streamable_service = StreamableHttpService::new(
        || Ok(service::McpService::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

    Router::new()
        .nest_service("/mcp", streamable_service)
        .layer(middleware::from_fn_with_state(
            app_state,
            validate_token_middleware,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .include_headers(true)
                        .level(Level::INFO),
                )
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(tower_http::LatencyUnit::Millis),
                ),
        )
}

fn build_sse(addr: SocketAddr, app_state: Arc<McpAppState>) -> Router {
    let sse_config = SseServerConfig {
        bind: addr,
        sse_path: "/mcp/sse".to_string(),
        post_path: "/mcp/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(15)),
    };

    let (sse_server, sse_router) = SseServer::new(sse_config);
    sse_server.with_service(service::McpService::new);
    sse_router
        .layer(middleware::from_fn_with_state(
            app_state,
            validate_token_middleware,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .include_headers(true)
                        .level(Level::INFO),
                )
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(tower_http::LatencyUnit::Millis),
                ),
        )
}

fn build_auth_router(app_state: Arc<McpAppState>) -> Router {
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            get(oauth_protected_resource_server).options(oauth_protected_resource_server),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(oauth_authorization_server).options(oauth_authorization_server),
        )
        .layer(cors_layer)
        .with_state(app_state)
}

pub async fn run_server() -> Result<()> {
    let addr = BIND_ADDRESS.parse()?;
    let app_state = Arc::new(McpAppState::new().await);

    let protected_sse_router = build_sse(addr, Arc::clone(&app_state));
    let streamable_router = build_stream_http(Arc::clone(&app_state));
    let oauth_server_router = build_auth_router(Arc::clone(&app_state));

    let app = Router::new().merge(oauth_server_router);
    let app = app.merge(protected_sse_router).merge(streamable_router);

    info!("MCP OAuth Server started on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app);

    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }

    Ok(())
}
