use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png;
use image::codecs::png::{CompressionType, PngEncoder};
use image::{imageops, DynamicImage};
use image::{load_from_memory_with_format, ImageFormat};
use object_store::path::Path;
use object_store::ObjectStore;

use crate::model::{ImageDetails, Mode};
use crate::{Error, ImageThumbs, ThumbsResult};

impl<T: ObjectStore> ImageThumbs<T> {
    pub(crate) async fn create_thumb_images_from_bytes(
        &self,
        bytes: Vec<u8>,
        dest_dir: Path,
        stem: &str,
        format: ImageFormat,
        force_override: bool,
        center: (f32, f32),
    ) -> ThumbsResult<Vec<ImageDetails>> {
        
        let image = load_from_memory_with_format(&bytes, format)?;

        let mut res = Vec::with_capacity(self.settings.len());
        for params in self.settings.iter() {
            let naming_pattern = params
                .naming_pattern
                .clone()
                .unwrap_or("/{image_stem}_{thumb_name}".to_string());
            let thumb_stem = Self::generate_thumb_stem(stem, &params.name, &naming_pattern);
            if !force_override
                && self
                    .head(&Path::parse(Self::generate_path(
                        &dest_dir,
                        &thumb_stem,
                        &format,
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
                Mode::Crop => {
                    let image = crop_aspect_ratio_with_center(&image, params.size, center);
                    image.resize_to_fill(
                        params.size.0,
                        params.size.1,
                        imageops::FilterType::Nearest,
                    )
                },
            };

            match format {
                ImageFormat::Jpeg => {
                    let encoder = JpegEncoder::new_with_quality(writer, params.quality);
                    thumbnail.write_with_encoder(encoder)?;
                }
                ImageFormat::Png => {
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
                stem: thumb_stem,
                format,
                path: dest_dir.clone(),
                bytes: buf,
            })
        }
        Ok(res)
    }
}

fn crop_aspect_ratio_with_center(
    image: &DynamicImage,
    target_size: (u32, u32),
    center: (f32, f32),
) -> DynamicImage {
    let orig_aspect_ratio = image.width() as f32 / image.height() as f32;
    let target_aspect_ratio = target_size.0 as f32 / target_size.1 as f32;

    let (crop_width, crop_height) = if orig_aspect_ratio > target_aspect_ratio {
        (
            target_aspect_ratio * image.height() as f32,
            image.height() as f32,
        )
    } else if orig_aspect_ratio < target_aspect_ratio {
        (
            image.width() as f32,
            image.width() as f32 / target_aspect_ratio,
        )
    } else {
        (image.width() as f32, image.height() as f32)
    };

    let x = (image.width() as f32 * center.0 - crop_width * 0.5).round();
    let x = if x < 0. {
        0_u32
    } else if x > (image.width() as f32 - crop_width) {
        image.width() - crop_width as u32
    } else {
        x as u32
    };

    let y = (image.height() as f32 * center.1 - crop_height * 0.5).round();
    let y = if y < 0. {
        0_u32
    } else if y > (image.height() as f32 - crop_height) {
        image.height() - crop_height as u32
    } else {
        y as u32
    };

    image.crop_imm(x, y, crop_width.round() as u32, crop_height.round() as u32)
}

#[cfg(test)]
mod test {
    use image::{ColorType, DynamicImage};

    use crate::image::crop_aspect_ratio_with_center;

    #[test]
    fn crop_center_1() {
        let image = DynamicImage::new(100, 100, ColorType::L8);

        let cropped = crop_aspect_ratio_with_center(&image, (10, 10), (0.5, 0.5));
        assert_eq!(cropped.width(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");
        assert_eq!(cropped.height(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");

        let cropped = crop_aspect_ratio_with_center(&image, (200, 200), (0.5, 0.5));
        assert_eq!(cropped.width(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");
        assert_eq!(cropped.height(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");

        let cropped = crop_aspect_ratio_with_center(&image, (10, 10), (0.9, 1.));
        assert_eq!(cropped.width(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");
        assert_eq!(cropped.height(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");

        let cropped = crop_aspect_ratio_with_center(&image, (10, 10), (0., 0.5));
        assert_eq!(cropped.width(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");
        assert_eq!(cropped.height(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");
    }

    #[test]
    fn crop_center_2() {
        let image = DynamicImage::new(100, 150, ColorType::L8);

        let cropped = crop_aspect_ratio_with_center(&image, (10, 15), (0.5, 0.5));
        assert_eq!(cropped.width(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");
        assert_eq!(cropped.height(), 150, "As the source and target aspect ratio is the same, the image should not crop be cropped");

        let cropped = crop_aspect_ratio_with_center(&image, (15, 10), (0.5, 0.5));
        assert_eq!(cropped.width(), 100);
        assert_eq!(cropped.height(), 67);

        let cropped = crop_aspect_ratio_with_center(&image, (15, 10), (0., 1.));
        assert_eq!(cropped.width(), 100);
        assert_eq!(cropped.height(), 66);

        let cropped = crop_aspect_ratio_with_center(&image, (15, 10), (0.9, 0.1));
        assert_eq!(cropped.width(), 100);
        assert_eq!(cropped.height(), 67);
    }

    #[test]
    fn crop_center_3() {
        let image = DynamicImage::new(150, 100, ColorType::L8);

        let cropped = crop_aspect_ratio_with_center(&image, (15, 10), (0.5, 0.5));
        assert_eq!(cropped.width(), 150, "As the source and target aspect ratio is the same, the image should not crop be cropped");
        assert_eq!(cropped.height(), 100, "As the source and target aspect ratio is the same, the image should not crop be cropped");

        let cropped = crop_aspect_ratio_with_center(&image, (10, 15), (0.5, 0.5));
        assert_eq!(cropped.width(), 67);
        assert_eq!(cropped.height(), 100);

        let cropped = crop_aspect_ratio_with_center(&image, (10, 15), (0., 1.));
        assert_eq!(cropped.width(), 67);
        assert_eq!(cropped.height(), 100);

        let cropped = crop_aspect_ratio_with_center(&image, (10, 15), (0.9, 0.1));
        assert_eq!(cropped.width(), 66);
        assert_eq!(cropped.height(), 100);
    }
}
