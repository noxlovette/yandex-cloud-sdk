use crate::SDKError;
use base64::prelude::*;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

/// Env var read by [`AuthorizedKey::from_env`].
pub(crate) const AUTHORIZED_KEY_ENV: &str = "YANDEX_AUTHORIZED_KEY";

/// How long the JWTs exchanged for IAM tokens stay valid.
const JWT_LIFETIME_SECS: u64 = 30 * 60;

/// The JSON layout of a key file from `yc iam key create`.
#[derive(Deserialize)]
struct RawAuthorizedKey {
    id: String,
    service_account_id: String,
    private_key: String,
}

/// Service account authorized key, used to sign the JWTs that are exchanged for
/// IAM tokens.
///
/// Get one from `yc iam key create` or the Yandex Cloud console, then load it
/// with [`from_json`](Self::from_json), [`from_base64`](Self::from_base64) or
/// [`from_env`](Self::from_env).
#[derive(Clone)]
pub struct AuthorizedKey {
    id: String,
    service_account_id: String,
    encoding: EncodingKey,
}

impl fmt::Debug for AuthorizedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthorizedKey")
            .field("id", &self.id)
            .field("service_account_id", &self.service_account_id)
            .finish_non_exhaustive()
    }
}

impl AuthorizedKey {
    /// Parses the JSON authorized key file as produced by `yc iam key create`.
    pub fn from_json(json: impl AsRef<[u8]>) -> Result<Self, SDKError> {
        let raw: RawAuthorizedKey = serde_json::from_slice(json.as_ref())
            .map_err(|e| {
                SDKError::Config(format!("invalid authorized key JSON: {e}"))
            })?;

        // The file's private key starts with a "PLEASE DO NOT REMOVE THIS
        // LINE!" line ahead of the actual PEM block, which the PEM
        // parser rejects.
        let pem = raw
            .private_key
            .find("-----BEGIN")
            .map_or(raw.private_key.as_str(), |start| {
                &raw.private_key[start..]
            });

        let encoding =
            EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| {
                SDKError::Config(format!(
                    "invalid authorized key private key: {e}"
                ))
            })?;

        Ok(Self {
            id: raw.id,
            service_account_id: raw.service_account_id,
            encoding,
        })
    }

    /// Parses a base64-encoded authorized key JSON, e.g. `base64 -i
    /// authorized_key.json`.
    pub fn from_base64(encoded: &str) -> Result<Self, SDKError> {
        let json =
            decode_authorized_key_base64(encoded.trim()).map_err(|e| {
                SDKError::Config(format!(
                    "authorized key is not valid base64: {e}"
                ))
            })?;

        Self::from_json(json)
    }

    /// Reads a base64-encoded authorized key from the `YANDEX_AUTHORIZED_KEY`
    /// env var.
    pub fn from_env() -> Result<Self, SDKError> {
        let encoded = std::env::var(AUTHORIZED_KEY_ENV).map_err(|e| {
            SDKError::Config(format!("{AUTHORIZED_KEY_ENV}: {e}"))
        })?;

        Self::from_base64(&encoded)
    }

    /// Key identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Identifier of the service account the key belongs to.
    pub fn service_account_id(&self) -> &str {
        &self.service_account_id
    }

    /// Signs a JWT for `aud`, to be exchanged for an IAM token.
    pub(crate) fn sign_jwt(&self, aud: &Url) -> Result<String, SDKError> {
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let claims = Claims {
            aud: aud.clone(),
            iss: self.service_account_id.clone(),
            exp: iat + JWT_LIFETIME_SECS,
            iat,
        };

        let mut header = Header::new(Algorithm::PS256);
        header.kid = Some(self.id.clone());

        encode(&header, &claims, &self.encoding).map_err(|e| {
            tracing::error!("Token generation failed: {e:?}");

            SDKError::Internal("JWT TOKEN GENERATION FAILED".to_string())
        })
    }
}

fn decode_authorized_key_base64(
    encoded: &str,
) -> Result<Vec<u8>, base64::DecodeError> {
    BASE64_STANDARD.decode(encoded)
}

