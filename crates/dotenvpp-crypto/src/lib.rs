//! DotenvPP encryption support.
//!
//! The encrypted file format stores one random data key wrapped to one or more
//! X25519 recipients. Each variable value is encrypted separately with
//! AES-256-GCM and authenticated with the variable key as associated data.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use dotenvpp_parser::EnvPair;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(all(feature = "crypto-crabgraph", feature = "crypto-rustcrypto"))]
compile_error!("enable exactly one crypto backend: `crypto-crabgraph` or `crypto-rustcrypto`");

#[cfg(not(any(feature = "crypto-crabgraph", feature = "crypto-rustcrypto")))]
compile_error!("a crypto backend feature is required");

#[cfg(feature = "crypto-crabgraph")]
mod backend {
    use super::{EncryptedBlob, SecretBytes};
    use crabgraph::aead::{AesGcm256, Ciphertext, CrabAead};
    use crabgraph::asym::{X25519KeyPair, X25519PublicKey};

    pub fn generate_keypair() -> Result<KeyMaterial, String> {
        let keypair = X25519KeyPair::generate().map_err(|err| err.to_string())?;
        Ok(KeyMaterial {
            public: keypair.public_key().as_bytes().to_vec(),
            private: SecretBytes::new(keypair.secret_bytes().to_vec()),
        })
    }

    pub fn public_from_private(private: &[u8]) -> Result<Vec<u8>, String> {
        let keypair = X25519KeyPair::from_secret_bytes(private).map_err(|err| err.to_string())?;
        Ok(keypair.public_key().as_bytes().to_vec())
    }

    pub fn derive_wrap_key(
        private: &[u8],
        public: &[u8],
        info: &[u8],
    ) -> Result<SecretBytes, String> {
        let keypair = X25519KeyPair::from_secret_bytes(private).map_err(|err| err.to_string())?;
        let public = X25519PublicKey::from_bytes(public).map_err(|err| err.to_string())?;
        let shared = keypair.diffie_hellman(&public).map_err(|err| err.to_string())?;
        let key = shared.derive_key(info, 32).map_err(|err| err.to_string())?;
        Ok(SecretBytes::new(key.as_slice().to_vec()))
    }

    pub fn generate_data_key() -> Result<SecretBytes, String> {
        AesGcm256::generate_key().map(SecretBytes::new).map_err(|err| err.to_string())
    }

    pub fn seal(key: &[u8], plaintext: &[u8], aad: &[u8]) -> Result<EncryptedBlob, String> {
        let cipher = AesGcm256::new(key).map_err(|err| err.to_string())?;
        let ciphertext = cipher.encrypt(plaintext, Some(aad)).map_err(|err| err.to_string())?;
        Ok(EncryptedBlob {
            nonce: ciphertext.nonce,
            ciphertext: ciphertext.ciphertext,
            tag: ciphertext.tag,
        })
    }

    pub fn open(key: &[u8], blob: &EncryptedBlob, aad: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = AesGcm256::new(key).map_err(|err| err.to_string())?;
        let ciphertext =
            Ciphertext::new(blob.nonce.clone(), blob.ciphertext.clone(), blob.tag.clone());
        cipher.decrypt(&ciphertext, Some(aad)).map_err(|err| err.to_string())
    }

    pub struct KeyMaterial {
        pub public: Vec<u8>,
        pub private: SecretBytes,
    }
}

#[cfg(all(not(feature = "crypto-crabgraph"), feature = "crypto-rustcrypto"))]
mod backend {
    use super::{EncryptedBlob, SecretBytes};
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::{Aes256Gcm, Nonce};
    use hkdf::Hkdf;
    use rand_core::{OsRng, RngCore};
    use sha2::Sha256;
    use x25519_dalek::{PublicKey, StaticSecret};

    pub fn generate_keypair() -> Result<KeyMaterial, String> {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Ok(KeyMaterial {
            public: public.as_bytes().to_vec(),
            private: SecretBytes::new(secret.to_bytes().to_vec()),
        })
    }

    pub fn public_from_private(private: &[u8]) -> Result<Vec<u8>, String> {
        let private = secret_from_slice(private)?;
        Ok(PublicKey::from(&private).as_bytes().to_vec())
    }

