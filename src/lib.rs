//! # Image Thumbs
//! Easy-to-use library to create image thumbnails from images existing on some (cloud) object
//! storage or from disk.
//!
//! Currently implemented is a connection to Google Cloud Storage,
//! but it can be easily extended to other providers.
//!
//! ## Supported formats
//! PNG and JPEG are currently the only supported image formats.
//!
//! # How to use
//! ## Sizes
//! Configure what thumbnails you would like to have in a .yaml file:
//! ```yaml
#![doc = include_str!("../examples/image_thumbs.yaml")]
//! ```
//!
//! ## Google credentials
//! This crate relies on [object_store](https://crates.io/crates/object_store) for the interaction
//! with the storage backend.
//! Currently, this crate only supports Google Cloud Storage.
//!
//! To configure the Google Service Account, use one of the following environment variables as
//! [described in the object_store](https://docs.rs/object_store/0.9.0/object_store/gcp/struct.GoogleCloudStorageBuilder.html#method.from_env)
//! crate.
//!
//! ```text
//! GOOGLE_SERVICE_ACCOUNT: location of service account file
//! GOOGLE_SERVICE_ACCOUNT_PATH: (alias) location of service account file
//! SERVICE_ACCOUNT: (alias) location of service account file
//! GOOGLE_SERVICE_ACCOUNT_KEY: JSON serialized service account key
//! GOOGLE_BUCKET: bucket name
//! GOOGLE_BUCKET_NAME: (alias) bucket name
//! ```
//!
//! Then use it in your code
//! ```no_run
//! # #[tokio::main]
//! # async fn main() {
//!     // Path to your thumbnail configuration yaml. You may specify the .yaml extension in the
//!     // path, but you don't need to.
//!     let thumbs = image_thumbs::ImageThumbs::new("examples/image_thumbs")
//!         .await
//!         .unwrap();
//!     thumbs
//!         .create_thumbs("penguin.jpg", "/thumbs", false)
//!         .await
//!         .unwrap();
//!     thumbs
//!         .create_thumbs("penguin.png", "/thumbs", false)
//!         .await
//!         .unwrap();
//! # }
//! ```

use ::image::ImageFormat;
use config::Config;
use object_store::path::Path;
use object_store::ObjectStore;
use thiserror::Error;

pub use crate::error::Error;
pub use crate::error::ThumbsResult;
pub use crate::model::ImageThumbs;
use crate::model::Params;

mod error;
mod gcs;
mod image;
mod model;
mod storage;

impl<T: ObjectStore> ImageThumbs<T> {
    /// Gets all images from one object storage level, creates thumbnails for each of them, and puts
    /// them in the `dest_dir` directory.
    ///
    /// # Arguments
    /// * `directory` - directory to create thumbnails for.
    /// It will list all objects on this level and create thumbnails (if they do not already exist).
    ///
    /// * `dest_dir` - directory to store all created thumbnails.
    /// This directory will be checked for already existent thumbnails, if `force_override` is false.
    ///
    /// * `force_override` - if `true` it will override already existent files with the same name.
    /// If false, it will preserve already existent files.
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

        let mut names = self.list_folder(prefix.as_ref()).await?;

        if force_override {
            let existent_thumbs = self.list_folder(Some(&Path::parse(dest_dir)?)).await?;
            names = self.filter_existent_thumbs(names, &existent_thumbs)?;
        }

