use std::str::from_utf8;

use yandex_cloud_sdk::{
    Client,
    yandex::cloud::kms::v1::{SymmetricDecryptRequest, SymmetricEncryptRequest},
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();

    let raw = "This is a message for you";

    let sdk = Client::new()?;
    let mut kms = sdk.kms_symmetric_crypto_client().await?;

    let response = kms
        .encrypt(SymmetricEncryptRequest {
            version_id: String::new(),
            key_id: "abj8sb6uplgh2mga9gdv".to_string(),
            plaintext: raw.as_bytes().to_vec(),
            aad_context: Vec::new(),
        })
        .await?
        .into_inner();

    println!("cipher {:?}", response.ciphertext);

    let decoded = kms
        .decrypt(SymmetricDecryptRequest {
            key_id: "abj8sb6uplgh2mga9gdv".to_string(),
            aad_context: Vec::new(),
            ciphertext: response.ciphertext,
        })
        .await?
        .into_inner();

    println!("decoded {:?}", from_utf8(&decoded.plaintext));

    Ok(())
}
