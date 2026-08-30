use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Grant token for capability checks.
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

/// ISO / test fallback material. Override with `THE_MACHINE_TOKEN_SECRET`
/// or a 0600 file at `/run/the-machine/secrets/token`.
const DEFAULT_TOKEN_MATERIAL: &[u8] = b"the-machine-grant-token-v1";

/// Load the shared HMAC key used by the broker (issue) and system-daemon (verify).
pub fn load_token_secret() -> [u8; 64] {
    if let Ok(s) = std::env::var("THE_MACHINE_TOKEN_SECRET") {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return derive_secret(trimmed.as_bytes());
        }
    }
    if let Ok(path) = std::env::var("THE_MACHINE_TOKEN_SECRET_FILE") {
        if let Ok(bytes) = std::fs::read(&path) {
            if !bytes.is_empty() {
                return derive_secret(&bytes);
            }
        }
    }
    for path in [
        "/run/the-machine/secrets/token",
        "/etc/the-machine/secrets/token",
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            if !bytes.is_empty() {
                return derive_secret(&bytes);
            }
        }
    }
    derive_secret(DEFAULT_TOKEN_MATERIAL)
}

fn derive_secret(input: &[u8]) -> [u8; 64] {
    let mut out = [0u8; 64];
    let h1 = Sha256::digest(input);
    let mut second = Vec::with_capacity(input.len() + 7);
    second.extend_from_slice(input);
    second.extend_from_slice(b":expand");
    let h2 = Sha256::digest(&second);
    out[..32].copy_from_slice(&h1);
    out[32..].copy_from_slice(&h2);
    out
}

/// Shared verifier used by policy-broker and system-daemon.
pub fn shared_verifier() -> TokenVerifier {
    TokenVerifier::new(load_token_secret())
}

/// Parse a grant token from either a JSON object or a serialized JSON string.
pub fn parse_grant_token(value: &serde_json::Value) -> Option<GrantToken> {
    if let Some(s) = value.as_str() {
        return serde_json::from_str(s).ok();
    }
    serde_json::from_value(value.clone()).ok()
}

/// Extract and verify a grant token for `method` from MCP params.
pub fn require_grant(
    params: Option<&serde_json::Value>,
    method: &str,
) -> Result<GrantToken, String> {
    let params = params.ok_or_else(|| "grant token required".to_string())?;
    let token_val = params
        .get("token")
        .or_else(|| params.get("grant_token"))
        .ok_or_else(|| "grant token required".to_string())?;
    let token = parse_grant_token(token_val).ok_or_else(|| "grant token malformed".to_string())?;
    if !shared_verifier().verify_method(&token, method) {
        return Err("grant token invalid, expired, or scope mismatch".into());
    }
    Ok(token)
}

/// Kept for call-site compatibility; secret is resolved per verifier.
pub fn init_token_secret() {}

pub struct TokenVerifier {
    secret: [u8; 64],
}

impl TokenVerifier {
    pub fn new(secret: [u8; 64]) -> Self {
        Self { secret }
    }

    fn signing_bytes(token_id: &Uuid, issued_at: u64, expires_at: u64, method: &str) -> Vec<u8> {
        format!("{token_id}{issued_at}{expires_at}{method}").into_bytes()
    }

    fn sign(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher =
            Hmac::<Sha256>::new_from_slice(&self.secret).expect("HMAC-SHA256 accepts 64-byte keys");
        hasher.update(data);
        hasher.finalize().into_bytes().to_vec()
    }

    pub fn verify(&self, token: &GrantToken) -> bool {
        if token.expires_at < crate::current_timestamp() {
            return false;
        }
        let data = Self::signing_bytes(
            &token.token_id,
            token.issued_at,
            token.expires_at,
            &token.scope.method,
        );
        let expected = self.sign(&data);
        expected.as_slice() == token.signature.as_slice()
    }

    pub fn verify_method(&self, token: &GrantToken, method: &str) -> bool {
        self.verify(token) && token.scope.method == method
    }

    pub fn issue_token(&self, scope: GrantScope, ttl_seconds: u64) -> GrantToken {
        let token_id = Uuid::new_v4();
        let issued_at = crate::current_timestamp();
        let expires_at = issued_at + ttl_seconds;
        let data = Self::signing_bytes(&token_id, issued_at, expires_at, &scope.method);
        GrantToken {
            token_id,
            issued_at,
            expires_at,
            scope,
            signature: self.sign(&data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_verify_round_trip() {
        let v = TokenVerifier::new([7u8; 64]);
        let token = v.issue_token(
            GrantScope {
                method: "power.set_profile".into(),
                request_hash: "x".into(),
                requester_identity: "test".into(),
            },
            60,
        );
        assert!(v.verify(&token));
        assert!(v.verify_method(&token, "power.set_profile"));
        assert!(!v.verify_method(&token, "display.set_mode"));
    }

    #[test]
    fn rejects_expired_token() {
        let v = TokenVerifier::new([7u8; 64]);
        let mut token = v.issue_token(
            GrantScope {
                method: "power.set_profile".into(),
                request_hash: "x".into(),
                requester_identity: "test".into(),
            },
            60,
        );
        token.expires_at = 1;
        assert!(!v.verify(&token));
    }

    #[test]
    fn rejects_tampered_signature() {
        let v = TokenVerifier::new([7u8; 64]);
        let mut token = v.issue_token(
            GrantScope {
                method: "power.set_profile".into(),
                request_hash: "x".into(),
                requester_identity: "test".into(),
            },
            60,
        );
        token.signature[0] ^= 0xff;
        assert!(!v.verify(&token));
    }

    #[test]
    fn parse_token_from_json_string_and_object() {
        let v = shared_verifier();
        let token = v.issue_token(
            GrantScope {
                method: "net.connect_wifi".into(),
                request_hash: "h".into(),
                requester_identity: "agent".into(),
            },
            30,
        );
        let as_str = serde_json::to_string(&token).unwrap();
        let parsed = parse_grant_token(&serde_json::Value::String(as_str)).unwrap();
        assert_eq!(parsed.token_id, token.token_id);
        let obj = serde_json::to_value(&token).unwrap();
        assert!(parse_grant_token(&obj).is_some());
    }

    #[test]
    fn require_grant_checks_scope() {
        let token = shared_verifier().issue_token(
            GrantScope {
                method: "power.set_profile".into(),
                request_hash: "h".into(),
                requester_identity: "agent".into(),
            },
            60,
        );
        let params = serde_json::json!({ "token": token, "profile": "powersave" });
        assert!(require_grant(Some(&params), "power.set_profile").is_ok());
        assert!(require_grant(Some(&params), "audio.set_default").is_err());
        assert!(require_grant(None, "power.set_profile").is_err());
    }
}
