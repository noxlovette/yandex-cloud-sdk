#[cfg(feature = "ocr")]
use crate::generated::yandex::cloud::ai::ocr::v1::text_recognition_service_client::TextRecognitionServiceClient;
#[cfg(feature = "vision")]
use crate::generated::yandex::cloud::ai::vision::v1::vision_service_client::VisionServiceClient;
#[cfg(feature = "kms")]
use crate::generated::yandex::cloud::kms::v1::symmetric_crypto_service_client::SymmetricCryptoServiceClient;
#[cfg(feature = "logging")]
use crate::generated::yandex::cloud::logging::v1::{
    log_group_service_client::LogGroupServiceClient,
    log_ingestion_service_client::LogIngestionServiceClient,
    log_reading_service_client::LogReadingServiceClient,
};
use crate::{
    SDKError,
    generated::yandex::cloud::iam::v1::{
        CreateIamTokenRequest, CreateIamTokenResponse, create_iam_token_request::Identity,
        iam_token_service_client::IamTokenServiceClient,
    },
    jwt::AuthorizedKey,
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
    codegen::{BoxFuture, Service as TowerService, StdError, http},
    transport::{Channel, ClientTlsConfig, Endpoint},
};

/// Audience of the JWTs exchanged for IAM tokens. Fixed, whatever the IAM
/// endpoint is.
const IAM_AUD: &str = "https://iam.api.cloud.yandex.net/iam/v1/tokens";

/// How long before its expiry a cached IAM token is considered stale and
/// replaced.
const TOKEN_REFRESH_MARGIN: Duration = Duration::from_secs(5 * 60);

/// Assumed IAM token lifetime when the response carries no `expires_at`.
const TOKEN_FALLBACK_TTL: Duration = Duration::from_secs(60 * 60);

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// A Yandex Cloud API that [`Client`] has a dedicated gRPC client for.
///
/// Used to point a service at a different endpoint with
/// [`ClientBuilder::endpoint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Service {
    /// IAM token service.
    Iam,
    /// KMS symmetric crypto service.
    Kms,
    /// Logging log group management.
    Logging,
    /// Logging ingestion (writing).
    LoggingIngestion,
    /// Logging reading.
    LoggingReading,
    /// OCR text recognition.
    Ocr,
    /// Vision.
    Vision,
}

impl Service {
    fn default_endpoint(self) -> &'static str {
        match self {
            Self::Iam => "https://iam.api.cloud.yandex.net",
            Self::Kms => "https://kms.yandex:443",
            Self::Logging => "https://logging.api.cloud.yandex.net",
            Self::LoggingIngestion => {
                "https://ingester.logging.yandexcloud.net"
            }
            Self::LoggingReading => "https://reader.logging.yandexcloud.net",
            Self::Ocr => "https://ocr.api.cloud.yandex.net",
            Self::Vision => "https://vision.api.cloud.yandex.net",
        }
    }
}

/// How a [`Client`] gets IAM tokens.
enum Auth {
    /// Exchange a service account key for short-lived tokens, refreshing as
    /// needed.
    ServiceAccount(AuthorizedKey),
    /// A caller-provided token, used as is and never refreshed.
    Static(http::HeaderValue),
}

/// Authenticated Yandex Cloud SDK client.
///
/// Cheap to clone; clones share one IAM token cache and one set of gRPC
/// channels. The IAM token is fetched on first use and transparently refreshed
/// shortly before it expires, so a `Client` (or any service client built from
/// it) can be held for as long as needed.
///
/// [`Client::new`] authenticates with the service account key in the
/// `YANDEX_AUTHORIZED_KEY` env var; use [`Client::builder`] for anything else.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    auth: Auth,
    /// Cached IAM token. The async mutex is held across the refresh so
    /// concurrent callers wait for one exchange instead of each doing their
    /// own.
    token: AsyncMutex<Option<CachedToken>>,
    channels: Mutex<HashMap<String, Channel>>,
    endpoints: HashMap<Service, String>,
    timeout: Option<Duration>,
    connect_timeout: Duration,
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

