use std::{
    collections::{HashMap, hash_map::Entry},
    io::Read,
    string::FromUtf8Error,
};

use thiserror::Error;

use crate::data::{DataStream, Deserialize, DeserializeError};

#[derive(Debug, Error)]
pub enum NbtError {
    #[error("Negative array length ({0})")]
    NegativeArrayLength(i32),
    #[error("Negative list length ({0})")]
    NegativeListLength(i32),
    #[error("Invalid UTF-8: {0}")]
    InvalidUTF8(#[from] FromUtf8Error),
    #[error("TAG_End type in non-empty list")]
    EndInList,
    #[error("Two entries with same name {0:?} in compound")]
    SameName(String),
    #[error("Unknown type id {0}")]
    UnknownType(u8),
    #[error("Malformed root")]
    MalformedRoot,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Nbt {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(Vec<Nbt>),
    Compound(HashMap<String, Nbt>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Nbt {
    fn deserialize_by_id(stream: &mut DataStream, id: u8) -> Result<Self, DeserializeError> {
        let r = match id {
            0 => Self::End,
            1 => Self::Byte(i8::deserialize(stream)?),
            2 => Self::Short(i16::deserialize(stream)?),
            3 => Self::Int(i32::deserialize(stream)?),
            4 => Self::Long(i64::deserialize(stream)?),
            5 => Self::Float(f32::deserialize(stream)?),
            6 => Self::Double(f64::deserialize(stream)?),
            7 => Self::ByteArray(Self::deserialize_array(stream)?),
            8 => Self::String(Self::deserialize_string(stream)?),
            9 => Self::List(Self::deserialize_list(stream)?),
            10 => Self::Compound(Self::deserialize_compound(stream)?),
            11 => Self::IntArray(Self::deserialize_array(stream)?),
            12 => Self::LongArray(Self::deserialize_array(stream)?),
            _ => return Err(NbtError::UnknownType(id).into()),
        };
        Ok(r)
    }

    fn deserialize_array<T: Deserialize>(
        stream: &mut DataStream,
    ) -> Result<Vec<T>, DeserializeError> {
        let len = i32::deserialize(stream)?;
        if len < 0 {
            return Err(DeserializeError::Nbt(NbtError::NegativeArrayLength(len)));
        }

        let mut data = Vec::with_capacity(len as usize);

        for _ in 0..len {
            data.push(T::deserialize(stream)?);
        }

        Ok(data)
    }

    fn deserialize_string(stream: &mut DataStream) -> Result<String, DeserializeError> {
        let len = u16::deserialize(stream)?;
        let mut data = vec![0; len as usize];

        stream.read_exact(&mut data)?;

        let str = String::from_utf8(data).map_err(|e| -> NbtError { e.into() })?;
        Ok(str)
    }

    fn deserialize_list(stream: &mut DataStream) -> Result<Vec<Nbt>, DeserializeError> {
        let type_id = u8::deserialize(stream)?;
        let len = i32::deserialize(stream)?;
        if len < 0 {
            return Err(DeserializeError::Nbt(NbtError::NegativeListLength(len)));
        }

        if len == 0 {
            return Ok(Vec::new());
        }

        if type_id == 0 {
            return Err(NbtError::EndInList.into());
        }

        let mut data = Vec::with_capacity(len as usize);

        for _ in 0..len {
            data.push(Self::deserialize_by_id(stream, type_id)?);
        }

        Ok(data)
    }

    fn deserialize_compound(
        stream: &mut DataStream,
    ) -> Result<HashMap<String, Nbt>, DeserializeError> {
        let mut data = HashMap::new();

        loop {
            let id = u8::deserialize(stream)?;
            let name = Self::deserialize_string(stream)?;
            let value = Self::deserialize_by_id(stream, id)?;

            if matches!(value, Self::End) {
                return Ok(data);
            }

            match data.entry(name) {
                Entry::Vacant(e) => e.insert(value),
                Entry::Occupied(e) => return Err(NbtError::SameName(e.key().clone()).into()),
            };
        }
    }
}

impl Deserialize for Nbt {
    fn deserialize(stream: &mut DataStream) -> Result<Self, DeserializeError> {
        let id = u8::deserialize(stream)?;

        if id != 10 {
            return Err(NbtError::MalformedRoot.into());
        }

        let root = Self::deserialize_compound(stream)?;
        Ok(Self::Compound(root))
    }
}
