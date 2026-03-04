use yandex_cloud_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();

    let client = Client::new()?;

    let iam = client.iam().await?;

    println!("{iam:?}");

    Ok(())
}
