use std::{env, time::Duration};

use anyhow::{Context, Result};
use tokio::time::sleep;
use yandex_cloud_sdk::{Client, yandex::cloud::logging::v1::log_level};

#[tokio::main]
async fn main() -> Result<()> {
    let log_group_id =
        env::var("YANDEX_LOG_GROUP_ID").context("missing YANDEX_LOG_GROUP_ID env var")?;
    let write_message = env::var("YANDEX_LOG_MESSAGE")
        .unwrap_or_else(|_| "hello from yandex-cloud-sdk example".to_string());
    let read_filter = env::var("YANDEX_LOG_FILTER")
        .unwrap_or_else(|_| format!("message=\"{write_message}\""));

    let client = Client::new()?;

    println!("writing log entry to group {log_group_id}...");
    let write_response = client
        .logging_write_message(&log_group_id, log_level::Level::Info, write_message.clone())
        .await?;
    println!("write response: {write_response:#?}");

    sleep(Duration::from_secs(3)).await;

    println!("reading log entries back...");
    let read_response = client
        .logging_read_group(&log_group_id, 10, read_filter)
        .await?;

    println!("read {} entries", read_response.entries.len());
    for entry in read_response.entries {
        println!(
            "- {} [{}] {}",
            entry.saved_at.map(|ts| ts.seconds).unwrap_or_default(),
            log_level::Level::try_from(entry.level)
                .map(|level| level.as_str_name().to_string())
                .unwrap_or_else(|_| entry.level.to_string()),
            entry.message
        );
    }

    Ok(())
}
