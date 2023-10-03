use std::ffi::OsStr;

use image::ImageFormat;
use object_store::path::Path;
use object_store::ObjectStore;
use tokio::sync::mpsc;

use crate::{Error, ImageDetails, ImageThumbs, ThumbsResult};

impl<T: ObjectStore> ImageThumbs<T> {
    pub(crate) async fn upload_thumbs(
        &self,
        mut image_channel: mpsc::Receiver<(Vec<u8>, String, ImageFormat)>,
        path: &std::path::Path,
        name: String,
    ) -> ThumbsResult<()> {
        while let Some((bytes, thumb_name, image_format)) = image_channel.recv().await {
            let path = format!(
                "{}/{}_{}.{}",
                path.to_str().unwrap_or(""),
                name,
                thumb_name,
                image_format.extensions_str()[0]
            );

            self.client.put(&Path::parse(path)?, bytes.into()).await?;
        }

        Ok(())
    }

    pub(crate) async fn download_image<'a>(
        &self,
        path: &'a std::path::Path,
    ) -> ThumbsResult<ImageDetails<'a>> {
        let result = self
            .client
            .get(&Path::parse(path.to_str().ok_or(Error::Utf)?)?)
            .await?;
        let format = ImageFormat::from_path(result.meta.location.as_ref())?;
        let stem = path
            .file_stem()
            .unwrap_or(OsStr::new("unknown"))
            .to_str()
            .unwrap_or("unknown");

        let bytes = result.bytes().await?.to_vec();

        Ok(ImageDetails {
            stem,
            format,
            path: path.parent().unwrap_or(std::path::Path::new("/")),
            bytes,
        })
    }
}
