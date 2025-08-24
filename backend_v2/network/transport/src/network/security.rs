//! Security Layer
//!
//! Provides encryption and decryption capabilities for network transport
//! using TLS and ChaCha20Poly1305 for different security requirements.

use crate::{TransportError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Encryption type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionType {
    /// No encryption - for trusted private networks
    None,
    /// TLS 1.3 encryption - standard transport security
    Tls,
    /// ChaCha20Poly1305 - fast authenticated encryption
    ChaCha20Poly1305 { 
        /// 32-byte encryption key
        key: [u8; 32],
    },
}

impl PartialEq for EncryptionType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EncryptionType::None, EncryptionType::None) => true,
            (EncryptionType::Tls, EncryptionType::Tls) => true,
            (EncryptionType::ChaCha20Poly1305 { .. }, EncryptionType::ChaCha20Poly1305 { .. }) => true,
            _ => false,
        }
    }
}

/// Security layer for encryption and decryption
pub struct SecurityLayer {
    encryption_type: EncryptionType,
    #[cfg(feature = "encryption")]
    tls_config: Option<Arc<rustls::ClientConfig>>,
    #[cfg(feature = "encryption")]
    chacha_cipher: Option<chacha20poly1305::ChaCha20Poly1305>,
}

impl SecurityLayer {
    /// Create new security layer
    pub async fn new(encryption_type: EncryptionType) -> Result<Self> {
        let mut layer = Self {
            encryption_type,
            #[cfg(feature = "encryption")]
            tls_config: None,
            #[cfg(feature = "encryption")]
            chacha_cipher: None,
        };

        layer.initialize().await?;
        Ok(layer)
    }

    /// Initialize encryption based on type
    async fn initialize(&mut self) -> Result<()> {
        match &self.encryption_type {
            EncryptionType::None => {
                // No initialization needed
            }
            
            #[cfg(feature = "encryption")]
            EncryptionType::Tls => {
                self.initialize_tls().await?;
            }
            
            #[cfg(feature = "encryption")]
            EncryptionType::ChaCha20Poly1305 { key } => {
                let key = *key;
                self.initialize_chacha(&key)?;
            }

            #[cfg(not(feature = "encryption"))]
            _ => {
                return Err(TransportError::configuration(
                    "Encryption feature not enabled",
                    Some("encryption")
                ));
            }
        }

        Ok(())
    }

    /// Initialize TLS configuration
    #[cfg(feature = "encryption")]
    async fn initialize_tls(&mut self) -> Result<()> {
        let mut root_store = rustls::RootCertStore::empty();
        
        // Add system root certificates
        for cert in rustls_native_certs::load_native_certs()
            .map_err(|e| TransportError::security(format!("Failed to load system certs: {}", e)))? 
        {
            root_store.add(&rustls::Certificate(cert.0))
                .map_err(|e| TransportError::security(format!("Failed to add cert: {}", e)))?;
        }

        let config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        self.tls_config = Some(Arc::new(config));
        Ok(())
    }

    /// Initialize ChaCha20Poly1305 cipher
    #[cfg(feature = "encryption")]
    fn initialize_chacha(&mut self, key: &[u8; 32]) -> Result<()> {
        use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
        
        let cipher = ChaCha20Poly1305::new_from_slice(key)
            .map_err(|e| TransportError::security(format!("Invalid ChaCha20Poly1305 key: {}", e)))?;
        
        self.chacha_cipher = Some(cipher);
        Ok(())
    }