        for name in names {
            self.create_thumbs(name.as_ref(), dest_dir, force_override)
                .await?;
        }
        Ok(())
    }

    /// Gets one image from the object storage, creates thumbnails for it, and puts them in the
    /// `dest_dir` directory.
    ///
    /// # Arguments
    /// * `file` - image to create thumbnails for.
    ///
    /// * `dest_dir` - directory to store all created thumbnails.
    /// This directory will be checked for already existent thumbnails if `force_override` is false.
    ///
    /// * `force_override` - if `true` it will override already existent files with the same name.
    /// If false, it will preserve already existent files.
    pub async fn create_thumbs(
        &self,
        file: &str,
        dest_dir: &str,
        force_override: bool,
    ) -> ThumbsResult<()> {
        let image = self.download_image(file).await?;
        self.create_thumbs_from_bytes(
            image.bytes,
            dest_dir,
            &image.stem,
            image.format,
            force_override,
        )
        .await
    }

    /// Takes the raw bytes of an image, creates thumbnails for it, and puts them in the `dest_dir`
    /// directory.
    ///
    /// # Arguments
    /// * `bytes` - raw image bytes to create thumbnails for.
    ///
    /// * `dest_dir` - directory to store all created thumbnails.
    /// This directory will be checked for already existent thumbnails, if `force_override` is false.
    ///
    /// * `image_name` - name used for the created thumbnails. Should not include the extension.
    /// Final thumbnail names will be of the form `<image_name>_<thumbnail_name>.<extension>`
    ///
    /// * `format` - format of the input image. The output image will have the same type.
    /// Currently supported are JPG and PNG.
    ///
    /// * `force_override` - if `true` it will override already existent files with the same name.
    /// If false, it will preserve already existent files.
    pub async fn create_thumbs_from_bytes(
        &self,
        bytes: Vec<u8>,
        dest_dir: &str,
        image_name: &str,
        format: ImageFormat,
        force_override: bool,
    ) -> ThumbsResult<()> {
        let dest_dir = Path::parse(dest_dir)?;

        let thumbs = self
            .create_thumb_images_from_bytes(bytes, dest_dir, image_name, format, force_override)
            .await?;
        self.upload_thumbs(thumbs).await
    }

    /// Extracts the settings from the given configuration file.
    ///
    /// The config file must look like the example in `examples/image_thumbs.yaml`:
    /// ```yaml
    #[doc = include_str!("../examples/image_thumbs.yaml")]
    /// ```
    ///
    /// # Arguments
    /// * `config` - Path to the config file from the crate root (`.yaml` may be omitted)
    fn settings(config: &str) -> ThumbsResult<Vec<Params>> {
        Ok(Config::builder()
            .add_source(config::File::with_name(config))
            .build()?
            .get("thumbs")?)
    }
}

#[cfg(test)]
mod tests {
    use image::ImageFormat;
    use object_store::path::Path;
    use sequential_test::sequential;
    use tokio::fs::File;
    use tokio::io::{AsyncReadExt, BufReader};

    use crate::model::ImageDetails;
    use crate::ImageThumbs;

    #[tokio::test]
    #[ignore]
    #[sequential]
    async fn create_thumbs() {
        let client = ImageThumbs::new("src/test/image_thumbs").await.unwrap();
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

    #[tokio::test]
    #[ignore]
    #[sequential]
    async fn create_thumbs_dir() {
        let client = ImageThumbs::new("src/test/image_thumbs").await.unwrap();
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

    #[tokio::test]
    #[ignore]
    #[sequential]
    async fn create_thumbs_from_bytes() {
        let client = ImageThumbs::new("src/test/image_thumbs").await.unwrap();
        // create JPG image thumbs
        {
            let test_jpg = File::open("src/test/mock_data/testBucket/penguin.jpg")
                .await
                .unwrap();
            let mut reader = BufReader::new(test_jpg);
            let mut buffer = Vec::new();

            reader.read_to_end(&mut buffer).await.unwrap();

            client
                .create_thumbs_from_bytes(
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
                .create_thumbs_from_bytes(
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

    #[tokio::test]
    #[ignore]
    #[sequential]
    async fn override_behaviour() {
        let client = ImageThumbs::new("src/test/image_thumbs").await.unwrap();
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
        assert!(
            client
                .download_image("thumbs/penguin_standard.png")
                .await
                .is_err(),
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
