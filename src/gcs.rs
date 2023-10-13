use object_store::gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder};

use crate::{ImageThumbs, ThumbsResult};

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
        let client = GoogleCloudStorageBuilder::from_env()
            .with_client_options(Self::client_options())
            .build()?;

        Ok(Self {
            client,
            settings: Self::settings(config)?,
        })
    }
}
