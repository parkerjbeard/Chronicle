use anyhow::{anyhow, Result};
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use rustls::{Certificate as RustlsCertificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use time::OffsetDateTime;
use tracing::{info, warn};

/// TLS configuration for the Chronicle server
pub struct TlsConfig {
    pub server_config: Arc<ServerConfig>,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_cert_path: PathBuf,
}

/// Certificate manager for generating and managing TLS certificates
pub struct CertificateManager {
    config_dir: PathBuf,
}

impl CertificateManager {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// Generate or load TLS configuration
    pub fn get_or_create_tls_config(&self) -> Result<TlsConfig> {
        let cert_path = self.config_dir.join("server.crt");
        let key_path = self.config_dir.join("server.key");
        let ca_cert_path = self.config_dir.join("ca.crt");

        // Check if certificates exist and are valid
        if self.certificates_exist(&cert_path, &key_path) {
            match self.load_existing_certificates(&cert_path, &key_path) {
                Ok(config) => {
                    info!("Loaded existing TLS certificates");
                    return Ok(TlsConfig {
                        server_config: config,
                        cert_path,
                        key_path,
                        ca_cert_path,
                    });
                }
                Err(e) => {
                    warn!("Failed to load existing certificates: {}. Generating new ones.", e);
                }
            }
        }

        // Generate new certificates
        info!("Generating new TLS certificates for Chronicle server");
        let (server_cert, server_key, ca_cert) = self.generate_certificates()?;
        
        // Save certificates to disk
        self.save_certificate(&cert_path, &server_cert)?;
        self.save_private_key(&key_path, &server_key)?;
        self.save_certificate(&ca_cert_path, &ca_cert)?;

        // Create server configuration
        let server_config = self.create_server_config(&server_cert, &server_key)?;

        Ok(TlsConfig {
            server_config,
            cert_path,
            key_path,
            ca_cert_path,
        })
    }

    /// Check if certificate files exist
    fn certificates_exist(&self, cert_path: &Path, key_path: &Path) -> bool {
        cert_path.exists() && key_path.exists()
    }

    /// Load existing certificates from disk
    fn load_existing_certificates(&self, cert_path: &Path, key_path: &Path) -> Result<Arc<ServerConfig>> {
        // Read certificate file
        let cert_file = File::open(cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain: Vec<RustlsCertificate> = certs(&mut cert_reader)?
            .into_iter()
            .map(RustlsCertificate)
            .collect();

        if cert_chain.is_empty() {
            return Err(anyhow!("No certificates found in file"));
        }

        // Read private key file
        let key_file = File::open(key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let keys = pkcs8_private_keys(&mut key_reader)?;

        if keys.is_empty() {
            return Err(anyhow!("No private keys found in file"));
        }

        let private_key = PrivateKey(keys[0].clone());

        // Create server configuration
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| anyhow!("Failed to create TLS config: {}", e))?;

        Ok(Arc::new(config))
    }

    /// Generate new self-signed certificates
    fn generate_certificates(&self) -> Result<(String, String, String)> {
        // Generate CA certificate
        let ca_cert = self.generate_ca_certificate()?;
        let ca_key_pair = ca_cert.get_key_pair();

        // Generate server certificate signed by CA
        let server_cert = self.generate_server_certificate(&ca_cert, ca_key_pair)?;

        // Convert to PEM format
        let ca_cert_pem = ca_cert.serialize_pem()?;
        let server_cert_pem = server_cert.serialize_pem()?;
        let server_key_pem = server_cert.serialize_private_key_pem();

        Ok((server_cert_pem, server_key_pem, ca_cert_pem))
    }

    /// Generate Certificate Authority certificate
    fn generate_ca_certificate(&self) -> Result<Certificate> {
        let mut params = CertificateParams::new(vec!["Chronicle Root CA".to_string()]);
        
        // Set CA-specific parameters
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![
            rcgen::KeyUsagePurpose::KeyCertSign,
            rcgen::KeyUsagePurpose::CrlSign,
        ];

        // Set validity period (1 year)
        let now = OffsetDateTime::now_utc();
        params.not_before = now;
        params.not_after = now + time::Duration::days(365);

        // Set distinguished name
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, "Chronicle Root CA");
        distinguished_name.push(DnType::OrganizationName, "Chronicle");
        distinguished_name.push(DnType::CountryName, "US");
        params.distinguished_name = distinguished_name;

        // Generate certificate
        Certificate::from_params(params).map_err(|e| anyhow!("Failed to generate CA certificate: {}", e))
    }

    /// Generate server certificate signed by CA
    fn generate_server_certificate(&self, ca_cert: &Certificate, ca_key_pair: &KeyPair) -> Result<Certificate> {
        let mut params = CertificateParams::new(vec!["localhost".to_string()]);
        
        // Add Subject Alternative Names for local development
        params.subject_alt_names = vec![
            SanType::DnsName("localhost".to_string()),
            SanType::DnsName("127.0.0.1".to_string()),
            SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
            SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))),
        ];

        // Set server-specific parameters
        params.key_usages = vec![
            rcgen::KeyUsagePurpose::DigitalSignature,
            rcgen::KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![
            rcgen::ExtendedKeyUsagePurpose::ServerAuth,
        ];

        // Set validity period (90 days for server cert)
        let now = OffsetDateTime::now_utc();
        params.not_before = now;
        params.not_after = now + time::Duration::days(90);

        // Set distinguished name
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, "Chronicle Server");
        distinguished_name.push(DnType::OrganizationName, "Chronicle");
        distinguished_name.push(DnType::CountryName, "US");
        params.distinguished_name = distinguished_name;

        // Generate certificate signed by CA
        let server_cert = Certificate::from_params(params)
            .map_err(|e| anyhow!("Failed to generate server certificate params: {}", e))?;
        
        let server_cert_der = server_cert.serialize_der_with_signer(ca_cert)
            .map_err(|e| anyhow!("Failed to sign server certificate: {}", e))?;

        // Create new certificate from the signed DER
        let server_key_pair = server_cert.get_key_pair();
        let mut signed_params = CertificateParams::from_ca_cert_der(&server_cert_der, server_key_pair.clone())
            .map_err(|e| anyhow!("Failed to create signed certificate params: {}", e))?;
        
        signed_params.alg = &rcgen::PKCS_ECDSA_P256_SHA256;
        
        Certificate::from_params(signed_params)
            .map_err(|e| anyhow!("Failed to create signed certificate: {}", e))
    }

    /// Create rustls ServerConfig from certificate and key
    fn create_server_config(&self, cert_pem: &str, key_pem: &str) -> Result<Arc<ServerConfig>> {
        // Parse certificate chain
        let cert_chain: Vec<RustlsCertificate> = rustls_pemfile::certs(&mut cert_pem.as_bytes())?
            .into_iter()
            .map(RustlsCertificate)
            .collect();

        if cert_chain.is_empty() {
            return Err(anyhow!("No certificates found in PEM data"));
        }

        // Parse private key
        let keys = rustls_pemfile::pkcs8_private_keys(&mut key_pem.as_bytes())?;
        if keys.is_empty() {
            return Err(anyhow!("No private keys found in PEM data"));
        }

        let private_key = PrivateKey(keys[0].clone());

        // Create server configuration with security best practices
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| anyhow!("Failed to create TLS server config: {}", e))?;

        Ok(Arc::new(config))
    }

    /// Save certificate to file with secure permissions
    fn save_certificate(&self, path: &Path, cert_pem: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write certificate file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        file.write_all(cert_pem.as_bytes())?;
        file.sync_all()?;

        // Set secure file permissions (readable by owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = file.metadata()?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600); // Owner read/write only
            std::fs::set_permissions(path, permissions)?;
        }

        info!("Saved certificate to {}", path.display());
        Ok(())
    }

    /// Save private key to file with secure permissions
    fn save_private_key(&self, path: &Path, key_pem: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write private key file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        file.write_all(key_pem.as_bytes())?;
        file.sync_all()?;

        // Set very restrictive permissions for private key
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = file.metadata()?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600); // Owner read/write only
            std::fs::set_permissions(path, permissions)?;
        }

        info!("Saved private key to {}", path.display());
        Ok(())
    }

    /// Validate certificate expiration
    pub fn check_certificate_expiry(&self, cert_path: &Path) -> Result<bool> {
        let cert_file = File::open(cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs = certs(&mut cert_reader)?;

        if certs.is_empty() {
            return Err(anyhow!("No certificates found"));
        }

        // Parse the first certificate to check expiry
        let cert = &certs[0];
        let parsed = x509_parser::parse_x509_certificate(cert)
            .map_err(|e| anyhow!("Failed to parse certificate: {}", e))?;

        let now = OffsetDateTime::now_utc();
        let not_after = OffsetDateTime::from_unix_timestamp(parsed.1.validity().not_after.timestamp())
            .map_err(|e| anyhow!("Failed to parse certificate expiry: {}", e))?;

        // Check if certificate expires within 30 days
        let expires_soon = (not_after - now) < time::Duration::days(30);
        
        if expires_soon {
            warn!("TLS certificate expires soon: {}", not_after);
        }

        Ok(expires_soon)
    }

    /// Rotate certificates if needed
    pub fn rotate_certificates_if_needed(&self) -> Result<Option<TlsConfig>> {
        let cert_path = self.config_dir.join("server.crt");
        
        if !cert_path.exists() {
            return Ok(None);
        }

        match self.check_certificate_expiry(&cert_path) {
            Ok(true) => {
                info!("Certificate expires soon, generating new certificates");
                Ok(Some(self.get_or_create_tls_config()?))
            }
            Ok(false) => Ok(None),
            Err(e) => {
                warn!("Failed to check certificate expiry: {}. Generating new certificates.", e);
                Ok(Some(self.get_or_create_tls_config()?))
            }
        }
    }
}

