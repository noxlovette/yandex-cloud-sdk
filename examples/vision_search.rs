//! Vision image copy search example — find web copies of an image.
//!
//! Required env vars:
//!   YANDEX_AUTHORIZED_KEY  — base64-encoded service-account key JSON
//!   YANDEX_IMAGE_PATH      — path to a JPEG or PNG file to search for
//!   YC_FOLDER_ID           — Yandex Cloud folder ID (required for user auth;
//!                            leave empty for service-account auth)
//!
//! Run:
//!   YANDEX_IMAGE_PATH=photo.jpg cargo run --example vision_search

use std::{env, fs};

use anyhow::{Context, Result};
use yandex_cloud_sdk::Client;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let image_path = env::var("YANDEX_IMAGE_PATH").context("missing YANDEX_IMAGE_PATH")?;
    let folder_id = env::var("YC_FOLDER_ID").unwrap_or_default();

    let content = fs::read(&image_path)
        .with_context(|| format!("failed to read image at {image_path}"))?;

    println!("searching for copies of {image_path} ({} bytes)...", content.len());

    let client = Client::new()?;

    let result = client
        .vision_image_copy_search(content, &folder_id)
        .await?;

    println!("found {} copy/copies on the web\n", result.copy_count);

    if result.top_results.is_empty() {
        println!("no top results returned");
        return Ok(());
    }

    for (i, m) in result.top_results.iter().enumerate() {
        println!("#{}", i + 1);
        if !m.title.is_empty() {
            println!("  title:       {}", m.title);
        }
        if !m.description.is_empty() {
            println!("  description: {}", m.description);
        }
        println!("  image url:   {}", m.image_url);
        println!("  page url:    {}", m.page_url);
        println!();
    }

    Ok(())
}
