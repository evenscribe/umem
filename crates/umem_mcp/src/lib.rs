pub mod service;

use axum::{
    Form, Json,
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};

const BIND_ADDRESS: &str = "127.0.0.1:3000";

pub async fn run_server() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config: StreamableHttpServerConfig = StreamableHttpServerConfig {
        stateful_mode: false,
        ..Default::default()
    };

    let service = StreamableHttpService::new(
        service::McpService::new,
        LocalSessionManager::default().into(),
        config,
    );

    println!("Listening on {BIND_ADDRESS}");

    let workos_client_id = std::env::var("WORKOS_CLIENT_ID").expect("WORKOS_CLIENT_ID not set");
    let workos_client_secret =
        std::env::var("WORKOS_SECRET").expect("WORKOS_CLIENT_SECRET not set");

    let app_state = AppState {
        client_id: workos_client_id,
        client_secret: workos_client_secret,
        main_url: "https://mcp.evenscribe.com/".into(),
        issuer: String::new(),
    };

    let router = axum::Router::<AppState>::new()
        .nest_service("/mcp", service)
        .route(
            "/.well-known/oauth-authorization-server",
            get(well_known_handler),
        )
        .route("/register", post(registration_handler))
        .route("/authorize", get(authorization_handler))
        .route("/callback", get(callback_handler))
        .route("/token", post(token_handler))
        .with_state(app_state);

    let tcp_listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;
    let _ = axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
        .await;
    Ok(())
}

async fn well_known_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json(WellKnownAnswer {
        authorization_endpoint: format!("{}authorize", state.main_url),
        registration_endpoint: format!("{}register", state.main_url),
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        scopes_supported: vec![
            "email".to_string(),
            "offline_access".to_string(),
            "openid".to_string(),
            "profile".to_string(),
        ],
        response_modes_supported: vec!["query".to_string()],
        response_types_supported: vec!["code".to_string()],
        token_endpoint: format!("{}token", state.main_url),
        issuer: state.issuer,
        code_challenge_methods_supported: vec!["S256".to_string()],
    })
}

async fn registration_handler(
    State(state): State<AppState>,
    Json(req): Json<ClientRegistrationRequest>,
) -> impl IntoResponse {
    println!("{:?}", req);
    Json(ClientRegistrationAnswer {
        client_id: state.client_id,
        redirect_uris: req.redirect_uris,
        response_types: req.response_types,
        grant_types: req.grant_types,
        client_name: req.client_name,
    })
}

async fn authorization_handler(
    State(app_state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let original_uri = params.get("redirect_uri").unwrap();

    let state = BASE64_STANDARD.encode(original_uri);

    let local_redirect = format!("{}callback", app_state.main_url);

    let url = reqwest::Url::parse_with_params(
        // FIX
        "https://api.workos.com/user_management/authorize",
        &[
            ("response_type", "code"),
            ("client_id", app_state.client_id.as_str()),
            ("redirect_uri", local_redirect.as_str()),
            ("code_challenge", params.get("code_challenge").unwrap()),
            ("code_challenge_method", "S256"),
            ("provider", "authkit"),
            ("state", state.as_str()),
            ("scope", "openid profile email offline_access"),
        ],
    )
    .unwrap();

    println!("redirecting to: {}", &url.as_str());

    Redirect::temporary(url.as_str())
}

async fn callback_handler(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let code = params.get("code").unwrap();

    let state = params.get("state").unwrap();

    let original_redirect_uri = String::from_utf8(BASE64_STANDARD.decode(state).unwrap()).unwrap();

    println!("original_redirect_uri: {}", original_redirect_uri);

    let response_url = reqwest::Url::parse_with_params(
        original_redirect_uri.as_str(),
        &[("code", &code), ("state", &state)],
    )
    .unwrap();

    Redirect::temporary(response_url.as_str())
}

async fn token_handler(
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let grant_type = form.get("grant_type").unwrap();

    let client = reqwest::Client::new();

    let request = match grant_type.as_str() {
        "authorization_code" => {
            let code = form.get("code").unwrap();
            let code_verifier = form.get("code_verifier").unwrap();
            let client_id = form.get("client_id").unwrap();

            json!({
                "client_id": client_id,
                "client_secret": state.client_secret,
                "grant_type": "authorization_code",
                "code": code,
                "code_verifier": code_verifier
            })
        }
        "refresh_token" => {
            let refresh_token = form.get("refresh_token").unwrap();
            let client_id = form.get("client_id").unwrap();

            json!({
                "client_id": client_id,
                "client_secret": state.client_secret,
                "grant_type": "refresh_token",
                "refresh_token": refresh_token,
            })
        }
        _ => panic!("Invalid grant type"),
    };

    println!("token request: {}", request);

    let res = client
        .post("https://api.workos.com/user_management/authenticate")
        .body(serde_json::to_string(&request).unwrap())
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();

    let authkit_response: AuthkitAuthResult =
        serde_json::from_str(res.text().await.unwrap().as_str()).unwrap();

    println!("token response {:?}", &authkit_response);

    let expires_at = chrono::Utc::now().timestamp() + 3600;

    let token_result = TokenResponse {
        access_token: authkit_response.access_token,
        refresh_token: authkit_response.refresh_token,
        token_type: "Bearer".to_string(),
        expires_at,
    };

    Json(token_result)
}

#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub token_endpoint_auth_method: String,
    pub response_types: Vec<String>,
}

#[derive(Serialize)]
pub struct ClientRegistrationAnswer {
    client_id: String,
    redirect_uris: Vec<String>,
    response_types: Vec<String>,
    grant_types: Vec<String>,
    client_name: String,
}

#[derive(serde::Serialize)]
pub struct WellKnownAnswer {
    pub authorization_endpoint: String,
    pub registration_endpoint: String,
    pub grant_types_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub response_modes_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub token_endpoint: String,
    pub issuer: String,
    pub code_challenge_methods_supported: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expires_at: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthkitAuthResult {
    access_token: String,
    refresh_token: String,
}

#[derive(Clone)]
pub struct AppState {
    pub client_id: String,
    pub client_secret: String,
    pub main_url: String,
    pub issuer: String,
}
