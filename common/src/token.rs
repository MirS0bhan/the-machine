use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

/// Grant token for capability checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantToken {
    pub token_id: Uuid,
    pub issued_at: u64,
    pub expires_at: u64,
    pub scope: GrantScope,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantScope {
    pub method: String,
    pub request_hash: String,
    pub requester_identity: String,
}

/// Initialize the process-wide token secret.
///
/// Placeholder: in a real system this would derive or load a persistent
/// secret used to sign and verify grant tokens.
pub fn init_token_secret() {
    // No-op for now; secret is generated per-verifier.
}

pub struct TokenVerifier {
    secret: [u8; 64],
}

impl TokenVerifier {
    pub fn new(secret: [u8; 64]) -> Self {
        Self { secret }
    }

    pub fn verify(&self, token: &GrantToken) -> bool {
        // Simplified: just check if signature matches a HMAC of the token data
        let data = format!(
            "{}{}{}{}",
            token.token_id, token.issued_at, token.expires_at, token.scope.method
        );
        let mut hasher = Hmac::<Sha256>::new_from_slice(&self.secret).unwrap();
        hasher.update(data.as_bytes());
        let result = hasher.finalize().into_bytes();
        result.as_slice() == token.signature.as_slice()
    }

    pub fn issue_token(&self, scope: GrantScope, ttl_seconds: u64) -> GrantToken {
        let token_id = Uuid::new_v4();
        let issued_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at = issued_at + ttl_seconds;
        let data = format!("{}{}{}{}", token_id, issued_at, expires_at, scope.method);
        let mut hasher = Hmac::<Sha256>::new_from_slice(&self.secret).unwrap();
        hasher.update(data.as_bytes());
        let result = hasher.finalize().into_bytes();
        let mut sig = vec![0u8; 64];
        sig[..32].copy_from_slice(&result);
        sig[32..].copy_from_slice(&[0u8; 32]); // placeholder for second half
        GrantToken {
            token_id,
            issued_at,
            expires_at,
            scope,
            signature: sig,
        }
    }
}