/// Certificate information for monitoring
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: OffsetDateTime,
    pub not_after: OffsetDateTime,
    pub serial_number: String,
    pub fingerprint: String,
}

impl CertificateInfo {
    pub fn from_certificate_file(path: &Path) -> Result<Self> {
        let cert_file = File::open(path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs = certs(&mut cert_reader)?;

        if certs.is_empty() {
            return Err(anyhow!("No certificates found"));
        }

        let cert = &certs[0];
        let parsed = x509_parser::parse_x509_certificate(cert)
            .map_err(|e| anyhow!("Failed to parse certificate: {}", e))?;

        let cert_info = parsed.1;
        
        // Calculate SHA-256 fingerprint
        let fingerprint = {
            use ring::digest;
            let digest = digest::digest(&digest::SHA256, cert);
            hex::encode(digest.as_ref())
        };

        Ok(CertificateInfo {
            subject: cert_info.subject().to_string(),
            issuer: cert_info.issuer().to_string(),
            not_before: OffsetDateTime::from_unix_timestamp(cert_info.validity().not_before.timestamp())?,
            not_after: OffsetDateTime::from_unix_timestamp(cert_info.validity().not_after.timestamp())?,
            serial_number: cert_info.serial.to_str_radix(16),
            fingerprint,
        })
    }

    pub fn is_expired(&self) -> bool {
        OffsetDateTime::now_utc() > self.not_after
    }

    pub fn expires_within_days(&self, days: i64) -> bool {
        let expiry_threshold = OffsetDateTime::now_utc() + time::Duration::days(days);
        self.not_after < expiry_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_certificate_generation() {
        let temp_dir = TempDir::new().unwrap();
        let cert_manager = CertificateManager::new(temp_dir.path().to_path_buf());
        
        let tls_config = cert_manager.get_or_create_tls_config().unwrap();
        
        // Check that certificate files were created
        assert!(tls_config.cert_path.exists());
        assert!(tls_config.key_path.exists());
        assert!(tls_config.ca_cert_path.exists());
    }

    #[tokio::test]
    async fn test_certificate_loading() {
        let temp_dir = TempDir::new().unwrap();
        let cert_manager = CertificateManager::new(temp_dir.path().to_path_buf());
        
        // Generate certificates first
        let _tls_config1 = cert_manager.get_or_create_tls_config().unwrap();
        
        // Load existing certificates
        let _tls_config2 = cert_manager.get_or_create_tls_config().unwrap();
        
        // Should load existing certificates without regenerating
    }

    #[tokio::test]
    async fn test_certificate_info() {
        let temp_dir = TempDir::new().unwrap();
        let cert_manager = CertificateManager::new(temp_dir.path().to_path_buf());
        
        let tls_config = cert_manager.get_or_create_tls_config().unwrap();
        let cert_info = CertificateInfo::from_certificate_file(&tls_config.cert_path).unwrap();
        
        assert!(!cert_info.is_expired());
        assert!(!cert_info.expires_within_days(30));
        assert!(!cert_info.fingerprint.is_empty());
    }
}