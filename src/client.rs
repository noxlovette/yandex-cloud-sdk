use crate::{
    SDKError,
    generated::yandex::cloud::{
        ai::{
            ocr::v1::text_recognition_service_client::TextRecognitionServiceClient,
            vision::v1::vision_service_client::VisionServiceClient,
        },
        iam::v1::{
            CreateIamTokenRequest, CreateIamTokenResponse, create_iam_token_request::Identity,
            iam_token_service_client::IamTokenServiceClient,
        },
        kms::v1::symmetric_crypto_service_client::SymmetricCryptoServiceClient,
        logging::v1::{
            log_group_service_client::LogGroupServiceClient,
            log_ingestion_service_client::LogIngestionServiceClient,
            log_reading_service_client::LogReadingServiceClient,
        },
    },
    jwt::Claims,
};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex, PoisonError},
    task::{Context, Poll},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex as AsyncMutex;
use tonic::{
    Status,
    body::Body,
    codegen::{BoxFuture, Service, StdError, http},
    transport::{Channel, ClientTlsConfig, Endpoint},
};

struct Endpoints;

impl Endpoints {
    const IAM_AUD: &str = "https://iam.api.cloud.yandex.net/iam/v1/tokens";
    const IAM_GRPC_ENDPOINT: &str = "https://iam.api.cloud.yandex.net";
    const KMS_CRYPTO_GRPC_ENDPOINT: &str = "https://kms.yandex:443";
    const LOGGING_GRPC_ENDPOINT: &str = "https://logging.api.cloud.yandex.net";
    const LOGGING_INGESTION_GRPC_ENDPOINT: &str = "https://ingester.logging.yandexcloud.net";
    const LOGGING_READING_GRPC_ENDPOINT: &str = "https://reader.logging.yandexcloud.net";
    const OCR_GRPC_ENDPOINT: &str = "https://ocr.api.cloud.yandex.net";
    const VISION_GRPC_ENDPOINT: &str = "https://vision.api.cloud.yandex.net";
}

/// How long before its expiry a cached IAM token is considered stale and replaced.
const TOKEN_REFRESH_MARGIN: Duration = Duration::from_secs(5 * 60);

/// Assumed IAM token lifetime when the response carries no `expires_at`.
const TOKEN_FALLBACK_TTL: Duration = Duration::from_secs(60 * 60);

/// Authenticated Yandex Cloud SDK client.
///
/// Cheap to clone; clones share one IAM token cache and one set of gRPC channels.
/// The IAM token is fetched on first use and transparently refreshed shortly
/// before it expires, so a `Client` (or any service client built from it) can
/// be held for as long as needed.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    /// Cached IAM token. The async mutex is held across the refresh so
    /// concurrent callers wait for one exchange instead of each doing their own.
    token: AsyncMutex<Option<CachedToken>>,
    channels: Mutex<HashMap<&'static str, Channel>>,
}

struct CachedToken {
    header: http::HeaderValue,
    refresh_at: Instant,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

/// gRPC channel that authenticates every request with the [`Client`]'s current IAM token.
///
/// This is the transport type of the service clients returned by
/// [`Client::vision_client`] and friends. The token is looked up (and refreshed
/// if needed) per request, so a service client never goes stale.
#[derive(Clone, Debug)]
pub struct AuthChannel {
    channel: Channel,
    client: Client,
}

impl Service<http::Request<Body>> for AuthChannel {
    type Response = http::Response<Body>;
    type Error = StdError;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.channel.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: http::Request<Body>) -> Self::Future {
        // Take the instance that was driven to readiness, leave a fresh clone behind.
        let ready = self.channel.clone();
        let mut channel = std::mem::replace(&mut self.channel, ready);
        let client = self.client.clone();

        Box::pin(async move {
            let header = client.bearer().await.map_err(|e| {
                Box::new(Status::unauthenticated(format!(
                    "failed to obtain IAM token: {e}"
                ))) as StdError
            })?;
            req.headers_mut()
                .insert(http::header::AUTHORIZATION, header);

            channel.call(req).await.map_err(Into::into)
        })
    }
}

