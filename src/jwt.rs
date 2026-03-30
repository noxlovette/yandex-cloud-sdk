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

fn decode_authorized_key_base64(
    encoded: &str,
) -> Result<Vec<u8>, base64::DecodeError> {
    BASE64_STANDARD.decode(encoded)
}

#[derive(Deserialize)]
pub struct AuthorisedKey {
    pub id: String,
    pub service_account_id: String,
    pub created_at: DateTime<Utc>,
    pub key_algorithm: String,
    pub public_key: String,
    pub private_key: String,
}

impl AuthorisedKey {
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
    pub fn decoding(&self) -> DecodingKey {
        DecodingKey::from_rsa_pem(self.public_key.as_bytes()).expect("failed to read decoding key")
    }
}

impl Claims {
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

    pub fn encode(&self) -> Result<String, SDKError> {
        let mut header = Header::new(Algorithm::PS256);
        header.kid = Some(KEY.id.clone());

        encode(&header, &self, &KEY.encoding()).map_err(|e| {
            tracing::error!("Token generation failed: {e:?}");

            SDKError::Internal("JWT TOKEN GENERATION FAILED".to_string())
        })
    }
}
pub static KEY: LazyLock<AuthorisedKey> = LazyLock::new(|| {
    let file = {
        #[cfg(debug_assertions)]
        {
            std::fs::read("authorized_key.json").expect("did not find authorized_key.json")
        }

        #[cfg(not(debug_assertions))]
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
