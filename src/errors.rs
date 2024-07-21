use bevy::asset::LoadDirectError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StartupError {
    #[error("No path specified for terrain")]
    MissingGltfPath,
}

#[derive(Error, Debug)]
pub enum LoaderError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    RonSpannedError(#[from] ron::error::SpannedError),
    #[error(transparent)]
    PostcardError(#[from] postcard::Error),
    #[error(transparent)]
    LoadDirectError(#[from] LoadDirectError),
    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum SaverError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    RonSpannedError(#[from] ron::error::SpannedError),
    #[error(transparent)]
    PostcardError(#[from] postcard::Error),
}
