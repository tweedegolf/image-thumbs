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

#[derive(Debug)]
struct ImageDetails {
    stem: String,
    format: ImageFormat,
    path: Path,
    bytes: Vec<u8>,
}

impl<T: ObjectStore> ImageThumbs<T> {
    /// Get image from object storage, create thumbnails, and put them in the `dest_dir` directory
    pub async fn create_thumbs_dir(
        &self,
        directory: Option<&str>,
        dest_dir: &str,
        force_override: bool,
    ) -> ThumbsResult<()> {
        let prefix = match directory {
            Some(p) => Some(Path::parse(p)?),
            None => None,
        };

        let names = self.list_folder(prefix.as_ref()).await?;
        let existent_thumbs = self.list_folder(Some(&Path::parse(dest_dir)?)).await?;
        let images_to_thumbnail = self.filter_existent_thumbs(names, &existent_thumbs)?;

        for name in images_to_thumbnail {
            self.create_thumbs(name.as_ref(), dest_dir, force_override)
                .await?;
        }
        Ok(())
    }

    pub async fn create_thumbs(
        &self,
        file: &str,
        dest_dir: &str,
        force_override: bool,
    ) -> ThumbsResult<()> {
        let image = self.download_image(file).await?;
        self.create_thumbs_dest_from_bytes(
            image.bytes,
            dest_dir,
            &image.stem,
            image.format,
            force_override,
        )
        .await
    }

    pub async fn create_thumbs_dest_from_bytes(
        &self,
        bytes: Vec<u8>,
        dest_dir: &str,
        image_name: &str,
        format: ImageFormat,
        force_override: bool,
    ) -> ThumbsResult<()> {
        let dest_dir = Path::parse(dest_dir)?;

        let thumbs = self
            .create_thumbs_from_bytes(bytes, dest_dir, image_name, format, force_override)
            .await?;
        self.upload_thumbs(thumbs).await
    }
}

#[cfg(test)]
mod tests {
    use image::ImageFormat;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use tokio::fs::File;
    use tokio::io::{AsyncReadExt, BufReader};

    use crate::{ImageDetails, ImageThumbs};

    #[tokio::test]
    async fn from_cloud() {
        let client = ImageThumbs::new("src/test/image_thumbs").await.unwrap();
        create_thumbs(&client).await;
        create_thumbs_dir(&client).await;
        create_thumbs_from_bytes(&client).await;
        override_behaviour(&client).await;
    }

    async fn create_thumbs<T: ObjectStore>(client: &ImageThumbs<T>) {
        client
            .create_thumbs("penguin.jpg", "/test_dir", false)
            .await
            .unwrap();
        client
            .create_thumbs("penguin.png", "/test_dir", false)
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
        client
            .create_thumbs_dir(None, "thumbs", false)
            .await
            .unwrap();

        // check if they exist
        client
            .download_image("thumbs/penguin_standard.jpg")
            .await
            .unwrap();
        client
            .download_image("thumbs/penguin_mini.jpg")
            .await
            .unwrap();
        client
            .download_image("thumbs/penguin_standard.png")
            .await
            .unwrap();
        client
            .download_image("thumbs/penguin_mini.png")
            .await
            .unwrap();

        // delete them to not influence following test
        client.delete("thumbs/penguin_standard.jpg").await.unwrap();
        client.delete("thumbs/penguin_mini.jpg").await.unwrap();
        client.delete("thumbs/penguin_standard.png").await.unwrap();
        client.delete("thumbs/penguin_mini.png").await.unwrap();

        client
            .create_thumbs_dir(Some("/"), "thumbs", false)
            .await
            .unwrap();

        // check if they exist
        client
            .download_image("thumbs/penguin_standard.jpg")
            .await
            .unwrap();
        client
            .download_image("thumbs/penguin_mini.jpg")
            .await
            .unwrap();
        client
            .download_image("thumbs/penguin_standard.png")
            .await
            .unwrap();
        client
            .download_image("thumbs/penguin_mini.png")
            .await
            .unwrap();

        // delete them to not influence following test
        client.delete("thumbs/penguin_standard.jpg").await.unwrap();
        client.delete("thumbs/penguin_mini.jpg").await.unwrap();
        client.delete("thumbs/penguin_standard.png").await.unwrap();
        client.delete("thumbs/penguin_mini.png").await.unwrap();
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
                    false,
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
                    false,
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

    async fn override_behaviour<T: ObjectStore>(client: &ImageThumbs<T>) {
        let broken_thumb = ImageDetails {
            stem: "penguin_standard".to_string(),
            format: ImageFormat::Png,
            path: Path::parse("/thumbs").unwrap(),
            bytes: vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
        };
        client.upload_thumbs(vec![broken_thumb]).await.unwrap();

        client
            .create_thumbs_dir(Some("/"), "thumbs", false)
            .await
            .unwrap();

        client
            .download_image("thumbs/penguin_standard.jpg")
            .await
            .unwrap();
        client
            .download_image("thumbs/penguin_mini.jpg")
            .await
            .unwrap();
        assert_eq!(
            client
                .download_image("thumbs/penguin_standard.png")
                .await
                .unwrap()
                .bytes,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            "This image should not be overwritten"
        );
        client
            .download_image("thumbs/penguin_mini.png")
            .await
            .unwrap();

        client
            .create_thumbs_dir(Some("/"), "thumbs", true)
            .await
            .unwrap();

        assert_ne!(
            client
                .download_image("thumbs/penguin_standard.png")
                .await
                .unwrap()
                .bytes,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            "The image should have been overwritten"
        );

        // delete them to not influence following test
        client.delete("thumbs/penguin_standard.jpg").await.unwrap();
        client.delete("thumbs/penguin_mini.jpg").await.unwrap();
        client.delete("thumbs/penguin_standard.png").await.unwrap();
        client.delete("thumbs/penguin_mini.png").await.unwrap();
    }
}
