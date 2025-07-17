use std::io::{self, Read, Write};

use thiserror::Error;

pub type SerializeError = io::Error;

pub trait Serialize {
    fn size(&self) -> usize;
    fn serialize(&self, to: &mut dyn Write) -> Result<(), SerializeError>;
}

#[derive(Debug, Error)]
pub enum DeserializeError {
    #[error("IO error")]
    Io(#[from] io::Error),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

pub trait Deserialize: Sized {
    fn deserialize(from: &mut dyn Read, remaining_size: &mut usize) -> Result<Self, DeserializeError>;
}
