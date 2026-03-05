use yandex_cloud_sdk::{Client, yandex::cloud::kms::v1::GetSymmetricKeyRequest};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();

    let sdk = Client::new()?;
    let mut kms = sdk.kms_symmetric_key_client().await?;

    let response = kms
        .get(GetSymmetricKeyRequest {
            key_id: "abjv2dkc1rgpe749lkr0".to_string(),
        })
        .await?
        .into_inner();

    println!("found {} symmetric key", response.name);

    Ok(())
}
