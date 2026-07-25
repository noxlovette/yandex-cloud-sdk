use crate::{
    Client, SDKError,
    yandex::cloud::ai::ocr::v1::{
        RecognizeTextRequest, RecognizeTextResponse,
        recognize_text_request::Source,
    },
};

impl Client {
    /// Sends image bytes to the OCR service and collects all streamed page responses.
    ///
    /// `mime_type` should be `"image/jpeg"`, `"image/png"`, or `"application/pdf"`.
    /// Pass an empty `model` string to use the server default (`"page"`).
    pub async fn ocr_recognize(
        &self,
        content: Vec<u8>,
        mime_type: impl Into<String>,
        language_codes: Vec<String>,
        model: impl Into<String>,
    ) -> Result<Vec<RecognizeTextResponse>, SDKError> {
        let mut client = self.ocr_text_recognition_client().await?;

        let mut stream = client
            .recognize(RecognizeTextRequest {
                source: Some(Source::Content(content)),
                mime_type: mime_type.into(),
                language_codes,
                model: model.into(),
            })
            .await?
            .into_inner();

        let mut pages = Vec::new();
        while let Some(page) = stream.message().await? {
            pages.push(page);
        }

        Ok(pages)
    }

    /// Extracts the full plain-text content from an image or PDF.
    ///
    /// Calls `ocr_recognize` with the `"page"` model and no language hints, then
    /// assembles the final string from the per-page `full_text` fields.
    pub async fn ocr_extract_text(
        &self,
        content: Vec<u8>,
        mime_type: impl Into<String>,
    ) -> Result<String, SDKError> {
        let pages = self
            .ocr_recognize(content, mime_type, Vec::new(), "page")
            .await?;

        // TODO: implement multi-page text assembly
        //
        // `pages` is a Vec<RecognizeTextResponse> — each element has:
        //   page.page                              → i64 page index (0-based for images)
        //   page.text_annotation.full_text         → complete recognised text for that page
        //   page.text_annotation.blocks / .lines   → structured layout if you need coordinates
        //
        // Implement the body (5-10 lines). Some trade-offs to consider:
        //   • Join with "\n\n" for readable multi-page docs, or "\n" for tight output.
        //   • Sort by `page.page` first if the stream order is not guaranteed.
        //   • Return an error (SDKError::Internal) if `pages` is empty.
        todo!("assemble text from pages")
    }
}
