//! Raw gRPC client example — call an RPC directly through the generated `tonic`
//! client.
//!
//! The client from `Client::logging_group_client()` authenticates every request
//! with a cached IAM token that refreshes itself, so it can be kept around.
//! `Client::request` wraps a message with the client's default unary deadline.
//!
//! Required env vars:
//!   YANDEX_AUTHORIZED_KEY  — base64-encoded service-account key JSON
//!   YANDEX_FOLDER_ID       — folder whose log groups to list
//!
//! Run:
//!   YANDEX_FOLDER_ID=... cargo run --features logging --example raw_client

use std::env;

use anyhow::{Context, Result};
use yandex_cloud_sdk::{
    Client, yandex::cloud::logging::v1::ListLogGroupsRequest,
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let folder_id =
        env::var("YANDEX_FOLDER_ID").context("missing YANDEX_FOLDER_ID")?;

    let client = Client::new()?;
    let mut groups = client.logging_group_client().await?;

    let response = groups
        .list(client.request(ListLogGroupsRequest {
            folder_id,
            page_size: 100,
            ..Default::default()
        }))
        .await?
        .into_inner();

    for group in response.groups {
        println!("{} ({})", group.name, group.id);
    }

    Ok(())
}
