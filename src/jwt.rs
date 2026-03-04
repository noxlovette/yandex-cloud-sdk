use crate::SDKError;
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::{
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

struct AuthorisedKey {
    id: String,
    service_account_id: String,
    created_at: DateTime<Utc>,
    key_algoruthm: String,
    public_key: String,
    private_key: String,
}

impl AuthorisedKey {
    pub fn encoding(&self) -> EncodingKey {
        EncodingKey::from_rsa_pem(
            self.private_key
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
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

        let exp = iat + chrono::Duration::minutes(30);

        Self {
            exp: exp,
            iat: iat,
            aud: aud.to_owned(),
            iss: KEY.service_account_id,
        }
    }

    pub fn encode(&self) -> Result<String, SDKError> {
        let mut header = Header::new(Algorithm::PS256);
        header.kid = Some(KEY.id);

        encode(&header, &self, &KEY.encoding()).map_err(|e| {
            tracing::error!("Token generation failed: {e:?}");

            SDKError::Internal("JWT TOKEN GENERATION FAILED".to_string())
        })
    }
}

pub static KEY: LazyLock<AuthorisedKey> = LazyLock::new(|| {
    let file = std::fs::read("authorized_key.json").expect("did not find authorized_key.json");

    serde_json::from_slice::<AuthorisedKey>(&file).expect("failed to read json")
});

#[derive(Deserialize, Serialize)]
pub struct Claims {
    aud: Url,
    iss: String,
    exp: u64,
    iat: u64,
}
