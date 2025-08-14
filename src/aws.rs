use crate::{ImageThumbs, ThumbsResult, model::Params};
use object_store::aws::{AmazonS3, AmazonS3Builder};
use std::sync::Arc;
use tokio::sync::RwLock;

impl ImageThumbs<AmazonS3> {
    /// Creates new ImageThumbs instance connected to Amazon S3 using the environment
    /// variables `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` and `AWS_SECRET_ACCESS_KEY` to connect to S3.
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
    pub fn new(config: &str) -> ThumbsResult<Self> {
        let bucket = std::env::var("AWS_BUCKET").expect("AWS_BUCKET must be set");

        let client = AmazonS3Builder::from_env()
            .with_client_options(Self::client_options())
            .with_bucket_name(bucket)
            .build()?;

        Ok(Self {
            client: Arc::new(RwLock::new(client)),
            settings: Arc::new(Self::settings(config)?),
        })
    }

    pub fn new_with_settings(settings: Vec<Params>) -> ThumbsResult<Self> {
        let bucket = std::env::var("AWS_BUCKET").expect("AWS_BUCKET must be set");

        let client = AmazonS3Builder::from_env()
            .with_client_options(Self::client_options())
            .with_bucket_name(bucket)
            .build()?;

        Ok(Self {
            client: Arc::new(RwLock::new(client)),
            settings: Arc::new(settings),
        })
    }
}