impl Client {
    /// Creates new SDK client.
    pub fn new() -> Result<Self, SDKError> {
        Ok(Self {
            inner: Arc::new(Inner {
                token: AsyncMutex::new(None),
                channels: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// Exchanges service account JWT for a fresh IAM token.
    ///
    /// This always performs the exchange. Service clients don't need it: they
    /// use a cached token that is refreshed automatically.
    pub async fn iam(&self) -> Result<CreateIamTokenResponse, SDKError> {
        let jwt = Claims::new(
            &url::Url::from_str(Endpoints::IAM_AUD)
                .map_err(|e| SDKError::Internal(e.to_string()))?,
        )
        .encode()?;

        let channel = self.api_channel(Endpoints::IAM_GRPC_ENDPOINT).await?;

        let mut client = IamTokenServiceClient::new(channel);

        let response = client
            .create(CreateIamTokenRequest {
                identity: Some(Identity::Jwt(jwt)),
            })
            .await?
            .into_inner();

        Ok(response)
    }

    /// Returns the `authorization` header value for the current IAM token,
    /// exchanging a new one first if none is cached or it is about to expire.
    async fn bearer(&self) -> Result<http::HeaderValue, SDKError> {
        let mut cached = self.inner.token.lock().await;

        if let Some(token) = cached.as_ref().filter(|t| Instant::now() < t.refresh_at) {
            return Ok(token.header.clone());
        }

        let response = self.iam().await?;

        let mut header = http::HeaderValue::from_str(&format!("Bearer {}", response.iam_token))
            .map_err(|e| SDKError::Config(format!("failed to parse authorization header: {e}")))?;
        header.set_sensitive(true);

        let lifetime = response
            .expires_at
            .and_then(|ts| u64::try_from(ts.seconds).ok())
            .map(|expires| Duration::from_secs(expires).saturating_sub(unix_now()))
            .unwrap_or(TOKEN_FALLBACK_TTL);

        *cached = Some(CachedToken {
            header: header.clone(),
            refresh_at: Instant::now() + lifetime.saturating_sub(TOKEN_REFRESH_MARGIN),
        });

        Ok(header)
    }

    async fn api_channel(&self, endpoint: &'static str) -> Result<Channel, SDKError> {
        let cached = self
            .inner
            .channels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(endpoint)
            .cloned();
        if let Some(channel) = cached {
            return Ok(channel);
        }

        let version = env!("CARGO_PKG_VERSION");
        let ep = Endpoint::from_static(endpoint)
            .tls_config(ClientTlsConfig::new().with_enabled_roots())?
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .user_agent(format!("yandex-cloud-rust-sdk/{version}"))
            .map_err(|e| SDKError::Config(e.to_string()))?;

        let channel = ep.connect().await?;

        self.inner
            .channels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(endpoint, channel.clone());

        Ok(channel)
    }

    async fn auth_channel(&self, endpoint: &'static str) -> Result<AuthChannel, SDKError> {
        Ok(AuthChannel {
            channel: self.api_channel(endpoint).await?,
            client: self.clone(),
        })
    }

    /// Raw KMS symmetric crypto gRPC client, authenticated as this [`Client`].
    pub async fn kms_symmetric_crypto_client(
        &self,
    ) -> Result<SymmetricCryptoServiceClient<AuthChannel>, SDKError> {
        Ok(SymmetricCryptoServiceClient::new(
            self.auth_channel(Endpoints::KMS_CRYPTO_GRPC_ENDPOINT)
                .await?,
        ))
    }

    /// Raw Logging log group management gRPC client, authenticated as this [`Client`].
    pub async fn logging_group_client(
        &self,
    ) -> Result<LogGroupServiceClient<AuthChannel>, SDKError> {
        Ok(LogGroupServiceClient::new(
            self.auth_channel(Endpoints::LOGGING_GRPC_ENDPOINT).await?,
        ))
    }

    /// Raw Logging ingestion gRPC client, authenticated as this [`Client`].
    pub async fn logging_ingestion_client(
        &self,
    ) -> Result<LogIngestionServiceClient<AuthChannel>, SDKError> {
        Ok(LogIngestionServiceClient::new(
            self.auth_channel(Endpoints::LOGGING_INGESTION_GRPC_ENDPOINT)
                .await?,
        ))
    }

    /// Raw Logging reading gRPC client, authenticated as this [`Client`].
    pub async fn logging_reading_client(
        &self,
    ) -> Result<LogReadingServiceClient<AuthChannel>, SDKError> {
        Ok(LogReadingServiceClient::new(
            self.auth_channel(Endpoints::LOGGING_READING_GRPC_ENDPOINT)
                .await?,
        ))
    }

    /// Raw OCR text recognition gRPC client, authenticated as this [`Client`].
    pub async fn ocr_text_recognition_client(
        &self,
    ) -> Result<TextRecognitionServiceClient<AuthChannel>, SDKError> {
        Ok(TextRecognitionServiceClient::new(
            self.auth_channel(Endpoints::OCR_GRPC_ENDPOINT).await?,
        ))
    }

    /// Raw Vision gRPC client, authenticated as this [`Client`].
    pub async fn vision_client(&self) -> Result<VisionServiceClient<AuthChannel>, SDKError> {
        Ok(VisionServiceClient::new(
            self.auth_channel(Endpoints::VISION_GRPC_ENDPOINT).await?,
        ))
    }
}

fn unix_now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}
