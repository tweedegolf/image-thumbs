use crate::{ImageThumbs, ThumbsResult};
use config::Config;
use object_store::gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder};
use object_store::ClientOptions;

impl ImageThumbs<GoogleCloudStorage> {
    /// Create new ImageThumbs instance connected to Google Cloud Storage using the environment
    /// variables `GOOGLE_BUCKET` and `GOOGLE_SERVICE_ACCOUNT_KEY` to connect to GCS.
    ///
    /// Reads the config YAML file to know which thumbnails to create
    ///
    /// TODO document config file structure
    ///
    /// # Arguments
    /// * config - Path to the config file from the crate root (`.yaml` may be omitted)
    pub async fn new(config: &str) -> ThumbsResult<Self> {
        let settings = Config::builder()
            .add_source(config::File::with_name(config))
            .build()?
            .get("thumbs")?;

        #[allow(unused_mut)]
        let mut client_options = ClientOptions::new()
            .with_content_type_for_suffix("jpg", mime::IMAGE_JPEG.to_string())
            .with_content_type_for_suffix("jpeg", mime::IMAGE_JPEG.to_string())
            .with_content_type_for_suffix("png", mime::TEXT_PLAIN.to_string());

        #[cfg(test)]
        {
            client_options = client_options.with_allow_http(true);
        }

        let client = GoogleCloudStorageBuilder::from_env()
            .with_client_options(client_options)
            .build()?;

        Ok(Self { client, settings })
    }
}
