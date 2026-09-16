//! Rust SDK for Yandex Cloud gRPC APIs with small handwritten helpers for auth,
//! KMS, and logging workflows.

#![warn(missing_docs)]

mod generated {
    #![allow(missing_docs)]
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

/// Protobuf message types actually reachable through [`Client`]'s methods.
///
/// The full generated tree also contains the raw gRPC service client structs
/// (`Client` always wraps these with auth/TLS/timeouts, so client code never
/// touches them directly) and unused management RPCs (e.g. KMS key
/// management, log group CRUD) that [`Client`] doesn't expose — those stay
/// crate-internal.
#[allow(missing_docs)]
pub mod yandex {
    pub mod cloud {
        pub mod iam {
            pub mod v1 {
                pub use crate::generated::yandex::cloud::iam::v1::CreateIamTokenResponse;
            }
        }

        pub mod logging {
            pub mod v1 {
                pub use crate::generated::yandex::cloud::logging::v1::{
                    Criteria, Destination, IncomingLogEntry, LogEntry, LogEntryDefaults,
                    LogEntryResource, LogGroup, LogGroupResource, LogLevel, ListLogGroupsResponse,
                    ReadRequest, ReadResponse, WriteRequest, WriteResponse, destination,
                    log_group, log_level, read_request,
                };
            }
        }

        pub mod ai {
            pub mod ocr {
                pub mod v1 {
                    pub use crate::generated::yandex::cloud::ai::ocr::v1::{
                        Angle, Block, Entity, LayoutType, Line, Picture, Polygon,
                        RecognizeTextResponse, Table, TableCell, TextAnnotation, TextSegments,
                        Vertex, Word, block,
                    };
                }
            }

            pub mod vision {
                pub mod v1 {
                    pub use crate::generated::yandex::cloud::ai::vision::v1::{
                        AnalyzeResult, AnalyzeSpec, BatchAnalyzeRequest, BatchAnalyzeResponse,
                        Block, ClassAnnotation, CopyMatch, Entity, Face, FaceAnnotation, Feature,
                        FeatureClassificationConfig, FeatureResult, FeatureTextDetectionConfig,
                        ImageCopySearchAnnotation, Line, Page, Polygon, Property, TextAnnotation,
                        Vertex, Word, analyze_spec, feature, feature_result, word,
                    };
                }
            }
        }
    }
}
