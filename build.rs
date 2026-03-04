use std::{fs, path::{Path, PathBuf}};

fn collect_proto_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if path.strip_prefix("proto").ok().is_some_and(|p| p.starts_with("third_party")) {
                continue;
            }
            collect_proto_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "proto") {
            files.push(path);
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto");

    let mut protos = Vec::new();
    let includes = vec![
        PathBuf::from("proto"),
        PathBuf::from("proto/third_party/googleapis"),
    ];

    collect_proto_files(Path::new("proto"), &mut protos)?;
    protos.sort();

    tonic_prost_build::configure()
        .build_server(false)
        .include_file("_includes.rs")
        .compile_protos(&protos, &includes)?;

    Ok(())
}
