use thiserror::Error;

#[derive(Error, Debug)]
pub enum SDKError {
    #[error("Jwt error")]
    Jwt,

    #[error("Internal: {0}")]
    Internal(String),

    #[error("Config: {0}")]
    Config(String),

    #[error("reqwest error: {0}")]
    Server(#[from] reqwest::Error),
}
