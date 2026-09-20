//! Rust SDK for Yandex Cloud gRPC APIs with small handwritten helpers for auth,
//! KMS, logging, OCR and Vision workflows.
//!
//! Each API's generated code sits behind a cargo feature, and none is enabled
//! by default: `kms`, `logging`, `ocr`, `vision`, `foundation-models`, `stt`,
//! `tts`, `translate`, or `full` for everything. Only IAM (auth) is always
//! available. The separate `http` feature adds an HTTP error variant and the
//! `reqwest` dependency. Use [`Client::channel`] to reach any API with a raw
//! gRPC client.

#![warn(missing_docs)]

mod generated {
    #![allow(missing_docs)]
    #![allow(rustdoc::all)]
    #![allow(dead_code)]
    #![allow(clippy::module_inception)]
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/_includes.rs"));
}

mod error;
mod jwt;
pub use error::*;
pub use jwt::AuthorizedKey;
mod client;
pub use client::*;
#[cfg(feature = "kms")]
mod kms;
#[cfg(feature = "logging")]
mod logging;
#[cfg(feature = "ocr")]
mod ocr;
#[cfg(feature = "vision")]
mod vision;

/// Generated protobuf messages and gRPC service clients for the compiled
/// Yandex Cloud APIs.
///
/// Each `…::v1::<service>_client` module holds the raw `tonic` client. Get
/// an authenticated one from [`Client`] (e.g. [`Client::vision_client`])
/// rather than constructing it yourself.
#[doc(inline)]
pub use generated::{google, yandex};
