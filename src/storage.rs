use image::ImageFormat;
use object_store::path::{Path, PathPart};
use object_store::{ObjectMeta, ObjectStore};

use crate::Error::NotSupported;
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

        Ok(ImageDetails {
            stem,
            format,
            path,
            bytes,
        })
    }

    pub(crate) fn extract_stem(path: &Path) -> ThumbsResult<&str> {
        let (stem, _) = path
            .filename()
            .ok_or(NotSupported)?
            .rsplit_once('.')
            .ok_or(NotSupported)?;
        Ok(stem)
    }

    pub(crate) async fn list_folder(&self, prefix: Option<&Path>) -> ThumbsResult<Vec<Path>> {
        Ok(self
            .client
            .list_with_delimiter(prefix)
            .await?
            .objects
            .into_iter()
            .map(|meta| meta.location)
            .collect::<Vec<Path>>())
    }

    pub(crate) async fn head(&self, path: &Path) -> ThumbsResult<ObjectMeta> {
        Ok(self.client.head(path).await?)
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
            for params in self.settings.clone() {
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
        self.client.delete(&Path::parse(path)?).await?;
        Ok(())
    }
}