/// Builder for a [`Client`], from [`Client::builder`].
///
/// Without explicit credentials, [`build`](Self::build) falls back to the
/// `YANDEX_AUTHORIZED_KEY` env var.
#[derive(Debug)]
pub struct ClientBuilder {
    credentials: Option<Credentials>,
    endpoints: HashMap<Service, String>,
    timeout: Option<Duration>,
    connect_timeout: Duration,
}

#[derive(Debug)]
enum Credentials {
    Key(AuthorizedKey),
    Token(String),
}

impl ClientBuilder {
    /// Authenticates as the service account that owns `key`.
    pub fn authorized_key(mut self, key: AuthorizedKey) -> Self {
        self.credentials = Some(Credentials::Key(key));
        self
    }

    /// Uses a ready-made IAM token as is (for example from `yc iam
    /// create-token`).
    ///
    /// The token is never refreshed, so calls fail once it expires.
    pub fn iam_token(mut self, token: impl Into<String>) -> Self {
        self.credentials = Some(Credentials::Token(token.into()));
        self
    }

    /// Talks to `url` (e.g. `"http://localhost:50051"`) instead of the default endpoint for `service`.
    ///
    /// Plain `http://` URLs are connected without TLS.
    pub fn endpoint(
        mut self,
        service: Service,
        url: impl Into<String>,
    ) -> Self {
        self.endpoints.insert(service, url.into());
        self
    }

    /// Deadline for unary calls made by [`Client`]'s helpers and
    /// [`Client::request`]; `None` means no deadline. Defaults to 20
    /// seconds.
    ///
    /// Streaming calls never get a default deadline: a channel-wide one would
    /// cut off long-lived streams. Set one on the individual `tonic::Request`
    /// with [`set_timeout`](tonic::Request::set_timeout) if you want it.
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// How long to wait when connecting to an endpoint. Defaults to 5 seconds.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Builds the client. Doesn't touch the network; connecting and the IAM
    /// exchange happen on first use.
    pub fn build(self) -> Result<Client, SDKError> {
        let auth = match self.credentials {
            Some(Credentials::Key(key)) => Auth::ServiceAccount(key),
            Some(Credentials::Token(token)) => {
                Auth::Static(bearer_header(&token)?)
            }
            None => Auth::ServiceAccount(AuthorizedKey::from_env()?),
        };

        Ok(Client {
            inner: Arc::new(Inner {
                auth,
                token: AsyncMutex::new(None),
                channels: Mutex::new(HashMap::new()),
                endpoints: self.endpoints,
                timeout: self.timeout,
                connect_timeout: self.connect_timeout,
            }),
        })
    }
}

fn bearer_header(token: &str) -> Result<http::HeaderValue, SDKError> {
    let mut header = http::HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|e| {
            SDKError::Config(format!(
                "failed to parse authorization header: {e}"
            ))
        })?;
    header.set_sensitive(true);

    Ok(header)
}

/// gRPC channel that authenticates every request with the [`Client`]'s current
/// IAM token.
///
/// This is the transport type of every service client built from a [`Client`],
/// whether through [`Client::channel`] or a helper like
/// `Client::vision_client`. The token is looked up (and refreshed if needed)
/// per request, so a service client never goes stale.
#[derive(Clone, Debug)]
pub struct AuthChannel {
    channel: Channel,
    client: Client,
}

