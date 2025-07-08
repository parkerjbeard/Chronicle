use anyhow::{anyhow, Result};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{error, info, warn};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,  // Subject (user identifier)
    pub exp: usize,   // Expiration time (timestamp)
    pub iat: usize,   // Issued at (timestamp)
    pub jti: String,  // JWT ID (unique token identifier)
    pub scope: Vec<String>, // Permissions/scopes
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: EncodingKey,
    pub jwt_decode_secret: DecodingKey,
    pub token_expiry: Duration,
    pub session_timeout: Duration,
    pub max_sessions: usize,
    pub require_2fa: bool,
}

/// Session information
#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: String,
    pub token_id: String,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub permissions: Vec<String>,
    pub client_info: ClientInfo,
}

/// Client information for security auditing
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub ip_address: String,
    pub user_agent: String,
    pub process_name: Option<String>,
}

/// Secure password for authentication
#[derive(ZeroizeOnDrop, Zeroize)]
pub struct SecurePassword {
    password: Vec<u8>,
}

impl SecurePassword {
    pub fn new(password: String) -> Self {
        Self {
            password: password.into_bytes(),
        }
    }

    pub fn verify(&self, provided: &str) -> bool {
        use ring::constant_time;
        
        let provided_bytes = provided.as_bytes();
        
        // Use constant-time comparison to prevent timing attacks
        constant_time::verify_slices_are_equal(&self.password, provided_bytes).is_ok()
    }
}

/// Authentication service managing JWT tokens and sessions
pub struct AuthService {
    config: AuthConfig,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    revoked_tokens: Arc<RwLock<HashMap<String, SystemTime>>>,
    failed_attempts: Arc<RwLock<HashMap<String, Vec<SystemTime>>>>,
    master_password: SecurePassword,
}

