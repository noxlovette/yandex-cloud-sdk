//! Rust SDK for Yandex Cloud gRPC APIs with small handwritten helpers for auth,
//! KMS, and logging workflows.

#![warn(missing_docs)]

mod generated {
    #![allow(missing_docs)]
    include!(concat!(env!("OUT_DIR"), "/_includes.rs"));
}

pub use generated::*;

mod error;
mod jwt;
pub use error::*;
mod client;
pub use client::*;
mod kms;
mod logging;
