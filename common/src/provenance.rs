use serde::{Deserialize, Serialize};
use hmac::{Hmac, Mac};
use sha2::Sha256;

const HMAC_SECRET_SIZE: usize = 32;

/// Provenance marker for input events
#[derive(Debug, Serialize, Deserialize)]
pub struct ProvenanceMarker {
    pub kernel_timestamp: u64,
    pub device_major: u32,
    pub device_minor: u32,
    pub sequence: u64,
    pub hmac: Vec<u8>,
}

pub struct ProvenanceVerifier {
    secret: [u8; HMAC_SECRET_SIZE],
}

impl ProvenanceVerifier {
    pub fn new(secret: [u8; HMAC_SECRET_SIZE]) -> Self {
        Self { secret }
    }

    pub fn verify(&self, marker: &ProvenanceMarker) -> bool {
        let mut hasher = Hmac::<Sha256>::new_from_slice(&self.secret).unwrap();
        hasher.update(&marker.kernel_timestamp.to_be_bytes());
        hasher.update(&marker.device_major.to_be_bytes());
        hasher.update(&marker.device_minor.to_be_bytes());
        hasher.update(&marker.sequence.to_be_bytes());
        let result = hasher.finalize().into_bytes();
        result.as_slice() == marker.hmac.as_slice()
    }

    pub fn generate_marker(
        &self,
        kernel_timestamp: u64,
        device_major: u32,
        device_minor: u32,
        sequence: u64,
    ) -> ProvenanceMarker {
        let mut hasher = Hmac::<Sha256>::new_from_slice(&self.secret).unwrap();
        hasher.update(&kernel_timestamp.to_be_bytes());
        hasher.update(&device_major.to_be_bytes());
        hasher.update(&device_minor.to_be_bytes());
        hasher.update(&sequence.to_be_bytes());
        let hmac_bytes = hasher.finalize().into_bytes();
        let hmac_array = hmac_bytes.to_vec();
        ProvenanceMarker {
            kernel_timestamp,
            device_major,
            device_minor,
            sequence,
            hmac: hmac_array,
        }
    }
}