    pub fn derive_wrap_key(
        private: &[u8],
        public: &[u8],
        info: &[u8],
    ) -> Result<SecretBytes, String> {
        let private = secret_from_slice(private)?;
        let public = public_from_slice(public)?;
        let shared = private.diffie_hellman(&public);
        let hkdf = Hkdf::<Sha256>::new(None, shared.as_bytes());
        let mut output = vec![0u8; 32];
        hkdf.expand(info, &mut output)
            .map_err(|err| format!("HKDF expand failed: {err}"))?;
        Ok(SecretBytes::new(output))
    }

    pub fn generate_data_key() -> Result<SecretBytes, String> {
        let mut key = vec![0u8; 32];
        OsRng.fill_bytes(&mut key);
        Ok(SecretBytes::new(key))
    }

    pub fn seal(key: &[u8], plaintext: &[u8], aad: &[u8]) -> Result<EncryptedBlob, String> {
        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|err| format!("invalid AES key: {err}"))?;
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let encrypted = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|err| format!("AES-GCM encrypt failed: {err}"))?;
        let tag_start = encrypted.len().saturating_sub(16);
        Ok(EncryptedBlob {
            nonce: nonce.to_vec(),
            ciphertext: encrypted[..tag_start].to_vec(),
            tag: encrypted[tag_start..].to_vec(),
        })
    }

    pub fn open(key: &[u8], blob: &EncryptedBlob, aad: &[u8]) -> Result<Vec<u8>, String> {
        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|err| format!("invalid AES key: {err}"))?;
        if blob.nonce.len() != 12 || blob.tag.len() != 16 {
            return Err("invalid AES-GCM nonce or tag length".to_owned());
        }
        let mut encrypted = blob.ciphertext.clone();
        encrypted.extend_from_slice(&blob.tag);
        cipher
            .decrypt(
                Nonce::from_slice(&blob.nonce),
                Payload {
                    msg: &encrypted,
                    aad,
                },
            )
            .map_err(|err| format!("AES-GCM decrypt failed: {err}"))
    }

    fn secret_from_slice(bytes: &[u8]) -> Result<StaticSecret, String> {
        if bytes.len() != 32 {
            return Err(format!("X25519 private key must be 32 bytes, got {}", bytes.len()));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Ok(StaticSecret::from(array))
    }

    fn public_from_slice(bytes: &[u8]) -> Result<PublicKey, String> {
        if bytes.len() != 32 {
            return Err(format!("X25519 public key must be 32 bytes, got {}", bytes.len()));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Ok(PublicKey::from(array))
    }

    pub struct KeyMaterial {
        pub public: Vec<u8>,
        pub private: SecretBytes,
    }
}

const FORMAT: &str = "dotenvpp.enc.v1";
const ALGORITHM: &str = "X25519-HKDF-SHA256+Aes256Gcm";
const WRAP_INFO: &[u8] = b"dotenvpp-v1 data key wrap";

/// Crypto operation errors.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Backend-specific crypto failure.
    #[error("crypto backend error: {0}")]
    Backend(String),
    /// Base64 decoding failed.
    #[error("base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),
    /// JSON serialization or deserialization failed.
    #[error("encrypted file JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Encrypted file format is invalid.
    #[error("invalid encrypted file: {0}")]
    Format(String),
    /// Decrypted plaintext is not UTF-8.
    #[error("decrypted value is not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Secret bytes zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    /// Wrap bytes as secret material.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Borrow secret material.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretBytes([REDACTED {} bytes])", self.0.len())
    }
}

/// Public/private X25519 keypair encoded as base64 strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyPair {
    /// Recipient/public key.
    pub public_key: String,
    /// Private key. Store outside git and pass as `DOTENV_PRIVATE_KEY`.
    pub private_key: String,
}

/// Serialized encrypted file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedEnvFile {
    /// Format version.
    pub format: String,
    /// Algorithm suite.
    pub algorithm: String,
    /// Wrapped data keys.
    pub recipients: Vec<EncryptedRecipient>,
    /// Per-variable encrypted values.
    pub variables: Vec<EncryptedVariable>,
}

/// One recipient's encrypted data key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedRecipient {
    /// Recipient public key in base64.
    pub recipient_public_key: String,
    /// Ephemeral public key in base64.
    pub ephemeral_public_key: String,
    /// AES-GCM encrypted data key in base64.
    pub wrapped_key: String,
}

/// One encrypted variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedVariable {
    /// Variable key.
    pub key: String,
    /// AES-GCM encrypted value in base64.
    pub value: String,
    /// Original source line if known.
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EncryptedBlob {
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
    tag: Vec<u8>,
}