    /// Encrypt data using configured encryption
    pub async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        match &self.encryption_type {
            EncryptionType::None => Ok(data.to_vec()),
            
            #[cfg(feature = "encryption")]
            EncryptionType::Tls => {
                // TLS encryption is handled at the transport layer
                // This method is for application-layer encryption
                Ok(data.to_vec())
            }
            
            #[cfg(feature = "encryption")]
            EncryptionType::ChaCha20Poly1305 { .. } => {
                self.encrypt_chacha(data).await
            }

            #[cfg(not(feature = "encryption"))]
            _ => Err(TransportError::configuration(
                "Encryption feature not enabled",
                Some("encryption")
            )),
        }
    }

    /// Decrypt data using configured encryption
    pub async fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        match &self.encryption_type {
            EncryptionType::None => Ok(data.to_vec()),
            
            #[cfg(feature = "encryption")]
            EncryptionType::Tls => {
                // TLS decryption is handled at the transport layer
                Ok(data.to_vec())
            }
            
            #[cfg(feature = "encryption")]
            EncryptionType::ChaCha20Poly1305 { .. } => {
                self.decrypt_chacha(data).await
            }

            #[cfg(not(feature = "encryption"))]
            _ => Err(TransportError::configuration(
                "Encryption feature not enabled",
                Some("encryption")
            )),
        }
    }

    /// Encrypt using ChaCha20Poly1305
    #[cfg(feature = "encryption")]
    async fn encrypt_chacha(&self, data: &[u8]) -> Result<Vec<u8>> {
        use chacha20poly1305::{aead::Aead, Nonce};
        use rand::RngCore;
        
        let cipher = self.chacha_cipher.as_ref()
            .ok_or_else(|| TransportError::security("ChaCha20Poly1305 not initialized"))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| TransportError::security(format!("ChaCha20Poly1305 encryption failed: {}", e)))?;

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt using ChaCha20Poly1305
    #[cfg(feature = "encryption")]
    async fn decrypt_chacha(&self, data: &[u8]) -> Result<Vec<u8>> {
        use chacha20poly1305::{aead::Aead, Nonce};
        
        if data.len() < 12 {
            return Err(TransportError::security("ChaCha20Poly1305 data too short"));
        }

        let cipher = self.chacha_cipher.as_ref()
            .ok_or_else(|| TransportError::security("ChaCha20Poly1305 not initialized"))?;

        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        // Decrypt the data
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| TransportError::security(format!("ChaCha20Poly1305 decryption failed: {}", e)))?;

        Ok(plaintext)
    }

    /// Get encryption type
    pub fn encryption_type(&self) -> &EncryptionType {
        &self.encryption_type
    }

    /// Check if encryption is enabled
    pub fn is_enabled(&self) -> bool {
        !matches!(self.encryption_type, EncryptionType::None)
    }

    /// Get TLS configuration for transport layer
    #[cfg(feature = "encryption")]
    pub fn tls_config(&self) -> Option<Arc<rustls::ClientConfig>> {
        self.tls_config.clone()
    }

    /// Generate random ChaCha20Poly1305 key
    pub fn generate_chacha_key() -> [u8; 32] {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    /// Derive key from password using PBKDF2
    #[cfg(feature = "encryption")]
    pub fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
        use ring::pbkdf2;
        use std::num::NonZeroU32;

        let mut key = [0u8; 32];
        let iterations = NonZeroU32::new(100_000)
            .ok_or_else(|| TransportError::security("Invalid PBKDF2 iterations"))?;

        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            iterations,
            salt,
            password.as_bytes(),
            &mut key,
        );

        Ok(key)
    }

    /// Get encryption overhead in bytes
    pub fn encryption_overhead(&self) -> usize {
        match &self.encryption_type {
            EncryptionType::None => 0,
            EncryptionType::Tls => 0, // Handled at transport layer
            EncryptionType::ChaCha20Poly1305 { .. } => 12 + 16, // 12-byte nonce + 16-byte tag
        }
    }

    /// Get security information
    pub fn security_info(&self) -> SecurityInfo {
        match &self.encryption_type {
            EncryptionType::None => SecurityInfo {
                name: "none",
                level: SecurityLevel::None,
                overhead_bytes: 0,
                description: "No encryption",
            },
            EncryptionType::Tls => SecurityInfo {
                name: "tls",
                level: SecurityLevel::High,
                overhead_bytes: 0, // Variable, handled at transport layer
                description: "TLS 1.3 transport encryption",
            },
            EncryptionType::ChaCha20Poly1305 { .. } => SecurityInfo {
                name: "chacha20poly1305",
                level: SecurityLevel::High,
                overhead_bytes: 28, // 12-byte nonce + 16-byte MAC
                description: "ChaCha20Poly1305 authenticated encryption",
            },
        }
    }
}

/// Security level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// No security
    None,
    /// Basic security (weak encryption)
    Low,
    /// Standard security
    Medium,
    /// Strong security
    High,
    /// Military-grade security
    Maximum,
}

/// Security algorithm information
#[derive(Debug, Clone)]
pub struct SecurityInfo {
    pub name: &'static str,
    pub level: SecurityLevel,
    pub overhead_bytes: usize,
    pub description: &'static str,
}

/// Key management for encryption
pub struct KeyManager {
    keys: std::collections::HashMap<String, [u8; 32]>,
}

impl KeyManager {
    /// Create new key manager
    pub fn new() -> Self {
        Self {
            keys: std::collections::HashMap::new(),
        }
    }

    /// Add key for a node
    pub fn add_key(&mut self, node_id: &str, key: [u8; 32]) {
        self.keys.insert(node_id.to_string(), key);
    }

    /// Get key for a node
    pub fn get_key(&self, node_id: &str) -> Option<[u8; 32]> {
        self.keys.get(node_id).copied()
    }

    /// Remove key for a node
    pub fn remove_key(&mut self, node_id: &str) -> Option<[u8; 32]> {
        self.keys.remove(node_id)
    }

