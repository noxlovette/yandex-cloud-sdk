use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

/// Protos that are always compiled: the IAM token exchange `Client` needs to
/// authenticate, plus the shared operation/access types.
const BASE: &[&str] = &[
    "proto/yandex/cloud/access/access.proto",
    "proto/yandex/cloud/api/operation.proto",
    "proto/yandex/cloud/iam/v1/iam_token_service.proto",
    "proto/yandex/cloud/operation/operation.proto",
];

/// Cargo feature -> proto directories (compiled recursively) it enables.
///
/// To add a service: add a line here, and a feature of the same name in
/// Cargo.toml (and to `full`). Imports between protos are resolved through the
/// include path, so only the service's own directory needs listing.
const FEATURES: &[(&str, &[&str])] = &[
    ("kms", &["proto/yandex/cloud/kms"]),
    ("logging", &["proto/yandex/cloud/logging"]),
    ("ocr", &["proto/yandex/cloud/ai/ocr"]),
    ("vision", &["proto/yandex/cloud/ai/vision"]),
    (
        "foundation-models",
        &["proto/yandex/cloud/ai/foundation_models"],
    ),
    ("stt", &["proto/yandex/cloud/ai/stt"]),
    ("tts", &["proto/yandex/cloud/ai/tts"]),
    ("translate", &["proto/yandex/cloud/ai/translate"]),
];

/// The `full` feature compiles every API in the vendored tree, not just
/// [`FEATURES`].
const FULL_TREE: &str = "proto/yandex/cloud";

fn feature_enabled(name: &str) -> bool {
    let var =
        format!("CARGO_FEATURE_{}", name.to_uppercase().replace('-', "_"));
    env::var_os(var).is_some()
}

fn collect_protos(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_protos(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "proto") {
            out.push(path);
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut protos: Vec<PathBuf> = BASE.iter().map(PathBuf::from).collect();
    let mut dirs: Vec<&str> = Vec::new();

    if feature_enabled("full") {
        dirs.push(FULL_TREE);
    } else {
        for (feature, feature_dirs) in FEATURES {
            if feature_enabled(feature) {
                dirs.extend(*feature_dirs);
            }
        }
    }

    for dir in dirs {
        println!("cargo:rerun-if-changed={dir}");
        collect_protos(Path::new(dir), &mut protos)?;
    }

    // The same file can be reached from several features (and `BASE`).
    protos.sort();
    protos.dedup();

    for proto in BASE {
        println!("cargo:rerun-if-changed={proto}");
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
