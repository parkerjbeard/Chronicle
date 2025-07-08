use anyhow::{anyhow, Result};
use rand::{distributions::Alphanumeric, Rng};
use ring::{
    digest,
    pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    num::NonZeroU32,
    time::{Duration, SystemTime},
};
use tracing::{info, warn};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure authentication challenge for destructive operations
#[derive(Debug, Clone)]
pub struct AuthChallenge {
    pub challenge_id: String,
    pub challenge_phrase: String,
    pub expected_response: String,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub operation_type: String,
}

/// Challenge response from user
#[derive(Deserialize)]
pub struct ChallengeResponse {
    pub challenge_id: String,
    pub response: String,
    pub additional_confirmation: String,
}

/// Secure authentication manager for high-risk operations
pub struct SecureAuthManager {
    active_challenges: HashMap<String, AuthChallenge>,
    challenge_history: Vec<String>, // Track used challenges to prevent replay
    rng: SystemRandom,
    master_key: [u8; 32], // For deriving challenge secrets
}

/// Secure passphrase that auto-zeros on drop
#[derive(ZeroizeOnDrop, Zeroize)]
pub struct SecurePassphrase {
    passphrase: String,
}

impl SecurePassphrase {
    pub fn new(passphrase: String) -> Self {
        Self { passphrase }
    }

    pub fn as_str(&self) -> &str {
        &self.passphrase
    }

    pub fn verify(&self, provided: &str) -> bool {
        use ring::constant_time;
        constant_time::verify_slices_are_equal(
            self.passphrase.as_bytes(), 
            provided.as_bytes()
        ).is_ok()
    }
}

impl SecureAuthManager {
    pub fn new() -> Result<Self> {
        let rng = SystemRandom::new();
        let mut master_key = [0u8; 32];
        rng.fill(&mut master_key)
            .map_err(|_| anyhow!("Failed to generate master key"))?;

        Ok(Self {
            active_challenges: HashMap::new(),
            challenge_history: Vec::new(),
            rng,
            master_key,
        })
    }

    /// Generate a secure challenge for destructive operations
    pub fn generate_challenge(&mut self, operation_type: &str) -> Result<AuthChallenge> {
        self.cleanup_expired_challenges();

        let challenge_id = self.generate_secure_id()?;
        let challenge_phrase = self.generate_challenge_phrase(operation_type)?;
        let expected_response = self.derive_expected_response(&challenge_id, &challenge_phrase)?;
        
        let now = SystemTime::now();
        let expires_at = now + Duration::from_secs(300); // 5 minutes

        let challenge = AuthChallenge {
            challenge_id: challenge_id.clone(),
            challenge_phrase,
            expected_response,
            created_at: now,
            expires_at,
            operation_type: operation_type.to_string(),
        };

        self.active_challenges.insert(challenge_id, challenge.clone());
        info!("Generated challenge for operation: {}", operation_type);

        Ok(challenge)
    }

    /// Verify challenge response
    pub fn verify_challenge(&mut self, response: &ChallengeResponse) -> Result<bool> {
        self.cleanup_expired_challenges();

        let challenge = self.active_challenges.get(&response.challenge_id)
            .ok_or_else(|| anyhow!("Invalid or expired challenge"))?;

        // Check if challenge has expired
        if SystemTime::now() > challenge.expires_at {
            self.active_challenges.remove(&response.challenge_id);
            return Err(anyhow!("Challenge has expired"));
        }

        // Verify the response using constant-time comparison
        let is_valid = ring::constant_time::verify_slices_are_equal(
            challenge.expected_response.as_bytes(),
            response.response.as_bytes(),
        ).is_ok();

        // Verify additional confirmation for destructive operations
        let has_confirmation = match challenge.operation_type.as_str() {
            "wipe_all" => {
                response.additional_confirmation == "I understand this will permanently delete all data"
            }
            "wipe_selective" => {
                response.additional_confirmation == "I understand this will permanently delete selected data"
            }
            _ => true, // Other operations don't require additional confirmation
        };

        if is_valid && has_confirmation {
            // Remove challenge after successful verification (one-time use)
            self.active_challenges.remove(&response.challenge_id);
            self.challenge_history.push(response.challenge_id.clone());
            
            // Keep only last 100 used challenges to prevent memory growth
            if self.challenge_history.len() > 100 {
                self.challenge_history.remove(0);
            }

            info!("Challenge successfully verified for operation: {}", challenge.operation_type);
            Ok(true)
        } else {
            warn!("Failed challenge verification attempt");
            Ok(false)
        }
    }

