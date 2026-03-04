use crate::{
    SDKError,
    iam::Iam,
    jwt::{Claims, IamPayload},
};
use std::{str::FromStr, time::Duration};

pub const IAMENDPOINT: &str = "https://iam.api.cloud.yandex.net/iam/v1/tokens";

pub struct Client {
    client: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self, SDKError> {
        let version = env!("CARGO_PKG_VERSION");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .user_agent(format!("yandex-cloud-rust-sdk/{version}"))
            .pool_idle_timeout(Some(Duration::from_secs(90)))
            .pool_max_idle_per_host(20)
            .build()
            .map_err(|e| SDKError::Config(e.to_string()))?;

        Ok(Self { client })
    }

    pub async fn iam(&self) -> Result<Iam, SDKError> {
        let jwt = Claims::new(
            &url::Url::from_str(IAMENDPOINT).map_err(|e| SDKError::Internal(e.to_string()))?,
        )
        .encode()?;

        let iam = self
            .client
            .post(IAMENDPOINT)
            .json(&IamPayload { jwt })
            .send()
            .await?
            .json::<Iam>()
            .await?;

        Ok(iam)
    }
}
