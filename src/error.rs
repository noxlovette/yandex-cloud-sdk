use thiserror::Error;

#[derive(Error, Debug)]
pub enum SDKError {
    #[error("Jwt error")]
    Jwt,
}
