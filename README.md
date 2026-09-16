# yandex-cloud-sdk

[![Crates.io](https://img.shields.io/crates/v/yandex-cloud-sdk.svg)](https://crates.io/crates/yandex-cloud-sdk)
[![docs.rs](https://img.shields.io/docsrs/yandex-cloud-sdk)](https://docs.rs/yandex-cloud-sdk)
[![CI](https://github.com/noxlovette/yandex-cloud-sdk/actions/workflows/rust.yml/badge.svg)](https://github.com/noxlovette/yandex-cloud-sdk/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Unofficial async Rust SDK for [Yandex Cloud's](https://cloud.yandex.com/) gRPC APIs. Auth
(service-account JWT → IAM token exchange), transport (TLS channels, user agent, timeouts) and
a handful of ergonomic helpers are handled for you; everything else is the generated
[`tonic`](https://github.com/hyperium/tonic) client for the underlying protobuf service, so any
method Yandex Cloud exposes is reachable even where this crate has no dedicated helper.

## Services

- **IAM** — exchanges a service-account authorized key for a short-lived IAM token
- **KMS** — symmetric encrypt/decrypt with a KMS key
- **Logging** — write log entries, read/list log groups, query log entries
- **OCR** — text recognition for images and PDFs (streaming)
- **Vision** — batch image analysis, including image copy search

Protobuf definitions are vendored from
[yandex-cloud/cloudapi](https://github.com/yandex-cloud/cloudapi) under `proto/` (see
[Updating the vendored proto files](#updating-the-vendored-proto-files)) and compiled with
[`tonic-prost-build`](https://docs.rs/tonic-prost-build) in `build.rs`.

## Installation

```sh
cargo add yandex-cloud-sdk
```

## Authentication

The client authenticates as a service account. Base64-encode the authorized key JSON you get
from `yc iam key create` (or the Yandex Cloud console) and set it as an env var:

```sh
export YANDEX_AUTHORIZED_KEY="$(base64 -i authorized_key.json)"
```

`Client::new()` reads `YANDEX_AUTHORIZED_KEY` lazily on first use, signs a JWT (`PS256`) for the
service account, and exchanges it for an IAM token before each authenticated call — you never
handle tokens directly.

```rust,no_run
use yandex_cloud_sdk::Client;

# async fn run() -> Result<(), yandex_cloud_sdk::SDKError> {
let client = Client::new()?;
let iam = client.iam().await?;
println!("{}", iam.iam_token);
# Ok(())
# }
```

## Usage

### KMS

```rust,no_run
# use yandex_cloud_sdk::Client;
# async fn run() -> Result<(), yandex_cloud_sdk::SDKError> {
let client = Client::new()?;

let ciphertext = client.encrypt("<key-id>", "hello world").await?;
let plaintext = client.decrypt("<key-id>", ciphertext).await?;
assert_eq!(plaintext, "hello world");
# Ok(())
# }
```

### Logging

```rust,no_run
# use yandex_cloud_sdk::{Client, yandex::cloud::logging::v1::log_level};
# async fn run() -> Result<(), yandex_cloud_sdk::SDKError> {
let client = Client::new()?;

client
    .logging_write_message("<log-group-id>", log_level::Level::Info, "hello from the SDK")
    .await?;

let entries = client
    .logging_read_group("<log-group-id>", 10, "")
    .await?;
# Ok(())
# }
```

### OCR

```rust,no_run
# use yandex_cloud_sdk::Client;
# async fn run() -> Result<(), yandex_cloud_sdk::SDKError> {
let client = Client::new()?;
let image = std::fs::read("photo.jpg").unwrap();

let pages = client
    .ocr_recognize(image, "image/jpeg", vec!["ru".into()], "page")
    .await?;
# Ok(())
# }
```

### Vision

```rust,no_run
# use yandex_cloud_sdk::Client;
# async fn run() -> Result<(), yandex_cloud_sdk::SDKError> {
let client = Client::new()?;
let image = std::fs::read("photo.jpg").unwrap();

let result = client.vision_image_copy_search(image, "").await?;
println!("{} copies found", result.copy_count);
# Ok(())
# }
```

More complete, runnable examples (reading env vars, printing structured output) live in
[`examples/`](examples/):

```sh
YANDEX_LOG_GROUP_ID=... cargo run --example logging
YANDEX_IMAGE_PATH=photo.jpg cargo run --example ocr
YANDEX_IMAGE_PATH=photo.jpg cargo run --example vision_search
```

For any RPC without a dedicated helper, `Client` also exposes the raw request/response types
(`logging_write`, `logging_read`, `vision_batch_analyze`) and the full generated module tree is
public, so you can build a `tonic` request by hand against any message in the vendored `.proto`
files.

## Updating the vendored proto files

```sh
git remote add cloudapi https://github.com/yandex-cloud/cloudapi.git
git fetch cloudapi master
git subtree pull --prefix=proto cloudapi master --squash
```

## License

Licensed under the [MIT license](LICENSE).