impl TowerService<http::Request<Body>> for AuthChannel {
    type Error = StdError;
    type Future = BoxFuture<Self::Response, Self::Error>;
    type Response = http::Response<Body>;

    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.channel.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: http::Request<Body>) -> Self::Future {
        // Take the instance that was driven to readiness, leave a fresh clone
        // behind.
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
    /// Creates a client that authenticates with the service account key in the
    /// `YANDEX_AUTHORIZED_KEY` env var (base64-encoded key JSON).
    ///
    /// Fails if the variable is missing or doesn't hold a valid key. Shorthand
    /// for `Client::builder().build()`.
    pub fn new() -> Result<Self, SDKError> {
        Self::builder().build()
    }

    /// Starts building a client with custom credentials, endpoints or timeouts.
    pub fn builder() -> ClientBuilder {
        ClientBuilder {
            credentials: None,
            endpoints: HashMap::new(),
            timeout: Some(DEFAULT_TIMEOUT),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
        }
    }

    /// Exchanges the service account JWT for a fresh IAM token.
    ///
    /// This always performs the exchange. Service clients don't need it: they
    /// use a cached token that is refreshed automatically. Fails with
    /// [`SDKError::Config`] if the client was built with a static
    /// [`iam_token`](ClientBuilder::iam_token).
    pub async fn iam(&self) -> Result<CreateIamTokenResponse, SDKError> {
        let Auth::ServiceAccount(key) = &self.inner.auth else {
            return Err(SDKError::Config(
                "client uses a static IAM token, there is no key to exchange"
                    .into(),
            ));
        };

        let jwt = key.sign_jwt(
            &url::Url::from_str(IAM_AUD)
                .map_err(|e| SDKError::Internal(e.to_string()))?,
        )?;

        let channel = self.api_channel(&self.endpoint(Service::Iam)).await?;

        let mut client = IamTokenServiceClient::new(channel);

        let response = client
            .create(self.request(CreateIamTokenRequest {
                identity: Some(Identity::Jwt(jwt)),
            }))
            .await?
            .into_inner();

        Ok(response)
    }

    /// Wraps `message` in a unary request carrying this client's
    /// [`timeout`](ClientBuilder::timeout).
    ///
    /// Use it with raw service clients to get the same deadline as the helpers:
    /// `client.create(sdk.request(msg))`.
    pub fn request<T>(&self, message: T) -> tonic::Request<T> {
        let mut request = tonic::Request::new(message);
        if let Some(timeout) = self.inner.timeout {
            request.set_timeout(timeout);
        }

        request
    }

    /// Returns a channel to `endpoint` (e.g. `"https://llm.api.cloud.yandex.net"`) that
    /// authenticates every request, for use with any generated service client:
    /// `TextGenerationServiceClient::new(client.channel(url).await?)`.
    ///
    /// Channels are cached per endpoint and shared between clones of the
    /// client.
    pub async fn channel(
        &self,
        endpoint: &str,
    ) -> Result<AuthChannel, SDKError> {
        Ok(AuthChannel {
            channel: self.api_channel(endpoint).await?,
            client: self.clone(),
        })
    }

    /// Returns the `authorization` header value for the current IAM token,
    /// exchanging a new one first if none is cached or it is about to expire.
    async fn bearer(&self) -> Result<http::HeaderValue, SDKError> {
        if let Auth::Static(header) = &self.inner.auth {
            return Ok(header.clone());
        }

        let mut cached = self.inner.token.lock().await;

        if let Some(token) =
            cached.as_ref().filter(|t| Instant::now() < t.refresh_at)
        {
            return Ok(token.header.clone());
        }

        let response = self.iam().await?;
        let header = bearer_header(&response.iam_token)?;

        let lifetime = response
            .expires_at
            .and_then(|ts| u64::try_from(ts.seconds).ok())
            .map(|expires| {
                Duration::from_secs(expires).saturating_sub(unix_now())
            })
            .unwrap_or(TOKEN_FALLBACK_TTL);

        *cached = Some(CachedToken {
            header: header.clone(),
            refresh_at: Instant::now()
                + lifetime.saturating_sub(TOKEN_REFRESH_MARGIN),
        });

        Ok(header)
    }

    fn endpoint(&self, service: Service) -> String {
        self.inner
            .endpoints
            .get(&service)
            .cloned()
            .unwrap_or_else(|| service.default_endpoint().to_string())
    }

    async fn api_channel(&self, url: &str) -> Result<Channel, SDKError> {
        let cached = self
            .inner
            .channels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(url)
            .cloned();
        if let Some(channel) = cached {
            return Ok(channel);
        }

        let version = env!("CARGO_PKG_VERSION");
        let mut ep = Endpoint::from_shared(url.to_string()).map_err(|e| {
            SDKError::Config(format!("invalid endpoint {url:?}: {e}"))
        })?;
        if url.starts_with("https://") {
            ep = ep.tls_config(ClientTlsConfig::new().with_enabled_roots())?;
        }
        // No channel-wide request timeout: it would also become the deadline of
        // streaming calls. Unary calls get theirs from `Client::request`.
        let ep = ep
            .connect_timeout(self.inner.connect_timeout)
            .user_agent(format!("yandex-cloud-rust-sdk/{version}"))
            .map_err(|e| SDKError::Config(e.to_string()))?;

        let channel = ep.connect().await?;

        self.inner
            .channels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(url.to_string(), channel.clone());

        Ok(channel)
    }

    /// Raw KMS symmetric crypto gRPC client, authenticated as this [`Client`].
    #[cfg(feature = "kms")]
    pub async fn kms_symmetric_crypto_client(
        &self,
    ) -> Result<SymmetricCryptoServiceClient<AuthChannel>, SDKError> {
        Ok(SymmetricCryptoServiceClient::new(
            self.channel(&self.endpoint(Service::Kms)).await?,
        ))
    }

    /// Raw Logging log group management gRPC client, authenticated as this
    /// [`Client`].
    #[cfg(feature = "logging")]
    pub async fn logging_group_client(
        &self,
    ) -> Result<LogGroupServiceClient<AuthChannel>, SDKError> {
        Ok(LogGroupServiceClient::new(
            self.channel(&self.endpoint(Service::Logging)).await?,
        ))
    }

    /// Raw Logging ingestion gRPC client, authenticated as this [`Client`].
    #[cfg(feature = "logging")]
    pub async fn logging_ingestion_client(
        &self,
    ) -> Result<LogIngestionServiceClient<AuthChannel>, SDKError> {
        Ok(LogIngestionServiceClient::new(
            self.channel(&self.endpoint(Service::LoggingIngestion))
                .await?,
        ))
    }

    /// Raw Logging reading gRPC client, authenticated as this [`Client`].
    #[cfg(feature = "logging")]
    pub async fn logging_reading_client(
        &self,
    ) -> Result<LogReadingServiceClient<AuthChannel>, SDKError> {
        Ok(LogReadingServiceClient::new(
            self.channel(&self.endpoint(Service::LoggingReading))
                .await?,
        ))
    }

    /// Raw OCR text recognition gRPC client, authenticated as this [`Client`].
    #[cfg(feature = "ocr")]
    pub async fn ocr_text_recognition_client(
        &self,
    ) -> Result<TextRecognitionServiceClient<AuthChannel>, SDKError> {
        Ok(TextRecognitionServiceClient::new(
            self.channel(&self.endpoint(Service::Ocr)).await?,
        ))
    }

    /// Raw Vision gRPC client, authenticated as this [`Client`].
    #[cfg(feature = "vision")]
    pub async fn vision_client(
        &self,
    ) -> Result<VisionServiceClient<AuthChannel>, SDKError> {
        Ok(VisionServiceClient::new(
            self.channel(&self.endpoint(Service::Vision)).await?,
        ))
    }
}

