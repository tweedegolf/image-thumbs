use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png;
use image::codecs::png::{CompressionType, PngEncoder};
use image::imageops;
use image::{load_from_memory_with_format, ImageFormat};

use object_store::ObjectStore;
use tokio::sync::mpsc;

use crate::{Error, ImageThumbs, Mode, ThumbsResult};

impl<T: ObjectStore> ImageThumbs<T> {
    pub(crate) fn create_thumbs_from_bytes(
        &self,
        bytes: Vec<u8>,
        format: ImageFormat,
        sender: mpsc::Sender<(Vec<u8>, String, ImageFormat)>,
    ) -> ThumbsResult<()> {
        let image = load_from_memory_with_format(&bytes, format)?;

        let settings = self.settings.clone();

        tokio::task::spawn(async move {
            for params in settings.iter() {
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

                sender
                    .send((buf, params.name.clone(), format))
                    .await
                    .unwrap();
            }
            Ok::<(), Error>(())
        });

        Ok(())
    }
}
