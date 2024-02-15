use object_store::gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder};

use crate::model::Params;
use crate::{ImageThumbs, ThumbsResult};

impl ImageThumbs<GoogleCloudStorage> {
    /// Creates new ImageThumbs instance connected to Google Cloud Storage using the environment
    /// variables `GOOGLE_BUCKET` and `GOOGLE_SERVICE_ACCOUNT_KEY` to connect to GCS.
    /// The later should be in the JSON format.
    ///
    /// Reads the config YAML file to know which thumbnails to create
    ///
    /// The config file must look like the example in `examples/image_thumbs.yaml`:
    /// ```yaml
    #[doc = include_str!("../examples/image_thumbs.yaml")]
    /// ```
    ///
    /// # Arguments
    /// * `config` - Path to the config file from the crate root (`.yaml` may be omitted)
    pub async fn new(config: &str) -> ThumbsResult<Self> {
        let client = GoogleCloudStorageBuilder::from_env()
            .with_client_options(Self::client_options())
            .build()?;

        Ok(Self {
            client,
            settings: Self::settings(config)?,
        })
    }

    pub async fn new_with_settings(settings: Vec<Params>) -> ThumbsResult<Self> {
        let client = GoogleCloudStorageBuilder::from_env()
            .with_client_options(Self::client_options())
            .build()?;

        Ok(Self { client, settings })
    }
}
