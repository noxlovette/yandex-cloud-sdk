use crate::{
    Client, SDKError,
    yandex::cloud::ai::vision::v1::{
        AnalyzeSpec, BatchAnalyzeRequest, BatchAnalyzeResponse, Feature, ImageCopySearchAnnotation,
        analyze_spec::Source,
        feature::Type as FeatureType,
        feature_result::Feature as FeatureVariant,
    },
};

impl Client {
    /// Analyzes a batch of images using the Vision service.
    pub async fn vision_batch_analyze(
        &self,
        request: BatchAnalyzeRequest,
    ) -> Result<BatchAnalyzeResponse, SDKError> {
        let mut client = self.vision_client().await?;
        Ok(client.batch_analyze(request).await?.into_inner())
    }

    /// Searches for web copies of an image using Vision's IMAGE_COPY_SEARCH feature.
    ///
    /// `folder_id` is required when authenticating as a user; pass an empty string
    /// for service-account auth where the folder is inferred from the token.
    pub async fn vision_image_copy_search(
        &self,
        content: Vec<u8>,
        folder_id: impl Into<String>,
    ) -> Result<ImageCopySearchAnnotation, SDKError> {
        let response = self
            .vision_batch_analyze(BatchAnalyzeRequest {
                analyze_specs: vec![AnalyzeSpec {
                    source: Some(Source::Content(content)),
                    features: vec![Feature {
                        r#type: FeatureType::ImageCopySearch as i32,
                        config: None,
                    }],
                    mime_type: String::new(),
                }],
                folder_id: folder_id.into(),
            })
            .await?;

        let result = response
            .results
            .into_iter()
            .next()
            .ok_or_else(|| SDKError::Internal("vision returned no results".into()))?;

        if let Some(err) = result.error {
            return Err(SDKError::Internal(format!(
                "vision error {}: {}",
                err.code, err.message
            )));
        }

        result
            .results
            .into_iter()
            .find_map(|r| {
                if let Some(FeatureVariant::ImageCopySearch(annotation)) = r.feature {
                    Some(annotation)
                } else {
                    None
                }
            })
            .ok_or_else(|| SDKError::Internal("no image copy search result in response".into()))
    }
}
