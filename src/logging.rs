use chrono::Utc;
use prost_types::Timestamp;

use crate::{
    Client, SDKError,
    yandex::cloud::logging::v1::{
        Criteria, Destination, GetLogGroupRequest, IncomingLogEntry, ListLogGroupsRequest,
        LogGroup, ReadRequest, ReadResponse, WriteRequest, WriteResponse, destination, log_level,
    },
};

fn now_timestamp() -> Timestamp {
    let now = Utc::now();

    Timestamp {
        seconds: now.timestamp(),
        nanos: now.timestamp_subsec_nanos() as i32,
    }
}

impl Client {
    /// Writes log entries using raw logging write request.
    pub async fn logging_write(&self, request: WriteRequest) -> Result<WriteResponse, SDKError> {
        let mut logging = self.logging_ingestion_client().await?;

        Ok(logging.write(request).await?.into_inner())
    }

    /// Writes single text message into log group with current timestamp.
    pub async fn logging_write_message(
        &self,
        log_group_id: &str,
        level: log_level::Level,
        message: impl Into<String>,
    ) -> Result<WriteResponse, SDKError> {
        self.logging_write(WriteRequest {
            destination: Some(Destination {
                destination: Some(destination::Destination::LogGroupId(log_group_id.to_string())),
            }),
            resource: None,
            entries: vec![IncomingLogEntry {
                timestamp: Some(now_timestamp()),
                level: level as i32,
                message: message.into(),
                json_payload: None,
                stream_name: String::new(),
            }],
            defaults: None,
        })
        .await
    }

    /// Fetches log group by ID.
    pub async fn logging_get_group(&self, log_group_id: &str) -> Result<LogGroup, SDKError> {
        let mut logging = self.logging_group_client().await?;

        Ok(logging
            .get(GetLogGroupRequest {
                log_group_id: log_group_id.to_string(),
            })
            .await?
            .into_inner())
    }

    /// Lists log groups in folder with optional page token and filter.
    pub async fn logging_list_groups(
        &self,
        folder_id: &str,
        page_size: i64,
        page_token: impl Into<String>,
        filter: impl Into<String>,
    ) -> Result<crate::yandex::cloud::logging::v1::ListLogGroupsResponse, SDKError> {
        let mut logging = self.logging_group_client().await?;

        Ok(logging
            .list(ListLogGroupsRequest {
                folder_id: folder_id.to_string(),
                page_size,
                page_token: page_token.into(),
                filter: filter.into(),
            })
            .await?
            .into_inner())
    }

    /// Reads logs using raw logging read request.
    pub async fn logging_read(&self, request: ReadRequest) -> Result<ReadResponse, SDKError> {
        let mut logging = self.logging_reading_client().await?;

        Ok(logging.read(request).await?.into_inner())
    }

    /// Reads logs from log group with simple criteria request.
    pub async fn logging_read_group(
        &self,
        log_group_id: &str,
        page_size: i64,
        filter: impl Into<String>,
    ) -> Result<ReadResponse, SDKError> {
        self.logging_read(ReadRequest {
            selector: Some(crate::yandex::cloud::logging::v1::read_request::Selector::Criteria(
                Criteria {
                    log_group_id: log_group_id.to_string(),
                    resource_types: Vec::new(),
                    resource_ids: Vec::new(),
                    since: None,
                    until: None,
                    levels: Vec::new(),
                    filter: filter.into(),
                    stream_names: Vec::new(),
                    page_size,
                    max_response_size: 0,
                },
            )),
        })
        .await
    }
}
