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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use image::ImageFormat;
    use object_store::ObjectStore;
    use tokio::fs::File;
    use tokio::io::{AsyncReadExt, BufReader};

    use crate::ImageThumbs;

    #[tokio::test]
    async fn from_cloud() {
        let client = ImageThumbs::new("src/test/image_thumbs").await.unwrap();
        create_thumbs(&client).await;
        create_thumbs_dest(&client).await;
        create_thumbs_dir(&client).await;
        create_thumbs_from_bytes(&client).await;
    }

    async fn create_thumbs<T: ObjectStore>(client: &ImageThumbs<T>) {
        // create thumbnails
        client.create_thumbs(Path::new("test.jpg")).await.unwrap();
        client
            .create_thumbs(Path::new("penguin.png"))
            .await
            .unwrap();

        // check if they exist
        client
            .download_image(Path::new("test_standard.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("test_mini.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("penguin_standard.png"))
            .await
            .unwrap();
        client
            .download_image(Path::new("penguin_mini.png"))
            .await
            .unwrap();

        // delete them to not influence following test
        client.delete(Path::new("test_standard.jpg")).await.unwrap();
        client.delete(Path::new("test_mini.jpg")).await.unwrap();
        client
            .delete(Path::new("penguin_standard.png"))
            .await
            .unwrap();
        client.delete(Path::new("penguin_mini.png")).await.unwrap();
    }

    async fn create_thumbs_dest<T: ObjectStore>(client: &ImageThumbs<T>) {
        client
            .create_thumbs_dest(Path::new("test.jpg"), Path::new("/test_dir"))
            .await
            .unwrap();
        client
            .create_thumbs_dest(Path::new("penguin.png"), Path::new("/test_dir"))
            .await
            .unwrap();

        // check if they exist
        client
            .download_image(Path::new("test_dir/test_standard.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("test_dir/test_mini.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("test_dir/penguin_standard.png"))
            .await
            .unwrap();
        client
            .download_image(Path::new("test_dir/penguin_mini.png"))
            .await
            .unwrap();

        // delete them to not influence following test
        client
            .delete(Path::new("test_dir/test_standard.jpg"))
            .await
            .unwrap();
        client
            .delete(Path::new("test_dir/test_mini.jpg"))
            .await
            .unwrap();
        client
            .delete(Path::new("test_dir/penguin_standard.png"))
            .await
            .unwrap();
        client
            .delete(Path::new("test_dir/penguin_mini.png"))
            .await
            .unwrap();
    }

    async fn create_thumbs_dir<T: ObjectStore>(client: &ImageThumbs<T>) {
        client.create_thumbs_dir(None).await.unwrap();

        // check if they exist
        client
            .download_image(Path::new("test_standard.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("test_mini.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("penguin_standard.png"))
            .await
            .unwrap();
        client
            .download_image(Path::new("penguin_mini.png"))
            .await
            .unwrap();

        // delete them to not influence following test
        client.delete(Path::new("test_standard.jpg")).await.unwrap();
        client.delete(Path::new("test_mini.jpg")).await.unwrap();
        client
            .delete(Path::new("penguin_standard.png"))
            .await
            .unwrap();
        client.delete(Path::new("penguin_mini.png")).await.unwrap();

        client
            .create_thumbs_dir(Some(Path::new("/")))
            .await
            .unwrap();

        // check if they exist
        client
            .download_image(Path::new("test_standard.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("test_mini.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("penguin_standard.png"))
            .await
            .unwrap();
        client
            .download_image(Path::new("penguin_mini.png"))
            .await
            .unwrap();

        // delete them to not influence following test
        client.delete(Path::new("test_standard.jpg")).await.unwrap();
        client.delete(Path::new("test_mini.jpg")).await.unwrap();
        client
            .delete(Path::new("penguin_standard.png"))
            .await
            .unwrap();
        client.delete(Path::new("penguin_mini.png")).await.unwrap();
    }

    async fn create_thumbs_from_bytes<T: ObjectStore>(client: &ImageThumbs<T>) {
        // create JPG image thumbs
        {
            let test_jpg = File::open("src/test/test.jpg").await.unwrap();
            let mut reader = BufReader::new(test_jpg);
            let mut buffer = Vec::new();

            reader.read_to_end(&mut buffer).await.unwrap();

            client
                .create_thumbs_dest_from_bytes(
                    buffer,
                    Path::new("/from_bytes_test"),
                    "test",
                    ImageFormat::Jpeg,
                )
                .await
                .unwrap();
        }

        // create PNG image thumbs
        {
            let test_jpg = File::open("src/test/penguin.png").await.unwrap();
            let mut reader = BufReader::new(test_jpg);
            let mut buffer = Vec::new();

            reader.read_to_end(&mut buffer).await.unwrap();

            client
                .create_thumbs_dest_from_bytes(
                    buffer,
                    Path::new("/from_bytes_test"),
                    "penguin",
                    ImageFormat::Png,
                )
                .await
                .unwrap();
        }

        // check if they exist
        client
            .download_image(Path::new("from_bytes_test/test_standard.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("from_bytes_test/test_mini.jpg"))
            .await
            .unwrap();
        client
            .download_image(Path::new("from_bytes_test/penguin_standard.png"))
            .await
            .unwrap();
        client
            .download_image(Path::new("from_bytes_test/penguin_mini.png"))
            .await
            .unwrap();

        // delete them to not influence following test
        client
            .delete(Path::new("from_bytes_test/test_standard.jpg"))
            .await
            .unwrap();
        client
            .delete(Path::new("from_bytes_test/test_mini.jpg"))
            .await
            .unwrap();
        client
            .delete(Path::new("from_bytes_test/penguin_standard.png"))
            .await
            .unwrap();
        client
            .delete(Path::new("from_bytes_test/penguin_mini.png"))
            .await
            .unwrap();
    }
}
