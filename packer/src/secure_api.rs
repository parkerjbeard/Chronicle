use crate::auth::{auth_middleware, login_handler, logout_handler, AuthService, Claims};
use crate::tls::{CertificateManager, TlsConfig};
use anyhow::{anyhow, Result};
use axum::{
    extract::{Query, Request, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::Json,
    routing::{get, post, put},
    Extension, Router,
};
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use nonzero_ext::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio_rustls::TlsAcceptor;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{error, info, warn};

/// Rate limiter type for API endpoints
type ApiRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub rate_limiter: Arc<ApiRateLimiter>,
    pub config: Arc<SecurityConfig>,
}

/// Security configuration for the API server
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub max_request_size: usize,
    pub request_timeout: Duration,
    pub rate_limit_per_minute: u32,
    pub enable_cors: bool,
    pub allowed_origins: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_request_size: 10 * 1024 * 1024, // 10MB
            request_timeout: Duration::from_secs(30),
            rate_limit_per_minute: 60, // 60 requests per minute
            enable_cors: false, // Disabled by default for security
            allowed_origins: vec!["https://localhost:8443".to_string()],
        }
    }
}

/// Secure API server implementation
pub struct SecureApiServer {
    app_state: AppState,
    tls_config: TlsConfig,
    bind_addr: SocketAddr,
}

impl SecureApiServer {
    pub fn new(
        auth_service: Arc<AuthService>,
        tls_config: TlsConfig,
        bind_addr: SocketAddr,
        security_config: SecurityConfig,
    ) -> Self {
        // Create rate limiter
        let quota = Quota::per_minute(nonzero!(security_config.rate_limit_per_minute));
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        let app_state = AppState {
            auth_service,
            rate_limiter,
            config: Arc::new(security_config),
        };

        Self {
            app_state,
            tls_config,
            bind_addr,
        }
    }

    /// Start the secure HTTPS API server
    pub async fn start(&self) -> Result<()> {
        let app = self.create_router();

        let tls_acceptor = TlsAcceptor::from(self.tls_config.server_config.clone());

        info!("Starting secure API server on https://{}", self.bind_addr);
        info!("TLS certificate: {}", self.tls_config.cert_path.display());

        // Create TCP listener
        let listener = tokio::net::TcpListener::bind(self.bind_addr).await?;

        loop {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let tls_acceptor = tls_acceptor.clone();
                    let app = app.clone();

                    tokio::spawn(async move {
                        match tls_acceptor.accept(stream).await {
                            Ok(tls_stream) => {
                                let hyper_service = hyper::service::service_fn(move |request| {
                                    app.clone().call(request)
                                });

                                if let Err(e) = hyper::server::conn::Http::new()
                                    .serve_connection(tls_stream, hyper_service)
                                    .await
                                {
                                    error!("Failed to serve connection from {}: {}", remote_addr, e);
                                }
                            }
                            Err(e) => {
                                error!("TLS handshake failed for {}: {}", remote_addr, e);
                            }
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Create the router with all security middleware and endpoints
    fn create_router(&self) -> Router {
        let protected_routes = Router::new()
            .route("/api/status/system", get(get_system_status))
            .route("/api/status/permissions", get(get_permission_status))
            .route("/api/collectors", get(get_collectors))
            .route("/api/collectors/:id/toggle", post(toggle_collector))
            .route("/api/backup/status", get(get_backup_status))
            .route("/api/backup/start", post(start_backup))
            .route("/api/ring-buffer/stats", get(get_ring_buffer_stats))
            .route("/api/search", post(search_events))
            .route("/api/config", get(get_configuration))
            .route("/api/config", put(update_configuration))
            .route("/api/export", post(export_data))
            .route("/api/export/:id", get(get_export_status))
            .route("/api/export/:id/download", get(download_export))
            .route("/api/wipe", post(wipe_database))
            .layer(middleware::from_fn_with_state(
                self.app_state.auth_service.clone(),
                auth_middleware,
            ));

        let public_routes = Router::new()
            .route("/api/auth/login", post(login_handler))
            .route("/api/auth/logout", post(logout_handler))
            .route("/api/health", get(health_check))
            .route("/api/ping", get(ping));

        Router::new()
            .merge(protected_routes)
            .merge(public_routes)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(TimeoutLayer::new(self.app_state.config.request_timeout))
                    .layer(RequestBodyLimitLayer::new(self.app_state.config.max_request_size))
                    .layer(middleware::from_fn_with_state(
                        self.app_state.rate_limiter.clone(),
                        rate_limit_middleware,
                    ))
                    .layer(self.create_cors_layer()),
            )
            .with_state(self.app_state.clone())
    }

    /// Create CORS layer based on configuration
    fn create_cors_layer(&self) -> CorsLayer {
        if self.app_state.config.enable_cors {
            CorsLayer::new()
                .allow_origin(
                    self.app_state
                        .config
                        .allowed_origins
                        .iter()
                        .map(|origin| origin.parse().unwrap())
                        .collect::<Vec<_>>(),
                )
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                ])
        } else {
            CorsLayer::new().allow_origin(tower_http::cors::AllowOrigin::exact(
                "https://localhost:8443".parse().unwrap(),
            ))
        }
    }
}

/// Rate limiting middleware
async fn rate_limit_middleware(
    State(rate_limiter): State<Arc<ApiRateLimiter>>,
    request: Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    match rate_limiter.check() {
        Ok(_) => Ok(next.run(request).await),
        Err(_) => {
            warn!("Rate limit exceeded for request");
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
}

/// Health check endpoint (public)
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "version": env!("CARGO_PKG_VERSION"),
        "service": "chronicle-packer"
    }))
}