impl AuthService {
    /// Create a new authentication service with secure defaults
    pub fn new(master_password: String) -> Result<Self> {
        let jwt_secret = Self::generate_jwt_secret()?;
        let jwt_decode_secret = DecodingKey::from_secret(&jwt_secret);
        let jwt_encode_secret = EncodingKey::from_secret(&jwt_secret);

        let config = AuthConfig {
            jwt_secret: jwt_encode_secret,
            jwt_decode_secret,
            token_expiry: Duration::from_secs(3600), // 1 hour
            session_timeout: Duration::from_secs(86400), // 24 hours
            max_sessions: 5,
            require_2fa: false, // Can be enabled later
        };

        Ok(Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            revoked_tokens: Arc::new(RwLock::new(HashMap::new())),
            failed_attempts: Arc::new(RwLock::new(HashMap::new())),
            master_password: SecurePassword::new(master_password),
        })
    }

    /// Generate a cryptographically secure JWT secret
    fn generate_jwt_secret() -> Result<Vec<u8>> {
        let rng = SystemRandom::new();
        let mut secret = vec![0u8; 64]; // 512-bit secret
        rng.fill(&mut secret)
            .map_err(|_| anyhow!("Failed to generate JWT secret"))?;
        Ok(secret)
    }

    /// Authenticate user with master password
    pub fn authenticate(&self, password: &str, client_info: ClientInfo) -> Result<String> {
        // Check for rate limiting
        if self.is_rate_limited(&client_info.ip_address) {
            self.log_failed_attempt(&client_info.ip_address);
            return Err(anyhow!("Too many failed attempts. Please try again later."));
        }

        // Verify password
        if !self.master_password.verify(password) {
            self.log_failed_attempt(&client_info.ip_address);
            warn!("Failed authentication attempt from {}", client_info.ip_address);
            return Err(anyhow!("Invalid credentials"));
        }

        // Clear failed attempts on successful auth
        self.clear_failed_attempts(&client_info.ip_address);

        // Generate session
        let token = self.create_session("master", client_info)?;
        info!("Successful authentication for master user");
        
        Ok(token)
    }

    /// Create a new authenticated session
    fn create_session(&self, user_id: &str, client_info: ClientInfo) -> Result<String> {
        let now = SystemTime::now();
        let exp = now + self.config.token_expiry;
        let token_id = uuid::Uuid::new_v4().to_string();

        let claims = Claims {
            sub: user_id.to_string(),
            exp: exp.duration_since(UNIX_EPOCH)?.as_secs() as usize,
            iat: now.duration_since(UNIX_EPOCH)?.as_secs() as usize,
            jti: token_id.clone(),
            scope: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        };

        let token = encode(&Header::default(), &claims, &self.config.jwt_secret)
            .map_err(|e| anyhow!("Failed to create JWT token: {}", e))?;

        // Store session
        let session = Session {
            user_id: user_id.to_string(),
            token_id: token_id.clone(),
            created_at: now,
            last_accessed: now,
            permissions: claims.scope.clone(),
            client_info,
        };

        let mut sessions = self.sessions.write().unwrap();
        
        // Limit concurrent sessions
        let user_sessions: Vec<_> = sessions
            .iter()
            .filter(|(_, s)| s.user_id == user_id)
            .map(|(k, _)| k.clone())
            .collect();

        if user_sessions.len() >= self.config.max_sessions {
            // Remove oldest session
            if let Some(oldest_key) = user_sessions.first() {
                sessions.remove(oldest_key);
                info!("Removed oldest session for user {} due to session limit", user_id);
            }
        }

        sessions.insert(token_id, session);
        
        info!("Created new session for user {}", user_id);
        Ok(token)
    }

    /// Validate JWT token and return claims
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        // Check if token is revoked
        if self.is_token_revoked(token)? {
            return Err(anyhow!("Token has been revoked"));
        }

        let token_data: TokenData<Claims> = decode(
            token,
            &self.config.jwt_decode_secret,
            &Validation::new(Algorithm::HS256),
        ).map_err(|e| anyhow!("Invalid JWT token: {}", e))?;

        // Update session last accessed time
        if let Ok(mut sessions) = self.sessions.write() {
            if let Some(session) = sessions.get_mut(&token_data.claims.jti) {
                session.last_accessed = SystemTime::now();
            }
        }

        Ok(token_data.claims)
    }

    /// Check if token is revoked
    fn is_token_revoked(&self, token: &str) -> Result<bool> {
        let token_data: TokenData<Claims> = decode(
            token,
            &self.config.jwt_decode_secret,
            &Validation::new(Algorithm::HS256),
        ).map_err(|e| anyhow!("Cannot decode token for revocation check: {}", e))?;

        let revoked = self.revoked_tokens.read().unwrap();
        Ok(revoked.contains_key(&token_data.claims.jti))
    }

    /// Revoke a specific token
    pub fn revoke_token(&self, token: &str) -> Result<()> {
        let token_data: TokenData<Claims> = decode(
            token,
            &self.config.jwt_decode_secret,
            &Validation::new(Algorithm::HS256),
        ).map_err(|e| anyhow!("Cannot decode token for revocation: {}", e))?;

        let mut revoked = self.revoked_tokens.write().unwrap();
        revoked.insert(token_data.claims.jti.clone(), SystemTime::now());

        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(&token_data.claims.jti);

        info!("Revoked token for user {}", token_data.claims.sub);
        Ok(())
    }

    /// Revoke all tokens for a user
    pub fn revoke_all_user_tokens(&self, user_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let mut revoked = self.revoked_tokens.write().unwrap();

        let tokens_to_revoke: Vec<_> = sessions
            .iter()
            .filter(|(_, session)| session.user_id == user_id)
            .map(|(token_id, _)| token_id.clone())
            .collect();

        for token_id in tokens_to_revoke {
            revoked.insert(token_id.clone(), SystemTime::now());
            sessions.remove(&token_id);
        }

        info!("Revoked all tokens for user {}", user_id);
        Ok(())
    }

    /// Check if IP is rate limited
    fn is_rate_limited(&self, ip: &str) -> bool {
        let failed_attempts = self.failed_attempts.read().unwrap();
        
        if let Some(attempts) = failed_attempts.get(ip) {
            let recent_attempts = attempts
                .iter()
                .filter(|&time| time.elapsed().unwrap_or(Duration::MAX) < Duration::from_secs(900)) // 15 minutes
                .count();
            
            recent_attempts >= 5 // Max 5 attempts per 15 minutes
        } else {
            false
        }
    }

    /// Log failed authentication attempt
    fn log_failed_attempt(&self, ip: &str) {
        let mut failed_attempts = self.failed_attempts.write().unwrap();
        let entry = failed_attempts.entry(ip.to_string()).or_insert_with(Vec::new);
        entry.push(SystemTime::now());
        
        // Keep only recent attempts (last hour)
        entry.retain(|time| time.elapsed().unwrap_or(Duration::MAX) < Duration::from_secs(3600));
    }

    /// Clear failed attempts for IP
    fn clear_failed_attempts(&self, ip: &str) {
        let mut failed_attempts = self.failed_attempts.write().unwrap();
        failed_attempts.remove(ip);
    }

    /// Clean up expired sessions and revoked tokens
    pub fn cleanup_expired(&self) {
        let now = SystemTime::now();
        
        // Clean up expired sessions
        {
            let mut sessions = self.sessions.write().unwrap();
            sessions.retain(|_, session| {
                now.duration_since(session.last_accessed).unwrap_or(Duration::MAX) < self.config.session_timeout
            });
        }

        // Clean up old revoked tokens (keep for 24 hours)
        {
            let mut revoked = self.revoked_tokens.write().unwrap();
            revoked.retain(|_, revoked_time| {
                now.duration_since(*revoked_time).unwrap_or(Duration::MAX) < Duration::from_secs(86400)
            });
        }

        // Clean up old failed attempts (keep for 1 hour)
        {
            let mut failed_attempts = self.failed_attempts.write().unwrap();
            for (_, attempts) in failed_attempts.iter_mut() {
                attempts.retain(|time| {
                    now.duration_since(*time).unwrap_or(Duration::MAX) < Duration::from_secs(3600)
                });
            }
            failed_attempts.retain(|_, attempts| !attempts.is_empty());
        }
    }

    /// Get active session count
    pub fn get_active_session_count(&self) -> usize {
        self.sessions.read().unwrap().len()
    }

    /// Get session information
    pub fn get_session_info(&self, token_id: &str) -> Option<Session> {
        self.sessions.read().unwrap().get(token_id).cloned()
    }
}

