use std::str::from_utf8;

use crate::{
    Client, SDKError,
    yandex::cloud::kms::v1::{SymmetricDecryptRequest, SymmetricEncryptRequest},
};

impl Client {
    /// Encrypts UTF-8 payload with symmetric KMS key.
    pub async fn encrypt(&self, key_id: &str, payload: &str) -> Result<Vec<u8>, SDKError> {
        let mut kms = self.kms_symmetric_crypto_client().await?;

        let response = kms
            .encrypt(SymmetricEncryptRequest {
                version_id: String::new(),
                key_id: key_id.to_string(),
                plaintext: payload.as_bytes().to_vec(),
                aad_context: Vec::new(),
            })
            .await?
            .into_inner();

        Ok(response.ciphertext)
    }

    /// Decrypts ciphertext with symmetric KMS key and returns UTF-8 plaintext.
    pub async fn decrypt(&self, key_id: &str, payload: Vec<u8>) -> Result<String, SDKError> {
        let mut kms = self.kms_symmetric_crypto_client().await?;

        let response = kms
            .decrypt(SymmetricDecryptRequest {
                key_id: key_id.to_string(),
                ciphertext: payload,
                aad_context: Vec::new(),
            })
            .await?
            .into_inner();

        Ok(from_utf8(&response.plaintext)?.to_string())
    }
}
