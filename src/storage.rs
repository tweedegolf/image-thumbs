use image::{guess_format, ImageFormat};
use object_store::path::{Path, PathPart};
use object_store::{ClientOptions, ObjectMeta, ObjectStore};
use std::ops::Deref;

use crate::model::ImageDetails;
use crate::Error::NotSupported;
use crate::{ImageThumbs, ThumbsResult};

impl<T: ObjectStore> ImageThumbs<T> {
    /// Returns options for an [`object_store`] client that maps the file extensions `.jpeg`,
    /// `.jpg`, and `.png` to its MIME types.
    ///
    /// Allows `http` connections in case of tests
    pub(crate) fn client_options() -> ClientOptions {
        #[allow(unused_mut)]
        let mut client_options = ClientOptions::new()
            .with_content_type_for_suffix("jpg", mime::IMAGE_JPEG.to_string())
            .with_content_type_for_suffix("jpeg", mime::IMAGE_JPEG.to_string())
            .with_content_type_for_suffix("png", mime::IMAGE_PNG.to_string());

        #[cfg(debug_assertions)]
        {
            client_options = client_options.with_allow_http(true);
        }
        client_options
    }

    pub(crate) async fn upload_thumbs(&self, images: Vec<ImageDetails>) -> ThumbsResult<()> {
        for image in images {
            let path = Self::generate_path(&image.path, &image.stem, &image.format);
            self.client
                .read()
                .await
                .put(&Path::parse(path)?, image.bytes.into())
                .await?;
        }

        Ok(())
    }

    pub(crate) fn generate_path(
        base: &Path,
        mut image_stem: &str,
        image_format: &ImageFormat,
    ) -> String {
        image_stem = image_stem.strip_prefix('/').unwrap_or(image_stem);
        format!(
            "{}/{}.{}",
            base,
            image_stem,
            image_format.extensions_str()[0]
        )
    }

    pub(crate) fn generate_thumb_stem(
        image_stem: &str,
        thumb_name: &str,
        naming_pattern: &str,
    ) -> String {
        naming_pattern
            .replace("{thumb_name}", thumb_name)
            .replace("{image_stem}", image_stem)
    }

    pub(crate) async fn download_image(&self, path: &str) -> ThumbsResult<ImageDetails> {
        let result = self.client.read().await.get(&Path::parse(path)?).await?;
        let stem = Self::extract_stem(&result.meta.location)?.to_string();

        let path = result.meta.location.parts().collect::<Vec<PathPart>>();
        let path = if path.len() > 1 {
            path[..path.len() - 2]
                .iter()
                .fold(Path::default(), |path, part| path.child(part.clone()))
        } else {
            Path::from("/")
        };

        let bytes = result.bytes().await?.to_vec();
        let format = guess_format(&bytes)?;

        Ok(ImageDetails {
            stem,
            format,
            path,
            bytes,
        })
    }

    fn extract_stem(path: &Path) -> ThumbsResult<&str> {
        let (stem, _) = match path.filename() {
            None => Err(NotSupported)?,
            Some(filename) => filename.rsplit_once('.').unwrap_or((filename, "")),
        };
        Ok(stem)
    }

    pub(crate) async fn list_folder(&self, prefix: Option<&Path>) -> ThumbsResult<Vec<Path>> {
        Ok(self
            .client
            .read()
            .await
            .list_with_delimiter(prefix)
            .await?
            .objects
            .into_iter()
            .map(|meta| meta.location)
            .collect::<Vec<Path>>())
    }

    pub(crate) async fn head(&self, path: &Path) -> ThumbsResult<ObjectMeta> {
        Ok(self.client.read().await.head(path).await?)
    }

    pub(crate) fn filter_existent_thumbs(
        &self,
        images: Vec<Path>,
        thumbs: &[Path],
    ) -> ThumbsResult<Vec<Path>> {
        let mut res = Vec::new();
        for image in images {
            let format = ImageFormat::from_extension(image.extension().ok_or(NotSupported)?)
                .ok_or(NotSupported)?;
            let file_stem = image.filename().ok_or(NotSupported)?;
            let mut has_all_thumbs = true;
            for params in self.settings.deref() {
                let target_name = format!(
                    "{}_{}.{}",
                    file_stem,
                    params.name,
                    format.extensions_str()[0]
                );
                let thumb_names: Vec<_> = thumbs.iter().map(|p| p.filename()).collect();
                if !thumb_names.contains(&Some(&target_name)) {
                    has_all_thumbs = false;
                    break;
                }
            }
            if !has_all_thumbs {
                res.push(image);
            }
        }
        Ok(res)
    }

    #[cfg(test)]
    pub(crate) async fn delete(&self, path: &str) -> ThumbsResult<()> {
        self.client.read().await.delete(&Path::parse(path)?).await?;
        Ok(())
    }
}
