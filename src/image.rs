use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png;
use image::codecs::png::{CompressionType, PngEncoder};
use image::imageops;
use image::{load_from_memory_with_format, ImageFormat};
use object_store::path::Path;
use object_store::ObjectStore;

use crate::model::{ImageDetails, Mode};
use crate::{Error, ImageThumbs, ThumbsResult};

impl<T: ObjectStore> ImageThumbs<T> {
    pub(crate) async fn create_thumbs_from_bytes(
        &self,
        bytes: Vec<u8>,
        dest_dir: Path,
        stem: &str,
        format: ImageFormat,
        force_override: bool,
    ) -> ThumbsResult<Vec<ImageDetails>> {
        let image = load_from_memory_with_format(&bytes, format)?;

        let mut res = Vec::with_capacity(self.settings.len());
        for params in self.settings.iter() {
            let new_name = format!("{stem}_{}", params.name);
            if !force_override
                && self
                    .head(&Path::parse(Self::generate_path(
                        &dest_dir, &new_name, &format,
                    ))?)
                    .await
                    .is_ok()
            {
                continue; // do not compute already existent thumbnails
            }

            let mut buf = Vec::new();
            let writer = Cursor::new(&mut buf);

            let thumbnail = match params.mode {
                Mode::Fit => image.thumbnail(params.size.0, params.size.1),
                Mode::Crop => image.resize_to_fill(
                    params.size.0,
                    params.size.1,
                    imageops::FilterType::Nearest,
                ),
            };

            match format {
                ImageFormat::Png => {
                    let encoder = JpegEncoder::new_with_quality(writer, params.quality);
                    thumbnail.write_with_encoder(encoder)?;
                }
                ImageFormat::Jpeg => {
                    let encoder = PngEncoder::new_with_quality(
                        writer,
                        CompressionType::default(),
                        png::FilterType::default(),
                    );
                    thumbnail.write_with_encoder(encoder)?;
                }
                _ => Err(Error::NotSupported)?,
            };
            res.push(ImageDetails {
                stem: new_name,
                format,
                path: dest_dir.clone(),
                bytes: buf,
            })
        }
        Ok(res)
    }
}