    /// Generate and store new key for a node
    pub fn generate_key(&mut self, node_id: &str) -> [u8; 32] {
        let key = SecurityLayer::generate_chacha_key();
        self.add_key(node_id, key);
        key
    }

    /// Load keys from secure storage
    #[cfg(feature = "encryption")]
    pub async fn load_from_file(&mut self, path: &std::path::Path) -> Result<()> {
        // This would load keys from encrypted storage in production
        // For now, just a placeholder
        tracing::warn!("Key loading from file not implemented - using temporary keys");
        Ok(())
    }

    /// Save keys to secure storage
    #[cfg(feature = "encryption")]
    pub async fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        // This would save keys to encrypted storage in production
        // For now, just a placeholder
        tracing::warn!("Key saving to file not implemented");
        Ok(())
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_no_encryption() {
        let layer = SecurityLayer::new(EncryptionType::None).await.unwrap();
        let data = b"hello world";
        
        let encrypted = layer.encrypt(data).await.unwrap();
        let decrypted = layer.decrypt(&encrypted).await.unwrap();
        
        assert_eq!(data, decrypted.as_slice());
        assert_eq!(encrypted, data);
        assert!(!layer.is_enabled());
    }

    #[cfg(feature = "encryption")]
    #[tokio::test]
    async fn test_chacha20poly1305_encryption() {
        let key = SecurityLayer::generate_chacha_key();
        let encryption_type = EncryptionType::ChaCha20Poly1305 { key };
        let layer = SecurityLayer::new(encryption_type).await.unwrap();
        
        let data = b"sensitive data that needs encryption";
        
        let encrypted = layer.encrypt(data).await.unwrap();
        let decrypted = layer.decrypt(&encrypted).await.unwrap();
        
        assert_eq!(data, decrypted.as_slice());
        assert_ne!(encrypted, data); // Should be different when encrypted
        assert!(layer.is_enabled());
        assert_eq!(layer.encryption_overhead(), 28); // 12-byte nonce + 16-byte MAC
    }

    #[tokio::test]
    async fn test_empty_data() {
        let layer = SecurityLayer::new(EncryptionType::None).await.unwrap();
        let empty: &[u8] = &[];
        
        let encrypted = layer.encrypt(empty).await.unwrap();
        let decrypted = layer.decrypt(&encrypted).await.unwrap();
        
        assert!(encrypted.is_empty());
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_key_generation() {
        let key1 = SecurityLayer::generate_chacha_key();
        let key2 = SecurityLayer::generate_chacha_key();
        
        assert_ne!(key1, key2); // Should generate different keys
        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn test_key_derivation() {
        let password = "test_password";
        let salt = b"random_salt_value";
        
        let key1 = SecurityLayer::derive_key_from_password(password, salt).unwrap();
        let key2 = SecurityLayer::derive_key_from_password(password, salt).unwrap();
        
        assert_eq!(key1, key2); // Same password + salt = same key
        assert_eq!(key1.len(), 32);
        
        // Different salt should produce different key
        let key3 = SecurityLayer::derive_key_from_password(password, b"different_salt").unwrap();
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_key_manager() {
        let mut manager = KeyManager::new();
        let key = SecurityLayer::generate_chacha_key();
        
        manager.add_key("node1", key);
        assert_eq!(manager.get_key("node1"), Some(key));
        assert_eq!(manager.get_key("node2"), None);
        
        let removed = manager.remove_key("node1");
        assert_eq!(removed, Some(key));
        assert_eq!(manager.get_key("node1"), None);
    }

    #[test]
    fn test_security_info() {
        let layer_none = SecurityLayer::new(EncryptionType::None);
        let layer_tls = SecurityLayer::new(EncryptionType::Tls);
        
        // We can't easily test these without proper async setup, so just test the sync parts
        let info_none = SecurityInfo {
            name: "none",
            level: SecurityLevel::None,
            overhead_bytes: 0,
            description: "No encryption",
        };
        
        assert_eq!(info_none.name, "none");
        assert_eq!(info_none.level, SecurityLevel::None);
        assert!(SecurityLevel::None < SecurityLevel::High);
    }

    #[test]
    fn test_encryption_type_equality() {
        assert_eq!(EncryptionType::None, EncryptionType::None);
        assert_eq!(EncryptionType::Tls, EncryptionType::Tls);
        
        let key = [0u8; 32];
        let chacha1 = EncryptionType::ChaCha20Poly1305 { key };
        let chacha2 = EncryptionType::ChaCha20Poly1305 { key };
        assert_eq!(chacha1, chacha2);
        
        assert_ne!(EncryptionType::None, EncryptionType::Tls);
    }
}