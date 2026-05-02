use crate::SDKError;
use base64::prelude::*;
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::{
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

fn decode_authorized_key_base64(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    BASE64_STANDARD.decode(encoded)
}

/// Service account authorized key payload.
#[derive(Deserialize)]
pub struct AuthorisedKey {
    /// Key identifier.
    pub id: String,
    /// Service account identifier.
    pub service_account_id: String,
    /// Key creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Key algorithm name.
    pub key_algorithm: String,
    /// Public key in PEM form.
    pub public_key: String,
    /// Private key in PEM form.
    pub private_key: String,
}

impl AuthorisedKey {
    /// Builds JWT signing key from private key material.
    pub fn encoding(&self) -> EncodingKey {
        EncodingKey::from_rsa_pem(
            self.private_key
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n")
                .as_bytes(),
        )
        .expect("failed to read encoding key")
    }

    /// Builds JWT decoding key from public key material.
    pub fn decoding(&self) -> DecodingKey {
        DecodingKey::from_rsa_pem(self.public_key.as_bytes()).expect("failed to read decoding key")
    }
}

impl Claims {
    /// Creates JWT claims for given audience.
    pub fn new(aud: &url::Url) -> Self {
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let exp = iat + (chrono::Duration::minutes(30).as_seconds_f64()) as u64;

        Self {
            exp: exp,
            iat: iat,
            aud: aud.to_owned(),
            iss: KEY.service_account_id.clone(),
        }
    }

    /// Encodes claims into signed JWT.
    pub fn encode(&self) -> Result<String, SDKError> {
        let mut header = Header::new(Algorithm::PS256);
        header.kid = Some(KEY.id.clone());

        encode(&header, &self, &KEY.encoding()).map_err(|e| {
            tracing::error!("Token generation failed: {e:?}");

            SDKError::Internal("JWT TOKEN GENERATION FAILED".to_string())
        })
    }
}

/// Lazily loaded service account key used for JWT signing.
pub static KEY: LazyLock<AuthorisedKey> = LazyLock::new(|| {
    let file = {
        {
            let encoded = std::env::var("YANDEX_AUTHORIZED_KEY")
                .expect("did not find YANDEX_AUTHORIZED_KEY env variable");

            decode_authorized_key_base64(&encoded)
                .expect("failed to decode YANDEX_AUTHORIZED_KEY as base64")
        }
    };

    serde_json::from_slice::<AuthorisedKey>(&file)
        .expect("failed to deserialize AuthorisedKey from json")
});

/// IAM JWT claims payload.
#[derive(Deserialize, Serialize)]
pub struct Claims {
    aud: Url,
    iss: String,
    exp: u64,
    iat: u64,
}

#[cfg(test)]
mod tests {
    use super::decode_authorized_key_base64;
    use base64::prelude::*;

    #[test]
    fn decodes_authorized_key_json_from_base64() {
        let authorized_key = br#"{"id":"test-key"}"#;
        let encoded = BASE64_STANDARD.encode(authorized_key);

        let decoded = decode_authorized_key_base64(&encoded).unwrap();

        assert_eq!(decoded, authorized_key);
    }

    #[test]
    fn fails_to_decode_invalid_authorized_key_base64() {
        let err = decode_authorized_key_base64("not-base64!").unwrap_err();

        assert_eq!(err, base64::DecodeError::InvalidByte(3, b'-'));
    }
}
