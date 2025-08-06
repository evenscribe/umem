pub mod service;
mod token;

use anyhow::Result;
use askama::Template;
use axum::{
    Json, Router,
    body::Body,
    extract::{Form, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use rand::{Rng, distr::Alphanumeric};
use rmcp::transport::{
    SseServer,
    auth::{
        AuthorizationMetadata, ClientRegistrationRequest, ClientRegistrationResponse,
        OAuthClientConfig,
    },
    sse_server::SseServerConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

// Import Counter tool for MCP service

const BIND_ADDRESS: &str = "127.0.0.1:3000";
const NGROK_ADDRESS: &str = "https://mccp.evenscribe.com";
const INDEX_HTML: &str = include_str!("../templates/mcp_oauth_index.html");

// A easy way to manage MCP OAuth Store for managing tokens and sessions
#[derive(Clone, Debug)]
struct McpOAuthStore {
    clients: Arc<RwLock<HashMap<String, OAuthClientConfig>>>,
    auth_sessions: Arc<RwLock<HashMap<String, AuthSession>>>,
    access_tokens: Arc<RwLock<HashMap<String, McpAccessToken>>>,
    jwks: Arc<token::JWKS>,
    workos_client_id: String,
    workos_client_secret: String,
}

impl McpOAuthStore {
    async fn new() -> Self {
        let mut clients = HashMap::new();
        clients.insert(
            "mcp-client".to_string(),
            OAuthClientConfig {
                client_id: "mcp-client".to_string(),
                client_secret: Some("mcp-client-secret".to_string()),
                scopes: vec!["profile".to_string(), "email".to_string()],
                redirect_uri: "http://localhost:8080/callback".to_string(),
            },
        );

        let workos_client_id: String = std::env::var("WORKOS_CLIENT_ID").unwrap();
        let workos_client_secret: String = std::env::var("WORKOS_CLIENT_SECRET").unwrap();

        let jwks_url = std::env::var("JWKS_URL").expect("JWKS_URL must be set");
        let jwks = token::get_jwks(jwks_url).await;

        Self {
            clients: Arc::new(RwLock::new(clients)),
            auth_sessions: Arc::new(RwLock::new(HashMap::new())),
            access_tokens: Arc::new(RwLock::new(HashMap::new())),
            jwks: Arc::new(jwks),
            workos_client_id,
            workos_client_secret,
        }
    }

    async fn validate_client(
        &self,
        client_id: &str,
        redirect_uri: &str,
    ) -> Option<OAuthClientConfig> {
        let clients = self.clients.read().await;
        println!("Clients: {:#?}", clients);

        if let Some(client) = clients.get(client_id) {
            if client.redirect_uri.contains(&redirect_uri.to_string()) {
                return Some(client.clone());
            }
        }
        None
    }

    async fn create_auth_session(
        &self,
        client_id: String,
        scope: Option<String>,
        state: Option<String>,
        session_id: String,
    ) -> String {
        let session = AuthSession {
            client_id,
            scope,
            _state: state,
            _created_at: chrono::Utc::now(),
            auth_token: None,
        };

        self.auth_sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        session_id
    }

    async fn update_auth_session_token(
        &self,
        session_id: &str,
        token: AuthToken,
    ) -> Result<(), String> {
        let mut sessions = self.auth_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.auth_token = Some(token);
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }
}

// a simple session record for auth session
#[derive(Clone, Debug)]
struct AuthSession {
    client_id: String,
    scope: Option<String>,
    _state: Option<String>,
    _created_at: chrono::DateTime<chrono::Utc>,
    auth_token: Option<AuthToken>,
}

// a simple token record for auth token
// not used oauth2 token for avoid include oauth2 crate in this example
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AuthToken {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: String,
    scope: Option<String>,
}

// a simple token record for mcp token ,
// not used oauth2 token for avoid include oauth2 crate in this example
#[derive(Clone, Debug, Serialize)]
struct McpAccessToken {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: String,
    scope: Option<String>,
    auth_token: AuthToken,
    client_id: String,
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

fn generate_random_string(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

// Root path handler
async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

#[derive(Template)]
#[template(path = "mcp_oauth_authorize.html")]
struct OAuthAuthorizeTemplate {
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
    scopes: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkOsState {
    client_id: String,
    original_state: String,
    scopes: String,
    original_redirect_uri: String,
}

// Initial OAuth authorize endpoint
async fn oauth_authorize(
    Query(params): Query<AuthorizeQuery>,
    State(state): State<Arc<McpOAuthStore>>,
) -> impl IntoResponse {
    debug!("doing oauth_authorize");
    // match state
    //     .validate_client(&params.client_id, &params.redirect_uri)
    //     .await
    // {
    //     Some(_) => {

    // let local_state = vec![
    //     ("client_id", params.client_id.clone()),
    //     ("original_state", params.state.unwrap_or(String::new())),
    //     ("scopes", params.scope.unwrap_or(String::new())),
    //     ("original_redirect_uri", params.redirect_uri.clone()),
    // ];

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
            ("redirect_uri", "https://mccp.evenscribe.com/mcp/callback"),
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
    // }
    // None => (
    //     StatusCode::BAD_REQUEST,
    //     Json(serde_json::json!({
    //         "error": "invalid_request",
    //         "error_description": "invalid client id or redirect uri"
    //     })),
    // )
    //     .into_response(),
    // }
}

// handle approval of authorization
#[derive(Debug, Deserialize)]
struct ApprovalForm {
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
    approved: String,
}

async fn oauth_approve(
    State(state): State<Arc<McpOAuthStore>>,
    Form(form): Form<ApprovalForm>,
) -> impl IntoResponse {
    if form.approved != "true" {
        // user rejected the authorization request
        let redirect_url = format!(
            "{}?error=access_denied&error_description={}{}",
            form.redirect_uri,
            "user rejected the authorization request",
            if form.state.is_empty() {
                "".to_string()
            } else {
                format!("&state={}", form.state)
            }
        );
        return Redirect::to(&redirect_url).into_response();
    }

    // user approved the authorization request, generate authorization code
    let session_id = Uuid::new_v4().to_string();
    let auth_code = format!("mcp-code-{}", session_id);

    // create new session record authorization information
    let session_id = state
        .create_auth_session(
            form.client_id.clone(),
            Some(form.scope.clone()),
            Some(form.state.clone()),
            session_id.clone(),
        )
        .await;

    // create token
    let created_token = AuthToken {
        access_token: format!("tp-token-{}", Uuid::new_v4()),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: format!("tp-refresh-{}", Uuid::new_v4()),
        scope: Some(form.scope),
    };

    // update session token
    if let Err(e) = state
        .update_auth_session_token(&session_id, created_token)
        .await
    {
        error!("Failed to update session token: {}", e);
    }

    // redirect back to client, with authorization code
    let redirect_url = format!(
        "{}?code={}{}",
        form.redirect_uri,
        auth_code,
        if form.state.is_empty() {
            "".to_string()
        } else {
            format!("&state={}", form.state)
        }
    );

    info!("authorization approved, redirecting to: {}", redirect_url);
    Redirect::to(&redirect_url).into_response()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthkitAuthResult {
    access_token: String,
    refresh_token: String,
}

// Handle token request from the MCP client
async fn oauth_token(
    State(state): State<Arc<McpOAuthStore>>,
    Form(form_state): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    info!("Received token request");

    let grant_type = form_state.get("grant_type").cloned().unwrap_or_default();

    let client = reqwest::Client::new();

    let request = match grant_type.as_str() {
        "authorization_code" => {
            let code = form_state.get("code").unwrap();
            let code_verifier = form_state.get("code_verifier").unwrap();
            // let client_id = form_state.get("client_id").unwrap();
            // let client_secret = form_state.get("client_secret").unwrap();

            json!({
                "client_id": "client_01K1HS6DV6AVDJKYSVSD6XRZQN",
                "client_secret": state.workos_client_secret,
                "grant_type": "authorization_code",
                "code": code,
                "code_verifier": code_verifier
            })
        }
        "refresh_token" => {
            let refresh_token = form_state.get("refresh_token").unwrap();
            // let client_id = form_state.get("client_id").unwrap();
            // let client_secret = form_state.get("client_secret").unwrap();

            json!({
                "client_id": "client_01K1HS6DV6AVDJKYSVSD6XRZQN",
                "client_secret": state.workos_client_secret,
                "grant_type": "refresh_token",
                "refresh_token": refresh_token,
            })
        }
        _ => panic!("Invalid grant type"),
    };

    let res = client
        .post("https://api.workos.com/user_management/authenticate")
        .body(serde_json::to_string(&request).unwrap())
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();

    let authkit_response: AuthkitAuthResult =
        serde_json::from_str(res.text().await.unwrap().as_str()).unwrap();

    let expires_at = chrono::Utc::now().timestamp() + 3600;

    info!("successfully created access token");
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

// Auth middleware for SSE connections
async fn validate_token_middleware(
    State(token_store): State<Arc<McpOAuthStore>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    debug!("validate_token_middleware");
    // Extract the access token from the Authorization header
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

/// oauth2 metadata
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
    let metadata = ProtectedResourceMetadata {
        authorization_servers: vec![ProtectedResourceInner {
            authorization_endpoint: format!("{}/oauth/authorize", NGROK_ADDRESS),
            issuer: "https://api.workos.com".to_string(),
        }],
    };
    debug!("metadata: {:?}", metadata);
    (StatusCode::OK, Json(metadata))
}

// handle oauth server metadata request
async fn oauth_authorization_server() -> impl IntoResponse {
    let mut additional_fields = HashMap::new();
    additional_fields.insert(
        "response_types_supported".into(),
        Value::Array(vec![Value::String("code".into())]),
    );
    additional_fields.insert(
        "code_challenge_methods_supported".into(),
        Value::Array(vec![Value::String("S256".into())]),
    );
    let metadata = AuthorizationMetadata {
        authorization_endpoint: format!("{}/oauth/authorize", NGROK_ADDRESS),
        token_endpoint: format!("{}/oauth/token", NGROK_ADDRESS),
        scopes_supported: Some(vec!["profile".to_string(), "email".to_string()]),
        registration_endpoint: format!("{}/oauth/register", NGROK_ADDRESS),
        issuer: Some("https://api.workos.com".to_string()),
        jwks_uri: Some(
            "https://api.workos.com/sso/jwks/client_01K1HS6DV6AVDJKYSVSD6XRZQN".to_string(),
        ),
        additional_fields,
    };
    debug!("metadata: {:?}", metadata);
    (StatusCode::OK, Json(metadata))
}

// handle client registration request
async fn oauth_register(
    State(state): State<Arc<McpOAuthStore>>,
    Json(req): Json<ClientRegistrationRequest>,
) -> impl IntoResponse {
    debug!("register request: {:?}", req);
    if req.redirect_uris.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_request",
                "error_description": "at least one redirect uri is required"
            })),
        )
            .into_response();
    }

    // generate client id and secret
    let client_id = format!("client-{}", Uuid::new_v4());
    let client_secret = generate_random_string(32);

    let client = OAuthClientConfig {
        client_id: client_id.clone(),
        client_secret: Some(client_secret.clone()),
        redirect_uri: req.redirect_uris[0].clone(),
        scopes: vec![],
    };

    state
        .clients
        .write()
        .await
        .insert(client_id.clone(), client);

    // return client information
    let response = ClientRegistrationResponse {
        client_id,
        client_secret: Some(client_secret),
        client_name: req.client_name,
        redirect_uris: req.redirect_uris,
        additional_fields: HashMap::new(),
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

// Log all HTTP requests
async fn log_request(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();

    // Log headers
    let headers = request.headers().clone();
    let mut header_log = String::new();
    for (key, value) in headers.iter() {
        let value_str = value.to_str().unwrap_or("<binary>");
        header_log.push_str(&format!("\n  {}: {}", key, value_str));
    }

    // Try to get request body for form submissions
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

    // Call the actual handler
    let response = next.run(request).await;

    // Log response status
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
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create the OAuth store
    let state = McpOAuthStore::new().await;
    dbg!(&state);
    let oauth_store = Arc::new(state);

    // Set up port
    let addr = BIND_ADDRESS.parse::<SocketAddr>()?;

    // Create SSE server configuration for MCP
    let sse_config = SseServerConfig {
        bind: addr,
        sse_path: "/mcp/sse".to_string(),
        post_path: "/mcp/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(15)),
    };

    // Create SSE server
    let (sse_server, sse_router) = SseServer::new(sse_config);

    // Create protected SSE routes (require authorization)
    let protected_sse_router = sse_router.layer(middleware::from_fn_with_state(
        oauth_store.clone(),
        validate_token_middleware,
    ));

    // Create CORS layer for the oauth authorization server endpoint
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create a sub-router for the oauth authorization server endpoint with CORS
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
        .route(
            "/oauth/register",
            post(oauth_register).options(oauth_register),
        )
        .route("/mcp/callback", get(workos_callback))
        .layer(cors_layer)
        .with_state(oauth_store.clone());

    // Create HTTP router with request logging middleware
    let app = Router::new()
        .route("/", get(index))
        .route("/mcp", get(index))
        .route("/oauth/authorize", get(oauth_authorize))
        .route("/oauth/approve", post(oauth_approve))
        // .merge(protected_sse_router)
        .merge(oauth_server_router) // Merge the CORS-enabled oauth server router
        .with_state(oauth_store.clone())
        .layer(middleware::from_fn(log_request));

    let app = app.merge(protected_sse_router);
    // Register token validation middleware for SSE
    let cancel_token = sse_server.config.ct.clone();
    // Handle Ctrl+C
    let cancel_token2 = sse_server.config.ct.clone();
    // Start SSE server with Counter service
    sse_server.with_service(service::McpService::new);

    // Start HTTP server
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
