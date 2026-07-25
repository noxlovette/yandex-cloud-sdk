//! OCR example — recognise text in an image or PDF.
//!
//! Required env vars:
//!   YANDEX_AUTHORIZED_KEY  — base64-encoded service-account key JSON
//!   YANDEX_IMAGE_PATH      — path to a JPEG, PNG, or PDF file to recognise
//!
//! Optional env vars:
//!   YANDEX_OCR_MIME_TYPE   — MIME type override (default: "image/jpeg")
//!   YANDEX_OCR_LANGUAGE    — ISO 639-1 language hint (default: "ru")
//!   YANDEX_OCR_MODEL       — model name, "page" or "line" (default: "page")
//!
//! Run:
//!   YANDEX_IMAGE_PATH=photo.jpg cargo run --example ocr

use std::{env, fs};

use anyhow::{Context, Result};
use yandex_cloud_sdk::Client;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let image_path = env::var("YANDEX_IMAGE_PATH").context("missing YANDEX_IMAGE_PATH")?;
    let mime_type = env::var("YANDEX_OCR_MIME_TYPE").unwrap_or_else(|_| "image/jpeg".into());
    let language = env::var("YANDEX_OCR_LANGUAGE").unwrap_or_else(|_| "ru".into());
    let model = env::var("YANDEX_OCR_MODEL").unwrap_or_else(|_| "page".into());

    let content = fs::read(&image_path)
        .with_context(|| format!("failed to read image at {image_path}"))?;

    println!("sending {image_path} ({} bytes) to OCR...", content.len());

    let client = Client::new()?;

    let pages = client
        .ocr_recognize(content, &mime_type, vec![language.clone()], &model)
        .await?;

    if pages.is_empty() {
        println!("no pages returned");
        return Ok(());
    }

    println!("recognised {} page(s)\n", pages.len());

    for page_response in &pages {
        let page_num = page_response.page;
        let annotation = page_response
            .text_annotation
            .as_ref()
            .context("missing text_annotation")?;

        println!("── page {page_num} ({} × {} px) ──", annotation.width, annotation.height);

        if annotation.full_text.is_empty() {
            println!("  (no text detected)");
        } else {
            println!("{}", annotation.full_text);
        }

        println!(
            "\n  blocks: {}  tables: {}  angle: {:?}\n",
            annotation.blocks.len(),
            annotation.tables.len(),
            annotation.rotate,
        );
    }

    Ok(())
}