impl EncryptedBlob {
    fn to_base64(&self) -> String {
        let mut bytes =
            Vec::with_capacity(self.nonce.len() + self.ciphertext.len() + self.tag.len());
        bytes.extend_from_slice(&self.nonce);
        bytes.extend_from_slice(&self.ciphertext);
        bytes.extend_from_slice(&self.tag);
        BASE64.encode(bytes)
    }

    fn from_base64(value: &str) -> Result<Self, CryptoError> {
        let bytes = BASE64.decode(value)?;
        if bytes.len() < 28 {
            return Err(CryptoError::Format(
                "encrypted blob is shorter than nonce plus tag".to_owned(),
            ));
        }
        Ok(Self {
            nonce: bytes[..12].to_vec(),
            ciphertext: bytes[12..bytes.len() - 16].to_vec(),
            tag: bytes[bytes.len() - 16..].to_vec(),
        })
    }
}

/// Generate a new X25519 keypair for DotenvPP encryption.
pub fn keygen() -> Result<KeyPair, CryptoError> {
    let keypair = backend::generate_keypair().map_err(CryptoError::Backend)?;
    Ok(KeyPair {
        public_key: BASE64.encode(keypair.public),
        private_key: BASE64.encode(keypair.private.as_slice()),
    })
}

/// Encrypt env pairs for one or more recipient public keys.
pub fn encrypt_pairs_for_recipients(
    pairs: &[EnvPair],
    recipient_public_keys: &[String],
) -> Result<EncryptedEnvFile, CryptoError> {
    if recipient_public_keys.is_empty() {
        return Err(CryptoError::Format(
            "at least one recipient public key is required".to_owned(),
        ));
    }

    let data_key = backend::generate_data_key().map_err(CryptoError::Backend)?;
    let mut recipients = Vec::with_capacity(recipient_public_keys.len());

    for recipient_public_key in recipient_public_keys {
        let recipient_public = BASE64.decode(recipient_public_key)?;
        let ephemeral = backend::generate_keypair().map_err(CryptoError::Backend)?;
        let wrap_key =
            backend::derive_wrap_key(ephemeral.private.as_slice(), &recipient_public, WRAP_INFO)
                .map_err(CryptoError::Backend)?;
        let wrapped = backend::seal(
            wrap_key.as_slice(),
            data_key.as_slice(),
            recipient_public_key.as_bytes(),
        )
        .map_err(CryptoError::Backend)?;

        recipients.push(EncryptedRecipient {
            recipient_public_key: recipient_public_key.clone(),
            ephemeral_public_key: BASE64.encode(ephemeral.public),
            wrapped_key: wrapped.to_base64(),
        });
    }

    let mut variables = Vec::with_capacity(pairs.len());
    for pair in pairs {
        let blob = backend::seal(data_key.as_slice(), pair.value.as_bytes(), pair.key.as_bytes())
            .map_err(CryptoError::Backend)?;
        variables.push(EncryptedVariable {
            key: pair.key.clone(),
            value: blob.to_base64(),
            line: pair.line,
        });
    }

    Ok(EncryptedEnvFile {
        format: FORMAT.to_owned(),
        algorithm: ALGORITHM.to_owned(),
        recipients,
        variables,
    })
}

/// Encrypt env pairs and return pretty JSON.
pub fn encrypt_pairs_to_string(
    pairs: &[EnvPair],
    recipient_public_keys: &[String],
) -> Result<String, CryptoError> {
    let file = encrypt_pairs_for_recipients(pairs, recipient_public_keys)?;
    serde_json::to_string_pretty(&file).map_err(CryptoError::from)
}

/// Decrypt an encrypted file JSON string.
pub fn decrypt_str(input: &str, private_key: &str) -> Result<Vec<EnvPair>, CryptoError> {
    let file: EncryptedEnvFile = serde_json::from_str(input)?;
    decrypt_file(&file, private_key)
}