    /// Generate cryptographically secure ID
    fn generate_secure_id(&self) -> Result<String> {
        let mut bytes = [0u8; 16];
        self.rng.fill(&mut bytes)
            .map_err(|_| anyhow!("Failed to generate secure ID"))?;
        Ok(hex::encode(bytes))
    }

    /// Generate human-readable challenge phrase
    fn generate_challenge_phrase(&self, operation_type: &str) -> Result<String> {
        let words = match operation_type {
            "wipe_all" => &[
                "DESTROY", "ELIMINATE", "ERASE", "PURGE", "OBLITERATE",
                "ANNIHILATE", "EXTERMINATE", "DEMOLISH"
            ],
            "wipe_selective" => &[
                "REMOVE", "DELETE", "DISCARD", "EXPUNGE", "CLEAR",
                "CLEAN", "FLUSH", "VOID"
            ],
            "export_sensitive" => &[
                "EXTRACT", "EXPORT", "TRANSFER", "BACKUP", "ARCHIVE",
                "COPY", "REPLICATE", "DUPLICATE"
            ],
            _ => &[
                "CONFIRM", "VERIFY", "AUTHENTICATE", "VALIDATE", "AUTHORIZE",
                "PROCEED", "EXECUTE", "CONTINUE"
            ],
        };

        // Select 3 random words
        let mut selected_words = Vec::new();
        for _ in 0..3 {
            let mut random_bytes = [0u8; 1];
            self.rng.fill(&mut random_bytes)
                .map_err(|_| anyhow!("Failed to generate random word selection"))?;
            let index = (random_bytes[0] as usize) % words.len();
            selected_words.push(words[index]);
        }

        // Add random number
        let mut random_bytes = [0u8; 2];
        self.rng.fill(&mut random_bytes)
            .map_err(|_| anyhow!("Failed to generate random number"))?;
        let random_num = u16::from_be_bytes(random_bytes) % 1000;

        Ok(format!("{} {} {} {}", 
            selected_words[0], 
            selected_words[1], 
            selected_words[2], 
            random_num
        ))
    }

    /// Derive expected response from challenge using cryptographic derivation
    fn derive_expected_response(&self, challenge_id: &str, challenge_phrase: &str) -> Result<String> {
        let input = format!("{}:{}", challenge_id, challenge_phrase);
        
        // Use PBKDF2 to derive response
        let mut response_bytes = [0u8; 32];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(10000).unwrap(), // 10k iterations
            challenge_id.as_bytes(), // Salt
            input.as_bytes(),
            &mut response_bytes,
        );

        // Convert to human-readable format
        let response_hash = digest::digest(&digest::SHA256, &response_bytes);
        let response_hex = hex::encode(response_hash.as_ref());
        
        // Take first 8 characters and format nicely
        let response_code = &response_hex[0..8].to_uppercase();
        Ok(format!("{}-{}", &response_code[0..4], &response_code[4..8]))
    }

    /// Clean up expired challenges
    fn cleanup_expired_challenges(&mut self) {
        let now = SystemTime::now();
        self.active_challenges.retain(|_, challenge| now <= challenge.expires_at);
    }

    /// Get active challenge count for monitoring
    pub fn get_active_challenge_count(&self) -> usize {
        self.active_challenges.len()
    }

    /// Generate temporary single-use password for API authentication
    pub fn generate_temporary_password(&self, duration_secs: u64) -> Result<(String, SystemTime)> {
        // Generate random password
        let password: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let expires_at = SystemTime::now() + Duration::from_secs(duration_secs);
        
        Ok((password, expires_at))
    }
}

/// Secure authentication for CLI operations
pub struct CliSecureAuth {
    auth_manager: SecureAuthManager,
}

impl CliSecureAuth {
    pub fn new() -> Result<Self> {
        Ok(Self {
            auth_manager: SecureAuthManager::new()?,
        })
    }