/// IAM JWT claims payload.
#[derive(Serialize)]
struct Claims {
    aud: Url,
    iss: String,
    exp: u64,
    iat: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Throwaway key generated for these tests only; it protects nothing.
    const TEST_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDOlZefYeXyou0U\noxkRocbejQAB4Vyl85oCv02Mb5IqOVamDMPuulVdu5U0/OkbnXnS1Sbg2OP9a5jr\nbDTV68rIRIYE9sHgtP5XtZXS0xdEKMEBIIln8OK8cix9AYIGvXoynyXPk1aS6crA\nCm4zHOcJIZQ07kujZ4XSBLccYTNfFdmDTeZkjzrvCJUAcD0/oGvG46vTAJ3bjrjO\nB7JWRaRkWSHCGBGTlU29zhmBzKjZab6kt0cgOBjyXhomplfbF3GGhMIvS7BTXvzt\n04PaAwTFujbPzyrqdFfyGdCz2ljlYJfZXiDFEY0AIGJ9YpQUJmGlooRk/8KefW/s\nmV10X02dAgMBAAECggEAMvofDClwQMuLjUgh64PsOwa4Nb6SyjCulHb4f+sYOmsj\nwp3ry4EXh6W+T/EI5NObODd4/IsI14QxnAZ7kz44L+sY1yi89uIy0Rnx+rA0UZFs\n7wZEYe3DRZU2/THeECO5f7xd7DcDagVfDryELQC40jwDAny7FXt6PjUDqnEh6Bhe\npqIy2D6StwZkQuF0W2s42SJbNVQUGBEwuck99pYgsqWMXGPBsvw23xumxkHvjCTt\nkrpGKqaykENux3P6pKvF9FQeIoMDPXU5sBS7M3cf0aEBpCEihJVNZS891iq+DVZm\net13AGRmqA8ScHSmOExxpPFeZOazfa9X93q71is+UQKBgQDsQxLs5yIZ8BGejO6L\nZllCowqoL0iE+LpDmnWDSz1VuvN7uG23AznQSyAHWgIyszKjdxaVNkMi6IDoy2YG\nHYK9TKLx3ExFJYdNLHhHmFYh3gR6rxguzSRgqyXPlQnz10Mr0oNcLz5XQ0T5ow5a\nqkhKsxIMJV1xNoO9qfcYM9c9owKBgQDf182pep4njk1Xb64oLcCqjx+L6F5Z6aXn\n01ubQi6gdT/tDSVp2AEq07Vi23rXL1Nh+YSxlSicRKf010AIaA/BVOL6U5zdt11d\n/UgPnQIgH3stEkwfi86zK2YvQgKqtILMkZaLIA+BRTgyNzudMJWvVc0j3w4El/1n\n8SmYP9l7vwKBgQCFGdB6qEf85tN2SB1HaVSWBvZFA8ZOKzX8SfM0EVovhxAOvjsJ\nJIcYgoo7ugiM+Ylor/mH/DbcXrHo9FW40j1KWfdeXGaXeen8nzXv17GGiOZrG2N7\nUlTRJPo7NNKTjc0ozgL1FyR+0sX0AVlf2Ji7hKCBOTeoRTb4zd0HxITZEwKBgDEY\nPe1cDXATj/cLcaEyV72Q4pRnBLqnulGcU807uMpMrRaq+Xe7fpYMFQ53oPutT5Z/\niZEBbATKMiRLSaoOWNZIbfgFerROUVYaBUIXZ63v+a4rAzMwEMfPTvwyeC9EpCOG\nEwS0pXHu4qJw5sxVKZ9pLDMf6w0q4NN5W2wfJO41AoGAZeFt4H0bIpUCuF2QyBrY\nlDP6PT1um2Cz1cBoimXRSTe9Pg/BFbNpURaGsEVlOy1dhqzSHQOgu48fas7yeOu5\nGD4bWawOI1VFsDzvIALNOzmTIDTXIqv0AKyML5DUg7C0RtbzqV0VikydF7MIidtu\ngyDoFkWhupmt3bMZgE/sX80=\n-----END PRIVATE KEY-----";

    fn key_json() -> String {
        serde_json::json!({
            "id": "test-key-id",
            "service_account_id": "test-sa-id",
            "private_key": format!("PLEASE DO NOT REMOVE THIS LINE!\n{TEST_PRIVATE_KEY}"),
        })
        .to_string()
    }

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

    #[test]
    fn parses_key_with_yc_preamble_and_signs_jwt() {
        let key = AuthorizedKey::from_json(key_json()).unwrap();
        assert_eq!(key.id(), "test-key-id");
        assert_eq!(key.service_account_id(), "test-sa-id");

        let aud = Url::parse("https://iam.api.cloud.yandex.net/iam/v1/tokens")
            .unwrap();
        let jwt = key.sign_jwt(&aud).unwrap();

        let mut parts = jwt.split('.');
        let decode = |part: &str| -> serde_json::Value {
            serde_json::from_slice(
                &BASE64_URL_SAFE_NO_PAD.decode(part).unwrap(),
            )
            .unwrap()
        };
        let header = decode(parts.next().unwrap());
        let claims = decode(parts.next().unwrap());
        assert!(parts.next().is_some() && parts.next().is_none());

        assert_eq!(header["alg"], "PS256");
        assert_eq!(header["kid"], "test-key-id");
        assert_eq!(claims["iss"], "test-sa-id");
        assert_eq!(claims["aud"], aud.as_str());
    }

    #[test]
    fn from_base64_round_trips() {
        let encoded = BASE64_STANDARD.encode(key_json());

        assert_eq!(
            AuthorizedKey::from_base64(&encoded).unwrap().id(),
            "test-key-id"
        );
    }

    #[test]
    fn invalid_keys_are_config_errors_not_panics() {
        assert!(matches!(
            AuthorizedKey::from_json("not json"),
            Err(SDKError::Config(_))
        ));
        assert!(matches!(
            AuthorizedKey::from_json(
                r#"{"id":"a","service_account_id":"b","private_key":"nope"}"#
            ),
            Err(SDKError::Config(_))
        ));
        assert!(matches!(
            AuthorizedKey::from_base64("not-base64!"),
            Err(SDKError::Config(_))
        ));
    }
}
