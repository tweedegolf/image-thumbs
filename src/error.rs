use image::ImageError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Storage error: {0}")]
    Storage(#[from] object_store::Error),
    #[error("Invalid path: {0}")]
    Path(#[from] object_store::path::Error),
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Image error: {0}")]
    Image(ImageError),
    #[error("Image format not supported")]
    NotSupported,
    #[error("Utf-8 error")]
    Utf,
}

impl From<ImageError> for Error {
    fn from(value: ImageError) -> Self {
        match value {
            ImageError::Unsupported(_) => Self::NotSupported,
            _ => Self::Image(value),
        }
    }
}

pub type ThumbsResult<T> = Result<T, Error>;
