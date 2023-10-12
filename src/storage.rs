use std::ops::Add;

use futures::TryStreamExt;
use image::ImageFormat;
use object_store::path::{Path, PathPart};
use object_store::ObjectStore;

use crate::{ImageDetails, ImageThumbs, ThumbsResult};

impl<T: ObjectStore> ImageThumbs<T> {
    pub(crate) async fn upload_thumbs(&self, images: Vec<ImageDetails>) -> ThumbsResult<()> {
        for image in images {
            let path = Self::generate_path(&image.path, &image.stem, &image.format);
            self.client
                .put(&Path::parse(path)?, image.bytes.into())
                .await?;
        }

        Ok(())
    }

    pub(crate) fn generate_path(base: &Path, name: &str, format: &ImageFormat) -> String {
        format!("{}/{}.{}", base, name, format.extensions_str()[0])
    }

    pub(crate) async fn download_image(&self, path: &str) -> ThumbsResult<ImageDetails> {
        let result = self.client.get(&Path::parse(path)?).await?;
        let format = ImageFormat::from_path(result.meta.location.as_ref())?;
        let stem = result
            .meta
            .location
            .filename()
            .unwrap_or("unknown")
            .split('.')
            .collect::<Vec<&str>>();
        let stem = stem[..stem.len() - 1]
            .iter()
            .fold(String::new(), |acc, &add| acc.add(add));

        let path = result.meta.location.parts().collect::<Vec<PathPart>>();
        let path = if path.len() > 1 {
            path[..path.len() - 2]
                .iter()
                .fold(Path::default(), |path, part| path.child(part.clone()))
        } else {
            Path::from("/")
        };

        let bytes = result.bytes().await?.to_vec();

        Ok(ImageDetails {
            stem,
            format,
            path,
            bytes,
        })
    }

    pub(crate) async fn list_folder(&self, prefix: Option<&Path>) -> ThumbsResult<Vec<Path>> {
        Ok(self
            .client
            .list(prefix)
            .await?
            .map_ok(|meta| meta.location)
            .try_collect::<Vec<Path>>()
            .await?)
    }

    #[cfg(test)]
    pub(crate) async fn delete(&self, path: &str) -> ThumbsResult<()> {
        self.client.delete(&Path::parse(path)?).await?;
        Ok(())
    }
}
