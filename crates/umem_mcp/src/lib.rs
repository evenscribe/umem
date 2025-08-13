pub mod service;
mod token;

use anyhow::Result;
use axum::{
    Json, Router,
    body::Body,
    extract::{Form, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use rmcp::transport::{SseServer, sse_server::SseServerConfig};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const BIND_ADDRESS: &str = "127.0.0.1:3000";
const REMOTE_ADDRESS: &str = "https://m.evenscribe.com";
const INDEX_HTML: &str = include_str!("../templates/mcp_oauth_index.html");
const ISSUER_WORKOS: &str = "https://api.workos.com";

#[derive(Clone, Debug)]
struct McpOAuthStore {
    jwks: Arc<token::JWKS>,
    workos_client_id: String,
    workos_client_secret: String,
}

impl McpOAuthStore {
    async fn new() -> Self {
        let workos_client_id: String = std::env::var("WORKOS_CLIENT_ID").unwrap();
        let workos_client_secret: String = std::env::var("WORKOS_CLIENT_SECRET").unwrap();
        let jwks_url = std::env::var("JWKS_URL").expect("JWKS_URL must be set");
        let jwks = token::get_jwks(jwks_url).await;

        Self {
            jwks: Arc::new(jwks),
            workos_client_id,
            workos_client_secret,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AuthToken {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: String,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorizeQuery {
    #[allow(dead_code)]
    response_type: String,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    client_id: String,
    redirect_uri: String,
    scope: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenRequest {
    grant_type: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    client_secret: String,
    #[serde(default)]
    redirect_uri: String,
    #[serde(default)]
    code_verifier: Option<String>,
    #[serde(default)]
    refresh_token: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct UserInfo {
    sub: String,
    name: String,
    email: String,
    username: String,
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkOsState {
    client_id: String,
    original_state: String,
    scopes: String,
    original_redirect_uri: String,
}

async fn oauth_authorize(
    Query(params): Query<AuthorizeQuery>,
    State(state): State<Arc<McpOAuthStore>>,
) -> impl IntoResponse {
    debug!("doing oauth_authorize");

    let local_state = WorkOsState {
        client_id: params.client_id.clone(),
        original_state: params.state.unwrap_or_default(),
        scopes: params.scope.unwrap_or_default(),
        original_redirect_uri: params.redirect_uri.clone(),
    };

    let url = reqwest::Url::parse_with_params(
        "https://api.workos.com/user_management/authorize",
        &[
            ("response_type", "code"),
            ("client_id", &state.workos_client_id),
            (
                "redirect_uri",
                format!("{}/mcp/callback", REMOTE_ADDRESS).as_str(),
            ),
            (
                "code_challenge",
                params.code_challenge.unwrap_or(String::new()).as_str(),
            ),
            (
                "code_challenge_method",
                params
                    .code_challenge_method
                    .unwrap_or("S256".to_string())
                    .as_str(),
            ),
            ("provider", "authkit"),
            (
                "state",
                serde_json::to_string(&local_state).unwrap().as_str(),
            ),
            ("scope", "openid profile email offline_access"),
        ],
    )
    .unwrap();

    Redirect::temporary(url.as_str()).into_response()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthkitAuthResult {
    access_token: String,
    refresh_token: String,
}

async fn oauth_token(
    State(state): State<Arc<McpOAuthStore>>,
    Form(form_state): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let grant_type = form_state.get("grant_type").cloned().unwrap_or_default();
    let client = reqwest::Client::new();

    let request = match grant_type.as_str() {
        "authorization_code" => {
            let code = form_state.get("code").unwrap();
            let code_verifier = form_state.get("code_verifier").unwrap();
            json!({
                "client_id": state.workos_client_id,
                "client_secret": state.workos_client_secret,
                "grant_type": "authorization_code",
                "code": code,
                "code_verifier": code_verifier
            })
        }
        "refresh_token" => {
            let refresh_token = form_state.get("refresh_token").unwrap();
            json!({
                "client_id": state.workos_client_id,
                "client_secret": state.workos_client_secret,
                "grant_type": "refresh_token",
                "refresh_token": refresh_token,
            })
        }
        _ => panic!("Invalid grant type"),
    };

    let response = client
        .post("https://api.workos.com/user_management/authenticate")
        .body(serde_json::to_string(&request).unwrap())
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();
    let authkit_response: AuthkitAuthResult =
        serde_json::from_str(response.text().await.unwrap().as_str()).unwrap();
    let expires_at = chrono::Utc::now().timestamp() + 3600;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "access_token": authkit_response.access_token,
            "token_type": "Bearer".to_string(),
            "expires_in": expires_at,
            "expires_at": expires_at,
            "refresh_token": authkit_response.refresh_token,
        })),
    )
        .into_response()
}

async fn validate_token_middleware(
    State(token_store): State<Arc<McpOAuthStore>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    debug!("validate_token_middleware");
    let auth_header = request.headers().get("Authorization");
    let token = match auth_header {
        Some(header) => {
            let header_str = header.to_str().unwrap_or("");
            if let Some(stripped) = header_str.strip_prefix("Bearer ") {
                stripped.to_string()
            } else {
                return StatusCode::UNAUTHORIZED.into_response();
            }
        }
        None => {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    println!("{:?}", &token_store.jwks);
    let claims = token::check_token(token.as_str(), &Arc::clone(&token_store.jwks)).await;
    match claims {
        Ok(tk) => {
            println!("Claims: {:?}", tk);
            next.run(request).await
        }
        Err(e) => {
            println!("Error: {:?}", e);
            StatusCode::UNAUTHORIZED.into_response()
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtectedResourceMetadata {
    pub authorization_servers: Vec<ProtectedResourceInner>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtectedResourceInner {
    pub issuer: String,
    pub authorization_endpoint: String,
}

async fn oauth_protected_resource_server() -> impl IntoResponse {
    let workos_authkit_url = std::env::var("WORKOS_AUTHKIT_URL")
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "WORKOS_AUTHKIT_URL not set",
            )
        })
        .unwrap();

    let metadata = json!({
        "resource": REMOTE_ADDRESS,
        "authorization_servers": [workos_authkit_url],
        "bearer_methods_supported": ["header"],
    });

    debug!("metadata: {:?}", metadata);
    (StatusCode::OK, Json(metadata))
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
    debug!("metadata: {:?}", metadata);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .body(Body::from(serde_json::to_string(&metadata).unwrap()))
        .unwrap()
}

async fn log_request(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();

    let headers = request.headers().clone();
    let mut header_log = String::new();
    for (key, value) in headers.iter() {
        let value_str = value.to_str().unwrap_or("<binary>");
        header_log.push_str(&format!("\n  {}: {}", key, value_str));
    }

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let request_info = if content_type.contains("application/x-www-form-urlencoded")
        || content_type.contains("application/json")
    {
        format!(
            "{} {} {:?}{}\nContent-Type: {}",
            method, uri, version, header_log, content_type
        )
    } else {
        format!("{} {} {:?}{}", method, uri, version, header_log)
    };

    info!("REQUEST: {}", request_info);

    let response = next.run(request).await;

    let status = response.status();
    info!("RESPONSE: {} for {} {}", status, method, uri);

    response
}

#[axum::debug_handler]
async fn workos_callback(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let code = params.get("code").cloned().unwrap_or_default();
    let state = params.get("state").cloned().unwrap_or_default();
    let decoded_state = serde_json::from_str::<WorkOsState>(&state).unwrap();

    let response_url = reqwest::Url::parse_with_params(
        &decoded_state.original_redirect_uri,
        &[("code", &code), ("state", &decoded_state.original_state)],
    )
    .unwrap();

    Redirect::temporary(response_url.as_str()).into_response()
}

pub async fn run_server() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = McpOAuthStore::new().await;
    let oauth_store = Arc::new(state);
    let addr = BIND_ADDRESS.parse::<SocketAddr>()?;
    let sse_config = SseServerConfig {
        bind: addr,
        sse_path: "/mcp/sse".to_string(),
        post_path: "/mcp/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(15)),
    };

    let (sse_server, sse_router) = SseServer::new(sse_config);
    let protected_sse_router = sse_router.layer(middleware::from_fn_with_state(
        oauth_store.clone(),
        validate_token_middleware,
    ));
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let oauth_server_router = Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            get(oauth_protected_resource_server).options(oauth_protected_resource_server),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(oauth_authorization_server).options(oauth_authorization_server),
        )
        .route("/oauth/token", post(oauth_token).options(oauth_token))
        .route("/mcp/callback", get(workos_callback))
        .layer(cors_layer)
        .with_state(oauth_store.clone());

    let app = Router::new()
        .route("/", get(index))
        .route("/mcp", get(index))
        .route("/oauth/authorize", get(oauth_authorize))
        .merge(oauth_server_router)
        .with_state(oauth_store.clone())
        .layer(middleware::from_fn(log_request));

    let app = app.merge(protected_sse_router);
    let cancel_token = sse_server.config.ct.clone();
    let cancel_token2 = sse_server.config.ct.clone();
    sse_server.with_service(service::McpService::new);

    info!("MCP OAuth Server started on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        cancel_token.cancelled().await;
        info!("Server is shutting down");
    });

    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received Ctrl+C, shutting down");
                cancel_token2.cancel();
            }
            Err(e) => error!("Failed to listen for Ctrl+C: {}", e),
        }
    });

    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }

    Ok(())
}
