use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("JSON serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("GLTF error: {0}")]
    Gltf(#[from] gltf::Error),

    #[error("Conversion error: {0}")]
    Conversion(String),

    #[error("File operation error on '{}': {message}", path.display())]
    File { path: PathBuf, message: String },
}

impl From<nom::Err<nom::error::Error<&[u8]>>> for Error {
    fn from(err: nom::Err<nom::error::Error<&[u8]>>) -> Self {
        match &err {
            nom::Err::Error(e) | nom::Err::Failure(e) => {
                Error::Parse(format!("nom {:?}: {}", e.code, err))
            }
            nom::Err::Incomplete(needed) => Error::Parse(format!("incomplete input: {needed:?}")),
        }
    }
}
