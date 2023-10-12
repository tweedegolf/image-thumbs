use ::image::{ImageError, ImageFormat};
use object_store::path::Path;
use object_store::ObjectStore;
use serde::Deserialize;
use thiserror::Error;

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

struct ImageDetails {
    stem: String,
    format: ImageFormat,
    path: Path,
    bytes: Vec<u8>,
}

impl<T: ObjectStore> ImageThumbs<T> {
    /// Get image from object storage, create thumbnails, and put them back in the same location
    pub async fn create_thumbs(&self, file: &str) -> ThumbsResult<()> {
        let image = self.download_image(file).await?;
        self.create_thumbs_dest_from_bytes(
            image.bytes,
            image.path.as_ref(),
            &image.stem,
            image.format,
        )
        .await
    }

    pub async fn create_thumbs_dir(&self, path: Option<&str>) -> ThumbsResult<()> {
        let os_p;
        let os_path = match path {
            Some(p) => {
                os_p = Path::from_filesystem_path(p)?;
                Some(&os_p)
            }
            None => None,
        };
        let names = self.list_folder(os_path).await?;

        for name in names {
            self.create_thumbs(name.as_ref()).await?;
        }
        Ok(())
    }

    pub async fn create_thumbs_dest(&self, file: &str, dest_dir: &str) -> ThumbsResult<()> {
        let image = self.download_image(file).await?;
        self.create_thumbs_dest_from_bytes(image.bytes, dest_dir, &image.stem, image.format)
            .await
    }

    pub async fn create_thumbs_dest_from_bytes(
        &self,
        bytes: Vec<u8>,
        dest_dir: &str,
        image_name: &str,
        format: ImageFormat,
    ) -> ThumbsResult<()> {
        let dest_dir = Path::parse(dest_dir)?;

        let thumbs = self.create_thumbs_from_bytes(bytes, dest_dir, image_name, format)?;
        self.upload_thumbs(thumbs).await
    }
}

#[cfg(test)]
mod tests {
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
        client.create_thumbs("penguin.jpg").await.unwrap();
        client.create_thumbs("penguin.png").await.unwrap();

        // check if they exist
        client.download_image("penguin_standard.jpg").await.unwrap();
        client.download_image("penguin_mini.jpg").await.unwrap();
        client.download_image("penguin_standard.png").await.unwrap();
        client.download_image("penguin_mini.png").await.unwrap();

        // delete them to not influence following test
        client.delete("penguin_standard.jpg").await.unwrap();
        client.delete("penguin_mini.jpg").await.unwrap();
        client.delete("penguin_standard.png").await.unwrap();
        client.delete("penguin_mini.png").await.unwrap();
    }

    async fn create_thumbs_dest<T: ObjectStore>(client: &ImageThumbs<T>) {
        client
            .create_thumbs_dest("penguin.jpg", "/test_dir")
            .await
            .unwrap();
        client
            .create_thumbs_dest("penguin.png", "/test_dir")
            .await
            .unwrap();

        // check if they exist
        client
            .download_image("test_dir/penguin_standard.jpg")
            .await
            .unwrap();
        client
            .download_image("test_dir/penguin_mini.jpg")
            .await
            .unwrap();
        client
            .download_image("test_dir/penguin_standard.png")
            .await
            .unwrap();
        client
            .download_image("test_dir/penguin_mini.png")
            .await
            .unwrap();

        // delete them to not influence following test
        client
            .delete("test_dir/penguin_standard.jpg")
            .await
            .unwrap();
        client.delete("test_dir/penguin_mini.jpg").await.unwrap();
        client
            .delete("test_dir/penguin_standard.png")
            .await
            .unwrap();
        client.delete("test_dir/penguin_mini.png").await.unwrap();
    }

    async fn create_thumbs_dir<T: ObjectStore>(client: &ImageThumbs<T>) {
        client.create_thumbs_dir(None).await.unwrap();

        // check if they exist
        client.download_image("penguin_standard.jpg").await.unwrap();
        client.download_image("penguin_mini.jpg").await.unwrap();
        client.download_image("penguin_standard.png").await.unwrap();
        client.download_image("penguin_mini.png").await.unwrap();

        // delete them to not influence following test
        client.delete("penguin_standard.jpg").await.unwrap();
        client.delete("penguin_mini.jpg").await.unwrap();
        client.delete("penguin_standard.png").await.unwrap();
        client.delete("penguin_mini.png").await.unwrap();

        client.create_thumbs_dir(Some("/")).await.unwrap();

        // check if they exist
        client.download_image("penguin_standard.jpg").await.unwrap();
        client.download_image("penguin_mini.jpg").await.unwrap();
        client.download_image("penguin_standard.png").await.unwrap();
        client.download_image("penguin_mini.png").await.unwrap();

        // delete them to not influence following test
        client.delete("penguin_standard.jpg").await.unwrap();
        client.delete("penguin_mini.jpg").await.unwrap();
        client.delete("penguin_standard.png").await.unwrap();
        client.delete("penguin_mini.png").await.unwrap();
    }

    async fn create_thumbs_from_bytes<T: ObjectStore>(client: &ImageThumbs<T>) {
        // create JPG image thumbs
        {
            let test_jpg = File::open("src/test/mock_data/testBucket/penguin.jpg")
                .await
                .unwrap();
            let mut reader = BufReader::new(test_jpg);
            let mut buffer = Vec::new();

            reader.read_to_end(&mut buffer).await.unwrap();

            client
                .create_thumbs_dest_from_bytes(
                    buffer,
                    "/from_bytes_test",
                    "penguin",
                    ImageFormat::Jpeg,
                )
                .await
                .unwrap();
        }

        // create PNG image thumbs
        {
            let test_png = File::open("src/test/mock_data/testBucket/penguin.png")
                .await
                .unwrap();
            let mut reader = BufReader::new(test_png);
            let mut buffer = Vec::new();

            reader.read_to_end(&mut buffer).await.unwrap();

            client
                .create_thumbs_dest_from_bytes(
                    buffer,
                    "/from_bytes_test",
                    "penguin",
                    ImageFormat::Png,
                )
                .await
                .unwrap();
        }

        // check if they exist
        client
            .download_image("from_bytes_test/penguin_standard.png")
            .await
            .unwrap();
        client
            .download_image("from_bytes_test/penguin_mini.png")
            .await
            .unwrap();
        client
            .download_image("from_bytes_test/penguin_standard.png")
            .await
            .unwrap();
        client
            .download_image("from_bytes_test/penguin_mini.png")
            .await
            .unwrap();

        // delete them to not influence following test
        client
            .delete("from_bytes_test/penguin_standard.jpg")
            .await
            .unwrap();
        client
            .delete("from_bytes_test/penguin_mini.jpg")
            .await
            .unwrap();
        client
            .delete("from_bytes_test/penguin_standard.png")
            .await
            .unwrap();
        client
            .delete("from_bytes_test/penguin_mini.png")
            .await
            .unwrap();
    }
}
