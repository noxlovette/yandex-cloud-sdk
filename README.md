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

## Features

Nothing is enabled by default: turn on the APIs you use, e.g.
`cargo add yandex-cloud-sdk --features ocr,vision`. Each feature compiles that API's generated
protobuf/gRPC code; the first four also enable the matching `Client` helpers and `*_client()`
accessors.

| Feature             | API                                        |
|---------------------|--------------------------------------------|
| `kms`               | KMS                                        |
| `logging`           | Logging                                    |
| `ocr`               | OCR                                        |
| `vision`            | Vision                                     |
| `foundation-models` | Foundation Models (text/image generation, embeddings, classification) |
| `stt`, `tts`, `translate` | Speech-to-text, text-to-speech, Translate |
| `full`              | Every API in the vendored proto tree (long first compile) |
| `http`              | Adds `SDKError::Http` and a `reqwest` dependency; the SDK itself only speaks gRPC, so most users don't need it |

Only IAM (needed for auth) is always compiled. Features without a `Client` helper are reached
through [`Client::channel`](#raw-grpc-clients).

## Authentication

The client authenticates as a service account. Base64-encode the authorized key JSON you get
from `yc iam key create` (or the Yandex Cloud console) and set it as an env var:

```sh
export YANDEX_AUTHORIZED_KEY="$(base64 -i authorized_key.json)"
```

`Client::new()` reads and validates `YANDEX_AUTHORIZED_KEY` up front (a missing or malformed key
is an `SDKError::Config`, not a panic). On first use the client signs a JWT (`PS256`) for the
service account and exchanges it for an IAM token, which is cached and refreshed shortly before
it expires — you never handle tokens directly.

For anything other than that env var, use the builder:

```rust,no_run
use std::time::Duration;
use yandex_cloud_sdk::{AuthorizedKey, Client, Service};

# fn run() -> Result<(), yandex_cloud_sdk::SDKError> {
let client = Client::builder()
    .authorized_key(AuthorizedKey::from_json(std::fs::read("authorized_key.json").unwrap())?)
    // or: .iam_token("t1.…")   — a static token, never refreshed
    .timeout(Some(Duration::from_secs(30)))                    // unary calls; default 20s
    .endpoint(Service::Vision, "http://localhost:50051")       // e.g. a test server
    .build()?;
# Ok(())
# }
```

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
YANDEX_LOG_GROUP_ID=... cargo run --features logging --example logging
YANDEX_IMAGE_PATH=photo.jpg cargo run --features ocr --example ocr
YANDEX_IMAGE_PATH=photo.jpg cargo run --features vision --example vision_search
```

### Raw gRPC clients

For any RPC without a dedicated helper, `Client` hands out the raw generated `tonic` service
clients (`vision_client`, `ocr_text_recognition_client`, `logging_group_client`,
`logging_ingestion_client`, `logging_reading_client`, `kms_symmetric_crypto_client`), already
authenticated. For any other API, wrap a channel to its endpoint yourself:

```rust,ignore
use yandex_cloud_sdk::yandex::cloud::ai::translate::v2::translation_service_client::TranslationServiceClient;

let mut translate =
    TranslationServiceClient::new(client.channel("https://translate.api.cloud.yandex.net").await?);
```

The IAM token is cached and refreshed shortly before it expires, and channels are shared between
clones of a `Client`, so it is fine to keep a service client for the life of your program. The
full generated module tree (`yandex_cloud_sdk::yandex`, `::google`) is public; see
[`examples/raw_client.rs`](examples/raw_client.rs).

**Timeouts.** The helpers and `Client::request(msg)` attach the client's unary timeout (default 20s)
as a per-call deadline. Streaming RPCs (OCR, log reading, STT, …) get no default deadline, since
one would cut long-lived streams off; set one on the `tonic::Request` if you want it. Raw clients
get no deadline unless you wrap the message with `client.request(..)` or set one yourself.

## Updating the vendored proto files

```sh
git remote add cloudapi https://github.com/yandex-cloud/cloudapi.git
git fetch cloudapi master
git subtree pull --prefix=proto cloudapi master --squash
```

## License

Licensed under the [MIT license](LICENSE).
