use image::ImageFormat;
use object_store::path::Path;
use serde::Deserialize;

#[derive(Debug)]
pub struct ImageThumbs<T> {
    pub(crate) client: T,
    pub(crate) settings: Vec<Params>,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Params {
    pub(crate) name: String,
    pub(crate) quality: u8,
    pub(crate) size: (u32, u32),
    pub(crate) mode: Mode,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Mode {
    Fit,
    Crop,
}

#[derive(Debug)]
pub(crate) struct ImageDetails {
    pub(crate) stem: String,
    pub(crate) format: ImageFormat,
    pub(crate) path: Path,
    pub(crate) bytes: Vec<u8>,
}