/// Ping endpoint (public)
async fn ping() -> &'static str {
    "pong"
}

/// Input validation for search queries
#[derive(Deserialize)]
struct SearchRequest {
    #[serde(deserialize_with = "validate_search_query")]
    query: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

fn validate_search_query<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let query = String::deserialize(deserializer)?;
    
    // Basic input validation
    if query.len() > 1000 {
        return Err(serde::de::Error::custom("Search query too long"));
    }
    
    // Check for potential injection patterns
    let dangerous_patterns = ["<script", "javascript:", "data:", "../", "\\x"];
    for pattern in &dangerous_patterns {
        if query.to_lowercase().contains(pattern) {
            return Err(serde::de::Error::custom("Invalid characters in search query"));
        }
    }
    
    Ok(query)
}

/// System status endpoint
async fn get_system_status(Extension(claims): Extension<Claims>) -> Result<Json<SystemStatus>, StatusCode> {
    // Verify user has read permissions
    if !claims.scope.contains(&"read".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let status = SystemStatus {
        uptime: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        memory_usage: get_memory_usage(),
        storage_usage: get_storage_usage(),
        active_collectors: get_active_collectors_count(),
        last_event_time: get_last_event_time(),
    };

    Ok(Json(status))
}

/// Permission status endpoint
async fn get_permission_status(Extension(claims): Extension<Claims>) -> Result<Json<PermissionStatus>, StatusCode> {
    if !claims.scope.contains(&"read".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let status = PermissionStatus {
        screen_recording: check_screen_recording_permission(),
        input_monitoring: check_input_monitoring_permission(),
        accessibility: check_accessibility_permission(),
        full_disk_access: check_full_disk_access_permission(),
    };

    Ok(Json(status))
}

/// Get collectors endpoint
async fn get_collectors(Extension(claims): Extension<Claims>) -> Result<Json<Vec<CollectorInfo>>, StatusCode> {
    if !claims.scope.contains(&"read".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let collectors = get_all_collectors();
    Ok(Json(collectors))
}

/// Toggle collector endpoint
async fn toggle_collector(
    axum::extract::Path(collector_id): axum::extract::Path<String>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<ToggleCollectorRequest>,
) -> Result<StatusCode, StatusCode> {
    if !claims.scope.contains(&"write".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate collector ID
    if collector_id.len() > 50 || !collector_id.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(StatusCode::BAD_REQUEST);
    }

    match toggle_collector_impl(&collector_id, request.enabled) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Search events endpoint with comprehensive input validation
async fn search_events(
    Extension(claims): Extension<Claims>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResults>, StatusCode> {
    if !claims.scope.contains(&"read".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Additional validation
    let limit = request.limit.unwrap_or(100).min(1000); // Cap at 1000 results
    let offset = request.offset.unwrap_or(0);

    match search_events_impl(&request.query, limit, offset) {
        Ok(results) => Ok(Json(results)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Wipe database endpoint with enhanced security
async fn wipe_database(
    Extension(claims): Extension<Claims>,
    Json(request): Json<WipeRequest>,
) -> Result<StatusCode, StatusCode> {
    if !claims.scope.contains(&"admin".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Enhanced wipe validation - require additional confirmation
    if request.confirmation_phrase != "I understand this will permanently delete all data" {
        return Err(StatusCode::BAD_REQUEST);
    }

    if !request.additional_confirmation {
        return Err(StatusCode::BAD_REQUEST);
    }

    match wipe_database_impl(request.secure_delete) {
        Ok(_) => {
            info!("Database wipe completed by user {}", claims.sub);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Supporting types and implementations

#[derive(Serialize)]
struct SystemStatus {
    uptime: u64,
    memory_usage: u64,
    storage_usage: u64,
    active_collectors: u32,
    last_event_time: Option<SystemTime>,
}

#[derive(Serialize)]
struct PermissionStatus {
    screen_recording: bool,
    input_monitoring: bool,
    accessibility: bool,
    full_disk_access: bool,
}

#[derive(Serialize)]
struct CollectorInfo {
    id: String,
    name: String,
    enabled: bool,
    status: String,
    last_event: Option<SystemTime>,
}

#[derive(Deserialize)]
struct ToggleCollectorRequest {
    enabled: bool,
}

#[derive(Serialize)]
struct SearchResults {
    events: Vec<serde_json::Value>,
    total_count: usize,
    query_time_ms: u64,
}

#[derive(Deserialize)]
struct WipeRequest {
    confirmation_phrase: String,
    additional_confirmation: bool,
    secure_delete: bool,
}

// Placeholder implementations - these would connect to actual Chronicle services
fn get_memory_usage() -> u64 { 0 }
fn get_storage_usage() -> u64 { 0 }
fn get_active_collectors_count() -> u32 { 0 }
fn get_last_event_time() -> Option<SystemTime> { None }
fn check_screen_recording_permission() -> bool { false }
fn check_input_monitoring_permission() -> bool { false }
fn check_accessibility_permission() -> bool { false }
fn check_full_disk_access_permission() -> bool { false }
fn get_all_collectors() -> Vec<CollectorInfo> { vec![] }
fn toggle_collector_impl(_id: &str, _enabled: bool) -> Result<()> { Ok(()) }
fn search_events_impl(_query: &str, _limit: usize, _offset: usize) -> Result<SearchResults> {
    Ok(SearchResults {
        events: vec![],
        total_count: 0,
        query_time_ms: 0,
    })
}
fn wipe_database_impl(_secure_delete: bool) -> Result<()> { Ok(()) }

// Additional endpoints would be implemented here following the same security patterns...
async fn get_backup_status(Extension(_claims): Extension<Claims>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "not_implemented"}))
}

async fn start_backup(Extension(_claims): Extension<Claims>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn get_ring_buffer_stats(Extension(_claims): Extension<Claims>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "not_implemented"}))
}

async fn get_configuration(Extension(_claims): Extension<Claims>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "not_implemented"}))
}

async fn update_configuration(
    Extension(_claims): Extension<Claims>,
    _body: String,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn export_data(Extension(_claims): Extension<Claims>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "not_implemented"}))
}

async fn get_export_status(
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "not_implemented"}))
}

async fn download_export(
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_secure_api_server_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cert_manager = CertificateManager::new(temp_dir.path().to_path_buf());
        let tls_config = cert_manager.get_or_create_tls_config().unwrap();
        let auth_service = Arc::new(AuthService::new("test_password".to_string()).unwrap());
        
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8443);
        let security_config = SecurityConfig::default();
        
        let _server = SecureApiServer::new(auth_service, tls_config, bind_addr, security_config);
    }

    #[test]
    fn test_search_query_validation() {
        // Valid query
        let valid_json = r#"{"query": "test search"}"#;
        let request: Result<SearchRequest, _> = serde_json::from_str(valid_json);
        assert!(request.is_ok());

        // Invalid query with script tag
        let invalid_json = r#"{"query": "<script>alert('xss')</script>"}"#;
        let request: Result<SearchRequest, _> = serde_json::from_str(invalid_json);
        assert!(request.is_err());

        // Invalid query with path traversal
        let invalid_json = r#"{"query": "../../../etc/passwd"}"#;
        let request: Result<SearchRequest, _> = serde_json::from_str(invalid_json);
        assert!(request.is_err());
    }
}