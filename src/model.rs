use image::ImageFormat;
use object_store::ObjectStore;
use object_store::path::Path;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ImageThumbs<T> {
    pub(crate) client: Arc<RwLock<T>>,
    pub(crate) settings: Arc<Vec<Params>>,
}

impl<T> ImageThumbs<T> {
    pub fn with_store(object_store: T, settings: Vec<Params>) -> Self
    where
        T: ObjectStore,
    {
        Self {
            client: Arc::new(RwLock::new(object_store)),
            settings: Arc::new(settings),
        }
    }
}

impl<T> Clone for ImageThumbs<T> {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            settings: Arc::clone(&self.settings),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Params {
    /// Can be used as `{thumb_name}` in the `naming_pattern`.
    /// If the naming_pattern is not explicitly given, the default is
    /// "`/{thumb_name}/{image_name}.{image_extension}`"
    pub name: String,
    pub naming_pattern: Option<String>,
    /// PNG ignores this variable as it is always lossless
    pub quality: u8,
    pub size: (u32, u32),
    pub mode: Mode,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// The image's aspect ratio is preserved. The image is scaled to the maximum possible size that
    /// fits within the bounds.
    Fit,
    /// The image's aspect ratio is preserved. The image is scaled to the maximum possible size that
    /// fits within the larger (relative to aspect ratio) of the bounds, then cropped to fit within
    /// the other bound.
    Crop,
}

#[derive(Debug)]
pub(crate) struct ImageDetails {
    /// image filename without path and extension
    pub(crate) stem: String,
    pub(crate) format: ImageFormat,
    pub(crate) path: Path,
    pub(crate) bytes: Vec<u8>,
}
