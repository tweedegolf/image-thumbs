use image::ImageFormat;
use object_store::path::Path;
use serde::Deserialize;

#[derive(Debug)]
pub struct ImageThumbs<T> {
    pub(crate) client: T,
    pub(crate) settings: Vec<Params>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Params {
    /// Can be used as `{thumb_name}` in the `naming_pattern`.
    /// If the naming_pattern is not explicitly given, the default is
    /// "`/{thumb_name}/{image_name}.{image_extension}`"
    pub(crate) name: String,
    pub(crate) naming_pattern: Option<String>,
    /// PNG ignores this variable as it is always lossless
    pub(crate) quality: u8,
    pub(crate) size: (u32, u32),
    pub(crate) mode: Mode,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Mode {
    /// The image's aspect ratio is preserved. The image is scaled to the maximum possible size that
    /// fits within the bounds.
    Fit,
    /// The image's aspect ratio is preserved. The image is scaled to the maximum possible size that
    /// fits within the larger (relative to aspect ratio) of the bounds, then cropped to fit within
    /// the other bound.
    Crop,
}

#[derive(Debug)]
pub(crate) struct ImageDetails {
    /// image filename without path and extension
    pub(crate) stem: String,
    pub(crate) format: ImageFormat,
    pub(crate) path: Path,
    pub(crate) bytes: Vec<u8>,
}
