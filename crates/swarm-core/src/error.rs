use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("manifest not found at {0}")]
    ManifestMissing(String),

    #[error("{0}")]
    Msg(String),
}
