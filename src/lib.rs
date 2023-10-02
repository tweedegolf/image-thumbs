use std::ffi::OsStr;
use std::path::Path;

use ::image::{ImageError, ImageFormat, load_from_memory_with_format};
use ::image::imageops::FilterType;
use config::Config;
use object_store::gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder};
use object_store::{ClientOptions, DynObjectStore, ObjectStore};
use serde::Deserialize;
use thiserror::Error;
use futures::TryStreamExt;

use tokio::sync::mpsc;

mod gcs;
mod image;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Storage error: {0}")]
    Storage(#[from ]  object_store::Error),
    #[error("Invalid path: {0}")]
    Path(#[from] object_store::path::Error),
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Image error: {0}")]
    Image(#[from] ImageError)
}

pub type ThumbsResult<T> = Result<T, Error>;

#[derive(Debug)]
pub struct ImageThumbs<T: ObjectStore> {
    client: T,
    settings: Vec<Params>,
}

#[derive(Deserialize, Debug, Clone)]
struct Params {
    name: String,
    quality: u8,
    size: (u32, u32),
    mode: Mode
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Fit,
    Crop,
}

impl ImageThumbs<GoogleCloudStorage> {
    /// Create new ImageThumbs instance connected to Google Cloud Storage using the environment
    /// variables `BUCKET_NAME` and `GOOGLE_SERVICE_ACCOUNT_KEY` to connect to GCS.
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

        let mut client_options = ClientOptions::new()
            .with_content_type_for_suffix("jpg", mime::IMAGE_JPEG.to_string())
            .with_content_type_for_suffix("jpeg", mime::IMAGE_JPEG.to_string())
            .with_content_type_for_suffix("png", mime::TEXT_PLAIN.to_string());

        client_options = client_options.with_allow_http(true); // FIXME allow this only for testing

        let client = GoogleCloudStorageBuilder::from_env().with_client_options(client_options).build()?;

        Ok(Self {
            client,
            settings,
        })
    }
}

async fn flatten_list_stream(
    storage: &DynObjectStore,
    prefix: Option<&object_store::path::Path>,
) -> ThumbsResult<Vec<object_store::path::Path>> {
    Ok(storage
        .list(prefix)
        .await?
        .map_ok(|meta| meta.location)
        .try_collect::<Vec<object_store::path::Path>>()
        .await?)
}

impl<T: ObjectStore> ImageThumbs<T> {
    /// Get image from object storage, create thumbnails, and put them back in the same location
    pub async fn create_thumbs(&self, file: &Path) -> ThumbsResult<()> {
        self.create_thumbs_dest(file, file.parent().unwrap_or(Path::new(""))).await
    }


    pub async fn create_thumbs_dest(&self, file: &Path, dest_dir: &Path) -> ThumbsResult<()> {
        let result = self.client.get(&object_store::path::Path::parse(file.to_str().unwrap())?).await?;
        let image_format = ImageFormat::from_path(result.meta.location.as_ref())?;
        let image_stem = file.file_stem().unwrap_or(OsStr::new("unknown")).to_str().unwrap_or("unknown");

        let bytes = result.bytes().await?;

        let (sender, receiver) = mpsc::channel(2);

        self.create_thumbs_from_bytes(bytes.as_ref(), image_format, sender)?;
        self.upload_thumbs(receiver, dest_dir, image_stem.to_string()).await

    }

    fn create_thumbs_from_bytes(&self, bytes: &[u8], format: ImageFormat, sender: mpsc::Sender<(Vec<u8>, String, ImageFormat)>) -> ThumbsResult<()> {
        let image = load_from_memory_with_format(bytes, format)?;

        let settings = self.settings.clone();

        tokio::task::spawn(async move {
        for params in settings.iter() {
            let thumbnail = match params.mode {
                Mode::Fit => image.thumbnail(params.size.0, params.size.1),
                Mode::Crop => image.resize_to_fill(params.size.0, params.size.1, FilterType::Nearest)
            };
            sender.send((thumbnail.into_bytes(), params.name.clone(), format)).await.unwrap();
        }});

        Ok(())
    }

    async fn upload_thumbs(&self, mut image_channel: mpsc::Receiver<(Vec<u8>, String, ImageFormat)>, path: &Path, name: String) -> ThumbsResult<()> {

        while let Some((bytes, thumb_name, image_format)) = image_channel.recv().await {
            let path = format!("{}/{}_{}.{}", path.to_str().unwrap(), name, thumb_name, image_format.extensions_str()[0]);

            self.client.put(&object_store::path::Path::parse(path)?, bytes.into()).await?;
        }

        Ok(())
    }
}