use std::str::Utf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SDKError {
    #[error("Jwt error")]
    Jwt,

    #[error("Internal: {0}")]
    Internal(String),

    #[error("Config: {0}")]
    Config(String),

    #[error("gRPC transport error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),

    #[error("gRPC status error: {0}")]
    GrpcStatus(#[from] tonic::Status),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl From<Utf8Error> for SDKError {
    fn from(value: Utf8Error) -> Self {
        Self::Internal(value.to_string())
    }
}
