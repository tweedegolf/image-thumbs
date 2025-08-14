use std::io::Cursor;

use image::{
    DynamicImage, GenericImageView, ImageFormat,
    codecs::{
        jpeg::JpegEncoder,
        png,
        png::{CompressionType, PngEncoder},
    },
    imageops, load_from_memory_with_format,
};
use object_store::{ObjectStore, path::Path};

use crate::{
    Error, ImageThumbs, ThumbsResult,
    model::{ImageDetails, Mode, Params},
};

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

            let thumbnail = calculate_thumbnail(&image, params, center)?;

            let mut buf = Vec::new();
            let writer = Cursor::new(&mut buf);
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

fn calculate_thumbnail(
    image: &DynamicImage,
    params: &Params,
    center: (f32, f32),
) -> ThumbsResult<DynamicImage> {
    Ok(match params.mode {
        Mode::Fit => {
            let (width, height) = limit_size_fit(params.size, image.dimensions());
            image.thumbnail(width, height)
        }
        Mode::Crop => {
            let image = crop_aspect_ratio_with_center(image, params.size, center);
            let (width, height) = limit_size_crop(params.size, image.dimensions());
            image.resize_to_fill(width, height, imageops::FilterType::Nearest)
        }
    })
}

fn limit_size_fit(target_size: (u32, u32), original_size: (u32, u32)) -> (u32, u32) {
    if target_size.0 > original_size.0 && target_size.1 > original_size.1 {
        original_size
    } else if target_size.0 > original_size.0 {
        (original_size.0, target_size.1)
    } else if target_size.1 > original_size.1 {
        (target_size.0, original_size.1)
    } else {
        target_size
    }
}

