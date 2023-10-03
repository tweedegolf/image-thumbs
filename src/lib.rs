use std::path::Path;

use ::image::{ImageError, ImageFormat};
use futures::TryStreamExt;
use object_store::{DynObjectStore, ObjectStore};
use serde::Deserialize;
use thiserror::Error;
use tokio::sync::mpsc;

mod gcs;
mod image;
mod storage;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Storage error: {0}")]
    Storage(#[from] object_store::Error),
    #[error("Invalid path: {0}")]
    Path(#[from] object_store::path::Error),
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Image error: {0}")]
    Image(#[from] ImageError),
    #[error("Image format not supported")]
    NotSupported,
    #[error("Utf-8 error")]
    Utf,
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
    mode: Mode,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Fit,
    Crop,
}

struct ImageDetails<'a> {
    stem: &'a str,
    format: ImageFormat,
    path: &'a Path,
    bytes: Vec<u8>,
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
        let image = self.download_image(file).await?;
        self.create_thumbs_dest_from_bytes(image.bytes, image.path, image.stem, image.format)
            .await
    }

    pub async fn create_thumbs_dir(&self, path: Option<&Path>) -> ThumbsResult<()> {
        let os_p;
        let os_path = match path {
            Some(p) => {
                os_p = object_store::path::Path::from_filesystem_path(p)?;
                Some(&os_p)
            }
            None => None,
        };
        let names = flatten_list_stream(&self.client, os_path).await?;

        for name in names {
            self.create_thumbs(Path::new(&name.to_string())).await?;
        }
        Ok(())
    }

    pub async fn create_thumbs_dest(&self, file: &Path, dest_dir: &Path) -> ThumbsResult<()> {
        let image = self.download_image(file).await?;
        self.create_thumbs_dest_from_bytes(image.bytes, dest_dir, image.stem, image.format)
            .await
    }

    pub async fn create_thumbs_dest_from_bytes(
        &self,
        bytes: Vec<u8>,
        dest_dir: &Path,
        image_name: &str,
        format: ImageFormat,
    ) -> ThumbsResult<()> {
        let (sender, receiver) = mpsc::channel(2);

        self.create_thumbs_from_bytes(bytes, format, sender)?;
        self.upload_thumbs(receiver, dest_dir, image_name.to_string())
            .await
    }
}