/// Axum middleware for JWT authentication
pub async fn auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = request.headers();
    
    // Extract JWT token from Authorization header
    let token = extract_jwt_token(headers)?;
    
    // Validate token
    match auth_service.validate_token(&token) {
        Ok(claims) => {
            // Add claims to request extensions for use in handlers
            let mut request = request;
            request.extensions_mut().insert(claims);
            Ok(next.run(request).await)
        }
        Err(e) => {
            error!("Authentication failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Extract JWT token from Authorization header
fn extract_jwt_token(headers: &HeaderMap) -> Result<String, StatusCode> {
    let auth_header = headers
        .get("Authorization")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    if let Some(token) = auth_header.strip_prefix("Bearer ") {
        Ok(token.to_string())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Login request structure
#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

/// Login response structure
#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
    pub token_type: String,
}

/// Login handler
pub async fn login_handler(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let client_info = ClientInfo {
        ip_address: "127.0.0.1".to_string(), // In real implementation, extract from request
        user_agent: "Chronicle CLI".to_string(),
        process_name: None,
    };

    match auth_service.authenticate(&request.password, client_info) {
        Ok(token) => {
            let response = LoginResponse {
                token,
                expires_in: 3600, // 1 hour
                token_type: "Bearer".to_string(),
            };
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Logout handler
pub async fn logout_handler(
    State(auth_service): State<Arc<AuthService>>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let token = extract_jwt_token(&headers)?;
    
    match auth_service.revoke_token(&token) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_service_creation() {
        let auth_service = AuthService::new("test_password".to_string()).unwrap();
        assert_eq!(auth_service.get_active_session_count(), 0);
    }

    #[tokio::test]
    async fn test_authentication_success() {
        let auth_service = AuthService::new("test_password".to_string()).unwrap();
        let client_info = ClientInfo {
            ip_address: "127.0.0.1".to_string(),
            user_agent: "test".to_string(),
            process_name: None,
        };

        let token = auth_service.authenticate("test_password", client_info).unwrap();
        assert!(!token.is_empty());
        assert_eq!(auth_service.get_active_session_count(), 1);
    }

    #[tokio::test]
    async fn test_authentication_failure() {
        let auth_service = AuthService::new("test_password".to_string()).unwrap();
        let client_info = ClientInfo {
            ip_address: "127.0.0.1".to_string(),
            user_agent: "test".to_string(),
            process_name: None,
        };

        let result = auth_service.authenticate("wrong_password", client_info);
        assert!(result.is_err());
        assert_eq!(auth_service.get_active_session_count(), 0);
    }

    #[tokio::test]
    async fn test_token_validation() {
        let auth_service = AuthService::new("test_password".to_string()).unwrap();
        let client_info = ClientInfo {
            ip_address: "127.0.0.1".to_string(),
            user_agent: "test".to_string(),
            process_name: None,
        };

        let token = auth_service.authenticate("test_password", client_info).unwrap();
        let claims = auth_service.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "master");
    }

    #[tokio::test]
    async fn test_token_revocation() {
        let auth_service = AuthService::new("test_password".to_string()).unwrap();
        let client_info = ClientInfo {
            ip_address: "127.0.0.1".to_string(),
            user_agent: "test".to_string(),
            process_name: None,
        };

        let token = auth_service.authenticate("test_password", client_info).unwrap();
        auth_service.revoke_token(&token).unwrap();
        
        let result = auth_service.validate_token(&token);
        assert!(result.is_err());
        assert_eq!(auth_service.get_active_session_count(), 0);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let auth_service = AuthService::new("test_password".to_string()).unwrap();
        let client_info = ClientInfo {
            ip_address: "127.0.0.1".to_string(),
            user_agent: "test".to_string(),
            process_name: None,
        };

        // Make 5 failed attempts
        for _ in 0..5 {
            let _ = auth_service.authenticate("wrong_password", client_info.clone());
        }

        // 6th attempt should be rate limited
        let result = auth_service.authenticate("wrong_password", client_info);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many failed attempts"));
    }
}