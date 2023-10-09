use std::ops::Add;

use image::ImageFormat;
use object_store::path::{Path, PathPart};
use object_store::ObjectStore;
use tokio::sync::mpsc;

use crate::{ImageDetails, ImageThumbs, ThumbsResult};

impl<T: ObjectStore> ImageThumbs<T> {
    pub(crate) async fn upload_thumbs(
        &self,
        mut image_channel: mpsc::Receiver<(Vec<u8>, String, ImageFormat)>,
        path: &Path,
        name: String,
    ) -> ThumbsResult<()> {
        while let Some((bytes, thumb_name, image_format)) = image_channel.recv().await {
            let path = format!(
                "{}/{}_{}.{}",
                path,
                name,
                thumb_name,
                image_format.extensions_str()[0]
            );

            self.client.put(&Path::parse(path)?, bytes.into()).await?;
        }

        Ok(())
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

    #[cfg(test)]
    pub(crate) async fn delete(&self, path: &str) -> ThumbsResult<()> {
        self.client.delete(&Path::parse(path)?).await?;
        Ok(())
    }
}