/// Decrypt a parsed encrypted file.
pub fn decrypt_file(
    file: &EncryptedEnvFile,
    private_key: &str,
) -> Result<Vec<EnvPair>, CryptoError> {
    validate_file(file)?;
    let private = SecretBytes::new(BASE64.decode(private_key)?);
    let public = backend::public_from_private(private.as_slice()).map_err(CryptoError::Backend)?;
    let public_b64 = BASE64.encode(&public);

    let recipient = file
        .recipients
        .iter()
        .find(|recipient| recipient.recipient_public_key == public_b64)
        .ok_or_else(|| {
            CryptoError::Format("no recipient matches the provided private key".to_owned())
        })?;

    let ephemeral_public = BASE64.decode(&recipient.ephemeral_public_key)?;
    let wrap_key = backend::derive_wrap_key(private.as_slice(), &ephemeral_public, WRAP_INFO)
        .map_err(CryptoError::Backend)?;
    let wrapped = EncryptedBlob::from_base64(&recipient.wrapped_key)?;
    let data_key = SecretBytes::new(
        backend::open(wrap_key.as_slice(), &wrapped, recipient.recipient_public_key.as_bytes())
            .map_err(CryptoError::Backend)?,
    );

    let mut pairs = Vec::with_capacity(file.variables.len());
    for variable in &file.variables {
        let blob = EncryptedBlob::from_base64(&variable.value)?;
        let plaintext = backend::open(data_key.as_slice(), &blob, variable.key.as_bytes())
            .map_err(CryptoError::Backend)?;
        pairs.push(EnvPair {
            key: variable.key.clone(),
            value: String::from_utf8(plaintext)?,
            line: variable.line,
        });
    }

    Ok(pairs)
}

/// Rotate an encrypted file to a new recipient set.
pub fn rotate_str(
    input: &str,
    private_key: &str,
    new_recipient_public_keys: &[String],
) -> Result<String, CryptoError> {
    let pairs = decrypt_str(input, private_key)?;
    encrypt_pairs_to_string(&pairs, new_recipient_public_keys)
}

fn validate_file(file: &EncryptedEnvFile) -> Result<(), CryptoError> {
    if file.format != FORMAT {
        return Err(CryptoError::Format(format!("unsupported format `{}`", file.format)));
    }
    if file.algorithm != ALGORITHM {
        return Err(CryptoError::Format(format!("unsupported algorithm `{}`", file.algorithm)));
    }
    if file.recipients.is_empty() {
        return Err(CryptoError::Format("encrypted file has no recipients".to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn pairs() -> Vec<EnvPair> {
        vec![
            EnvPair {
                key: "DATABASE_URL".to_owned(),
                value: "postgres://db/app".to_owned(),
                line: 1,
            },
            EnvPair {
                key: "API_KEY".to_owned(),
                value: "secret-value".to_owned(),
                line: 2,
            },
        ]
    }

    #[test]
    fn encrypts_and_decrypts_roundtrip() {
        let keypair = keygen().unwrap();
        let encrypted =
            encrypt_pairs_for_recipients(&pairs(), std::slice::from_ref(&keypair.public_key))
                .unwrap();
        assert_eq!(encrypted.variables.len(), 2);

        let decrypted = decrypt_file(&encrypted, &keypair.private_key).unwrap();
        assert_eq!(decrypted, pairs());
    }

    #[test]
    fn supports_multiple_recipients() {
        let alice = keygen().unwrap();
        let bob = keygen().unwrap();
        let encrypted = encrypt_pairs_for_recipients(
            &pairs(),
            &[alice.public_key.clone(), bob.public_key.clone()],
        )
        .unwrap();

        assert_eq!(decrypt_file(&encrypted, &alice.private_key).unwrap(), pairs());
        assert_eq!(decrypt_file(&encrypted, &bob.private_key).unwrap(), pairs());
    }

    #[test]
    fn rejects_tampered_ciphertext() {
        let keypair = keygen().unwrap();
        let mut encrypted =
            encrypt_pairs_for_recipients(&pairs(), std::slice::from_ref(&keypair.public_key))
                .unwrap();
        encrypted.variables[0].key = "OTHER_KEY".to_owned();

        assert!(decrypt_file(&encrypted, &keypair.private_key).is_err());
    }

    #[test]
    fn rotates_to_new_recipient() {
        let old = keygen().unwrap();
        let new = keygen().unwrap();
        let encrypted =
            encrypt_pairs_to_string(&pairs(), std::slice::from_ref(&old.public_key)).unwrap();
        let rotated =
            rotate_str(&encrypted, &old.private_key, std::slice::from_ref(&new.public_key))
                .unwrap();

        assert!(decrypt_str(&rotated, &old.private_key).is_err());
        assert_eq!(decrypt_str(&rotated, &new.private_key).unwrap(), pairs());
    }
}