fn unix_now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn static_client(builder: ClientBuilder) -> Client {
        builder.iam_token("t1.test").build().unwrap()
    }

    #[test]
    fn request_carries_configured_timeout() {
        let with = static_client(
            Client::builder().timeout(Some(Duration::from_secs(7))),
        );
        assert!(with.request(()).metadata().get("grpc-timeout").is_some());

        let without = static_client(Client::builder().timeout(None));
        assert!(without.request(()).metadata().get("grpc-timeout").is_none());
    }

    #[test]
    fn endpoint_overrides_apply_per_service() {
        let client = static_client(
            Client::builder().endpoint(Service::Ocr, "http://localhost:1"),
        );

        assert_eq!(client.endpoint(Service::Ocr), "http://localhost:1");
        assert_eq!(
            client.endpoint(Service::Vision),
            Service::Vision.default_endpoint()
        );
    }

    #[tokio::test]
    async fn static_token_is_used_as_is_and_cannot_be_exchanged() {
        let client = static_client(Client::builder());

        let header = client.bearer().await.unwrap();
        assert_eq!(header, "Bearer t1.test");
        assert!(header.is_sensitive());

        assert!(matches!(client.iam().await, Err(SDKError::Config(_))));
    }

    #[test]
    fn invalid_static_token_is_a_config_error() {
        let err = Client::builder()
            .iam_token("bad\ntoken")
            .build()
            .unwrap_err();

        assert!(matches!(err, SDKError::Config(_)));
    }

    #[tokio::test]
    async fn bad_endpoint_is_a_config_error() {
        let client = static_client(Client::builder());

        assert!(matches!(
            client.channel("not a url").await,
            Err(SDKError::Config(_))
        ));
    }
}
