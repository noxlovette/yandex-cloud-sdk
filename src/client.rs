use crate::{
    SDKError,
    jwt::Claims,
    yandex::cloud::{
        iam::v1::{
            CreateIamTokenRequest, CreateIamTokenResponse, create_iam_token_request::Identity,
            iam_token_service_client::IamTokenServiceClient,
        },
        kms::v1::{
            symmetric_crypto_service_client::SymmetricCryptoServiceClient,
            symmetric_key_service_client::SymmetricKeyServiceClient,
        },
        logging::v1::{
            log_group_service_client::LogGroupServiceClient,
            log_ingestion_service_client::LogIngestionServiceClient,
            log_reading_service_client::LogReadingServiceClient,
        },
    },
};
use std::{str::FromStr, time::Duration};
use tonic::{
    metadata::{Ascii, MetadataValue},
    service::interceptor::InterceptedService,
    transport::{Channel, ClientTlsConfig, Endpoint},
};

struct Endpoints;

impl Endpoints {
    pub const IAM_AUD: &str = "https://iam.api.cloud.yandex.net/iam/v1/tokens";
    pub const IAM_GRPC_ENDPOINT: &str = "https://iam.api.cloud.yandex.net";
    pub const KMS_GRPC_ENDPOINT: &str = "https://kms.api.cloud.yandex.net";
    pub const KMS_CRYPTO_GRPC_ENDPOINT: &str = "https://kms.yandex:443";
    pub const LOGGING_GRPC_ENDPOINT: &str = "https://logging.api.cloud.yandex.net";
    pub const LOGGING_INGESTION_GRPC_ENDPOINT: &str = "https://ingester.logging.yandexcloud.net";
    pub const LOGGING_READING_GRPC_ENDPOINT: &str = "https://reader.logging.yandexcloud.net";
}

/// Authenticated Yandex Cloud SDK client.
#[derive(Clone, Debug)]
pub struct Client;

/// Tonic interceptor that injects bearer auth header into requests.
#[derive(Clone)]
pub struct AuthInterceptor {
    auth_header: MetadataValue<Ascii>,
}

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        req.metadata_mut()
            .insert("authorization", self.auth_header.clone());
        Ok(req)
    }
}

impl Client {
    /// Creates new SDK client.
    pub fn new() -> Result<Self, SDKError> {
        Ok(Self)
    }

    /// Exchanges service account JWT for IAM token.
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

    async fn api_channel(&self, endpoint: &'static str) -> Result<Channel, SDKError> {
        let version = env!("CARGO_PKG_VERSION");
        let ep = Endpoint::from_static(endpoint)
            .tls_config(ClientTlsConfig::new().with_enabled_roots())?
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .user_agent(format!("yandex-cloud-rust-sdk/{version}"))
            .map_err(|e| SDKError::Config(e.to_string()))?;

        Ok(ep.connect().await?)
    }

    async fn interceptor(&self) -> Result<AuthInterceptor, SDKError> {
        let auth_header = format!("Bearer {}", self.iam().await?.iam_token)
            .parse()
            .map_err(|e| SDKError::Config(format!("failed to parse authorization header: {e}")))?;

        Ok(AuthInterceptor { auth_header })
    }

    pub(crate) async fn kms_symmetric_key_client(
        &self,
    ) -> Result<SymmetricKeyServiceClient<InterceptedService<Channel, AuthInterceptor>>, SDKError>
    {
        let channel = self.api_channel(Endpoints::KMS_GRPC_ENDPOINT).await?;

        Ok(SymmetricKeyServiceClient::with_interceptor(
            channel,
            self.interceptor().await?,
        ))
    }

    pub(crate) async fn kms_symmetric_crypto_client(
        &self,
    ) -> Result<SymmetricCryptoServiceClient<InterceptedService<Channel, AuthInterceptor>>, SDKError>
    {
        let channel = self
            .api_channel(Endpoints::KMS_CRYPTO_GRPC_ENDPOINT)
            .await?;

        Ok(SymmetricCryptoServiceClient::with_interceptor(
            channel,
            self.interceptor().await?,
        ))
    }

    pub(crate) async fn logging_group_client(
        &self,
    ) -> Result<LogGroupServiceClient<InterceptedService<Channel, AuthInterceptor>>, SDKError> {
        let channel = self.api_channel(Endpoints::LOGGING_GRPC_ENDPOINT).await?;

        Ok(LogGroupServiceClient::with_interceptor(
            channel,
            self.interceptor().await?,
        ))
    }

    pub(crate) async fn logging_ingestion_client(
        &self,
    ) -> Result<LogIngestionServiceClient<InterceptedService<Channel, AuthInterceptor>>, SDKError>
    {
        let channel = self
            .api_channel(Endpoints::LOGGING_INGESTION_GRPC_ENDPOINT)
            .await?;

        Ok(LogIngestionServiceClient::with_interceptor(
            channel,
            self.interceptor().await?,
        ))
    }

    pub(crate) async fn logging_reading_client(
        &self,
    ) -> Result<LogReadingServiceClient<InterceptedService<Channel, AuthInterceptor>>, SDKError> {
        let channel = self
            .api_channel(Endpoints::LOGGING_READING_GRPC_ENDPOINT)
            .await?;

        Ok(LogReadingServiceClient::with_interceptor(
            channel,
            self.interceptor().await?,
        ))
    }
}
