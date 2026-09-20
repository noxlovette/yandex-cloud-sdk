//! Rust SDK for Yandex Cloud gRPC APIs with small handwritten helpers for auth,
//! KMS, and logging workflows.

#![warn(missing_docs)]

mod generated {
    #![allow(missing_docs)]
    #![allow(rustdoc::invalid_html_tags)]
    #![allow(dead_code)]
    #![allow(clippy::module_inception)]
    include!(concat!(env!("OUT_DIR"), "/_includes.rs"));
}

mod error;
mod jwt;
pub use error::*;
mod client;
pub use client::*;
mod kms;
mod logging;
mod ocr;
mod vision;

/// Generated protobuf messages and gRPC service clients for the compiled
/// Yandex Cloud APIs.
///
/// Each `…::v1::<service>_client` module holds the raw `tonic` client. Get an
/// authenticated one from [`Client`] (e.g. [`Client::vision_client`]) rather
/// than constructing it yourself.
#[doc(inline)]
pub use generated::{google, yandex};
