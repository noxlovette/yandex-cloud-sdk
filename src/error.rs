use std::str::Utf8Error;

use thiserror::Error;

/// SDK error type for auth, transport, and decoding failures.
#[derive(Error, Debug)]
pub enum SDKError {
    /// JWT creation failed.
    #[error("Jwt error")]
    Jwt,

    /// Internal SDK error with context.
    #[error("Internal: {0}")]
    Internal(String),

    /// Client configuration error.
    #[error("Config: {0}")]
    Config(String),

    /// gRPC transport-layer error.
    #[error("gRPC transport error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),

    /// gRPC request status error.
    #[error("gRPC status error: {0}")]
    GrpcStatus(#[from] tonic::Status),

    /// HTTP client error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl From<Utf8Error> for SDKError {
    fn from(value: Utf8Error) -> Self {
        Self::Internal(value.to_string())
    }
}
