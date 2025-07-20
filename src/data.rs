mod stream;
use std::io;

pub use stream::{DataStream, ReadWrite};

use thiserror::Error;

pub type SerializeError = io::Error;

pub trait Serialize {
    fn size(&self) -> usize;
    fn serialize(&self, stream: &mut DataStream) -> Result<(), SerializeError>;
}

#[derive(Debug, Error)]
pub enum DeserializeError {
    #[error("IO error")]
    Io(#[from] io::Error),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

pub trait Deserialize: Sized {
    fn deserialize(stream: &mut DataStream) -> Result<Self, DeserializeError>;
}
