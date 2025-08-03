mod stream;
use std::io::{self, Write};

pub use stream::{DataStream, ReadWrite};

use thiserror::Error;

use crate::nbt::NbtError;

pub type SerializeError = io::Error;

pub trait Serialize {
    fn size(&self) -> usize;
    fn serialize(&self, stream: &mut dyn Write) -> Result<(), SerializeError>;
}

#[derive(Debug, Error)]
pub enum DeserializeError {
    #[error("IO error")]
    Io(#[from] io::Error),
    #[error("Malformed packet: {0}")]
    MalformedPacket(String),
    #[error("NBT error: {0}")]
    Nbt(#[from] NbtError),
}

pub trait Deserialize: Sized {
    fn deserialize(stream: &mut DataStream) -> Result<Self, DeserializeError>;
}

impl<U: Deserialize, V: Deserialize> Deserialize for (U, V) {
    fn deserialize(stream: &mut DataStream) -> Result<Self, DeserializeError> {
        let u = U::deserialize(stream)?;
        let v = V::deserialize(stream)?;
        Ok((u, v))
    }
}

impl<U: Serialize, V: Serialize> Serialize for (U, V) {
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }

    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), SerializeError> {
        self.0.serialize(stream)?;
        self.1.serialize(stream)?;
        Ok(())
    }
}