    /// Authenticate destructive operation with challenge-response
    pub fn authenticate_destructive_operation(
        &mut self, 
        operation_type: &str,
        output: &crate::output::OutputManager,
    ) -> Result<bool> {
        // Generate challenge
        let challenge = self.auth_manager.generate_challenge(operation_type)?;
        
        // Display challenge to user
        output.print_warning("🔐 SECURE AUTHENTICATION REQUIRED")?;
        output.print_info("To proceed with this destructive operation, you must complete a security challenge.")?;
        output.print_key_value("Challenge ID", &challenge.challenge_id)?;
        output.print_key_value("Challenge Phrase", &challenge.challenge_phrase)?;
        
        // Get expected response
        let expected_response = challenge.expected_response.clone();
        
        output.print_info("Please calculate the authentication code using the Chronicle security tool:")?;
        output.print_info("chronictl auth-code --challenge-id {} --phrase '{}'", 
            &challenge.challenge_id, 
            &challenge.challenge_phrase
        )?;
        
        // Prompt for response
        let response_code = output.prompt_password("Enter authentication code")?;
        
        // Get additional confirmation
        let confirmation_message = match operation_type {
            "wipe_all" => "I understand this will permanently delete all data",
            "wipe_selective" => "I understand this will permanently delete selected data",
            _ => "I confirm this operation",
        };
        
        output.print_warning(&format!("Type exactly: {}", confirmation_message))?;
        let confirmation = output.prompt_string("Confirmation")?;
        
        // Verify response
        let response = ChallengeResponse {
            challenge_id: challenge.challenge_id,
            response: response_code,
            additional_confirmation: confirmation,
        };
        
        match self.auth_manager.verify_challenge(&response) {
            Ok(true) => {
                output.print_success("✓ Authentication successful")?;
                Ok(true)
            }
            Ok(false) => {
                output.print_error("✗ Authentication failed")?;
                Ok(false)
            }
            Err(e) => {
                output.print_error(&format!("Authentication error: {}", e))?;
                Ok(false)
            }
        }
    }
}

/// Helper function to calculate authentication code (for CLI tool)
pub fn calculate_auth_code(challenge_id: &str, challenge_phrase: &str) -> Result<String> {
    let input = format!("{}:{}", challenge_id, challenge_phrase);
    
    // Use same derivation as server
    let mut response_bytes = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(10000).unwrap(),
        challenge_id.as_bytes(),
        input.as_bytes(),
        &mut response_bytes,
    );

    let response_hash = digest::digest(&digest::SHA256, &response_bytes);
    let response_hex = hex::encode(response_hash.as_ref());
    let response_code = &response_hex[0..8].to_uppercase();
    
    Ok(format!("{}-{}", &response_code[0..4], &response_code[4..8]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_auth_manager_creation() {
        let auth_manager = SecureAuthManager::new().unwrap();
        assert_eq!(auth_manager.get_active_challenge_count(), 0);
    }

    #[test]
    fn test_challenge_generation() {
        let mut auth_manager = SecureAuthManager::new().unwrap();
        let challenge = auth_manager.generate_challenge("test_operation").unwrap();
        
        assert!(!challenge.challenge_id.is_empty());
        assert!(!challenge.challenge_phrase.is_empty());
        assert!(!challenge.expected_response.is_empty());
        assert_eq!(auth_manager.get_active_challenge_count(), 1);
    }

    #[test]
    fn test_challenge_verification() {
        let mut auth_manager = SecureAuthManager::new().unwrap();
        let challenge = auth_manager.generate_challenge("test_operation").unwrap();
        
        let response = ChallengeResponse {
            challenge_id: challenge.challenge_id,
            response: challenge.expected_response,
            additional_confirmation: "I confirm this operation".to_string(),
        };
        
        let result = auth_manager.verify_challenge(&response).unwrap();
        assert!(result);
        assert_eq!(auth_manager.get_active_challenge_count(), 0); // Challenge consumed
    }

    #[test]
    fn test_invalid_challenge_verification() {
        let mut auth_manager = SecureAuthManager::new().unwrap();
        let challenge = auth_manager.generate_challenge("test_operation").unwrap();
        
        let response = ChallengeResponse {
            challenge_id: challenge.challenge_id,
            response: "wrong_response".to_string(),
            additional_confirmation: "I confirm this operation".to_string(),
        };
        
        let result = auth_manager.verify_challenge(&response).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_auth_code_calculation() {
        let challenge_id = "test_challenge_123";
        let challenge_phrase = "TEST PHRASE EXAMPLE 123";
        
        let code = calculate_auth_code(challenge_id, challenge_phrase).unwrap();
        assert_eq!(code.len(), 9); // Format: XXXX-XXXX
        assert!(code.contains('-'));
        
        // Same inputs should produce same output
        let code2 = calculate_auth_code(challenge_id, challenge_phrase).unwrap();
        assert_eq!(code, code2);
    }

    #[test]
    fn test_secure_passphrase() {
        let passphrase = SecurePassphrase::new("test_passphrase".to_string());
        assert!(passphrase.verify("test_passphrase"));
        assert!(!passphrase.verify("wrong_passphrase"));
    }
}