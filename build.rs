use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = vec![
        PathBuf::from("proto/yandex/cloud/access/access.proto"),
        PathBuf::from("proto/yandex/cloud/api/operation.proto"),
        PathBuf::from("proto/yandex/cloud/iam/v1/iam_token_service.proto"),
        PathBuf::from("proto/yandex/cloud/kms/v1/symmetric_crypto_service.proto"),
        PathBuf::from("proto/yandex/cloud/kms/v1/symmetric_key.proto"),
        PathBuf::from("proto/yandex/cloud/kms/v1/symmetric_key_service.proto"),
        PathBuf::from("proto/yandex/cloud/logging/v1/log_entry.proto"),
        PathBuf::from("proto/yandex/cloud/logging/v1/log_group.proto"),
        PathBuf::from("proto/yandex/cloud/logging/v1/log_group_service.proto"),
        PathBuf::from("proto/yandex/cloud/logging/v1/log_ingestion_service.proto"),
        PathBuf::from("proto/yandex/cloud/logging/v1/log_reading_service.proto"),
        PathBuf::from("proto/yandex/cloud/logging/v1/log_resource.proto"),
        PathBuf::from("proto/yandex/cloud/operation/operation.proto"),
    ];

    for proto in &protos {
        println!("cargo:rerun-if-changed={}", proto.display());
    }
    println!("cargo:rerun-if-changed=proto/third_party/googleapis");

    let includes = vec![
        PathBuf::from("proto"),
        PathBuf::from("proto/third_party/googleapis"),
    ];

    tonic_prost_build::configure()
        .build_server(false)
        .include_file("_includes.rs")
        .compile_protos(&protos, &includes)?;

    Ok(())
}