fn limit_size_crop(target_size: (u32, u32), original_size: (u32, u32)) -> (u32, u32) {
    if target_size.0 > original_size.0 || target_size.1 > original_size.1 {
        original_size
    } else {
        target_size
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

    use super::*;
    use crate::model::{Mode, Params};

    #[test]
    fn crop_center_1() {
        let image = DynamicImage::new(100, 100, ColorType::L8);

        let cropped = crop_aspect_ratio_with_center(&image, (10, 10), (0.5, 0.5));
        assert_eq!(
            cropped.width(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );
        assert_eq!(
            cropped.height(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );

        let cropped = crop_aspect_ratio_with_center(&image, (200, 200), (0.5, 0.5));
        assert_eq!(
            cropped.width(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );
        assert_eq!(
            cropped.height(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );

        let cropped = crop_aspect_ratio_with_center(&image, (10, 10), (0.9, 1.));
        assert_eq!(
            cropped.width(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );
        assert_eq!(
            cropped.height(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );

        let cropped = crop_aspect_ratio_with_center(&image, (10, 10), (0., 0.5));
        assert_eq!(
            cropped.width(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );
        assert_eq!(
            cropped.height(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );
    }

    #[test]
    fn crop_center_2() {
        let image = DynamicImage::new(100, 150, ColorType::L8);

        let cropped = crop_aspect_ratio_with_center(&image, (10, 15), (0.5, 0.5));
        assert_eq!(
            cropped.width(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );
        assert_eq!(
            cropped.height(),
            150,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );

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
        assert_eq!(
            cropped.width(),
            150,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );
        assert_eq!(
            cropped.height(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );

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

    #[test]
    fn crop_center_4() {
        let image = DynamicImage::new(150, 100, ColorType::L8);

        let cropped = crop_aspect_ratio_with_center(&image, (15, 10), (0.5, 0.5));
        assert_eq!(
            cropped.width(),
            150,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );
        assert_eq!(
            cropped.height(),
            100,
            "As the source and target aspect ratio is the same, the image should not crop be cropped"
        );

        let cropped = crop_aspect_ratio_with_center(&image, (100, 150), (0.5, 0.5));
        assert_eq!(cropped.width(), 67);
        assert_eq!(cropped.height(), 100);

        let cropped = crop_aspect_ratio_with_center(&image, (100, 150), (0., 1.));
        assert_eq!(cropped.width(), 67);
        assert_eq!(cropped.height(), 100);

        let cropped = crop_aspect_ratio_with_center(&image, (100, 150), (0.9, 0.1));
        assert_eq!(cropped.width(), 66);
        assert_eq!(cropped.height(), 100);
    }

    #[test]
    fn correct_final_size_crop_square() {
        let image = DynamicImage::new(100, 100, ColorType::L8);
        let params = Params {
            name: "".to_string(),
            naming_pattern: None,
            quality: 0,
            size: (0, 0),
            mode: Mode::Crop,
        };

        for (target_size, expect_output) in [
            ((10, 10), (10, 10)),
            ((200, 200), (100, 100)),
            ((90, 200), (45, 100)),
            ((200, 90), (100, 45)),
        ] {
            let cropped = calculate_thumbnail(
                &image,
                &Params {
                    size: target_size,
                    ..params.clone()
                },
                (0.5, 0.5),
            )
            .unwrap();
            assert_eq!(cropped.width(), expect_output.0);
            assert_eq!(cropped.height(), expect_output.1);
        }
    }

    #[test]
    fn correct_final_size_crop_non_square() {
        let portrait = DynamicImage::new(100, 150, ColorType::L8);
        let landscape = DynamicImage::new(150, 100, ColorType::L8);
        let params = Params {
            name: "".to_string(),
            naming_pattern: None,
            quality: 0,
            size: (0, 0),
            mode: Mode::Crop,
        };

        for (image, (target_size, expect_output)) in [
            (&portrait, ((10, 10), (10, 10))),
            (&portrait, ((200, 200), (100, 100))),
            (&portrait, ((90, 200), (68, 150))),
            (&portrait, ((200, 90), (100, 45))),
            (&landscape, ((10, 10), (10, 10))),
            (&landscape, ((200, 200), (100, 100))),
            (&landscape, ((90, 200), (45, 100))),
            (&landscape, ((200, 90), (150, 68))),
        ] {
            let cropped = calculate_thumbnail(
                image,
                &Params {
                    size: target_size,
                    ..params.clone()
                },
                (0.5, 0.5),
            )
            .unwrap();
            assert_eq!(cropped.width(), expect_output.0);
            assert_eq!(cropped.height(), expect_output.1);
        }
    }

    #[test]
    fn correct_final_size_fit_square() {
        let image = DynamicImage::new(100, 100, ColorType::L8);
        let params = Params {
            name: "".to_string(),
            naming_pattern: None,
            quality: 0,
            size: (0, 0),
            mode: Mode::Fit,
        };

        for (target_size, expect_output) in [
            ((10, 10), (10, 10)),
            ((200, 200), (100, 100)),
            ((90, 200), (90, 90)),
            ((200, 90), (90, 90)),
        ] {
            let cropped = calculate_thumbnail(
                &image,
                &Params {
                    size: target_size,
                    ..params.clone()
                },
                (0.5, 0.5),
            )
            .unwrap();
            assert_eq!(cropped.width(), expect_output.0);
            assert_eq!(cropped.height(), expect_output.1);
        }
    }

    #[test]
    fn correct_final_size_fit_non_square() {
        let portrait = DynamicImage::new(100, 150, ColorType::L8);
        let landscape = DynamicImage::new(150, 100, ColorType::L8);
        let params = Params {
            name: "".to_string(),
            naming_pattern: None,
            quality: 0,
            size: (0, 0),
            mode: Mode::Fit,
        };

        for (image, (target_size, expect_output)) in [
            (&portrait, ((10, 10), (7, 10))),
            (&portrait, ((200, 200), (100, 150))),
            (&portrait, ((90, 200), (90, 135))),
            (&portrait, ((200, 90), (60, 90))),
            (&landscape, ((10, 10), (10, 7))),
            (&landscape, ((200, 200), (150, 100))),
            (&landscape, ((90, 200), (90, 60))),
            (&landscape, ((200, 90), (135, 90))),
        ] {
            let cropped = calculate_thumbnail(
                image,
                &Params {
                    size: target_size,
                    ..params.clone()
                },
                (0.5, 0.5),
            )
            .unwrap();
            assert_eq!(cropped.width(), expect_output.0);
            assert_eq!(cropped.height(), expect_output.1);
        }
    }
}
