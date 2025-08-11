#![allow(dead_code)]

mod bitset;
mod varint;

pub use bitset::*;
use macros::{Deserialize, Serialize};
pub use varint::*;

use core::slice;
use std::{
    fmt::Debug, io::{Read, Write}, mem::MaybeUninit, ops::Deref
};

use crate::{
    data::{Deserialize, DeserializeError, Serialize, SerializeError},
    game::{IdSet, Slot, StructuredComponent, Vec3i},
};

impl Serialize for bool {
    fn size(&self) -> usize {
        1
    }

    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), SerializeError> {
        match *self {
            true => stream.write_all(&[1]),
            false => stream.write_all(&[0]),
        }
    }
}

impl Deserialize for bool {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let mut byte = 0;
        stream.read_exact(slice::from_mut(&mut byte))?;
        match byte {
            0 => Ok(false),
            1 => Ok(true),
            k => Err(DeserializeError::MalformedPacket(format!(
                "Invalid bool value (found {})",
                k
            ))),
        }
    }
}

macro_rules! SerializeNbr {
    ($SelfT: ty) => {
        impl Serialize for $SelfT {
            fn size(&self) -> usize {
                core::mem::size_of::<Self>()
            }

            fn serialize(
                &self,
                stream: &mut dyn Write,
            ) -> Result<(), $crate::data::SerializeError> {
                stream.write_all(&self.to_be_bytes())
            }
        }
    };
}

macro_rules! DeserializeNbr {
    ($SelfT: ty) => {
        impl Deserialize for $SelfT {
            fn deserialize(
                stream: &mut crate::data::DataStream,
            ) -> Result<Self, $crate::data::DeserializeError> {
                let mut buf = [0; core::mem::size_of::<Self>()];
                stream.read_exact(&mut buf)?;
                let val = <$SelfT>::from_be_bytes(buf);
                Ok(val)
            }
        }
    };
}

SerializeNbr!(u8);
DeserializeNbr!(u8);

SerializeNbr!(i8);
DeserializeNbr!(i8);

SerializeNbr!(u16);
DeserializeNbr!(u16);

SerializeNbr!(i16);
DeserializeNbr!(i16);

SerializeNbr!(i32);
DeserializeNbr!(i32);

SerializeNbr!(i64);
DeserializeNbr!(i64);

SerializeNbr!(u64);
DeserializeNbr!(u64);

SerializeNbr!(u128);
DeserializeNbr!(u128);

SerializeNbr!(f32);
DeserializeNbr!(f32);

SerializeNbr!(f64);
DeserializeNbr!(f64);

#[derive(Debug, Serialize, Deserialize)]
pub struct Angle(u8);

impl From<Angle> for f32 {
    /// Convert to degree
    fn from(value: Angle) -> Self {
        value.0 as f32 * (360. / 256.)
    }
}

impl Serialize for String {
    fn size(&self) -> usize {
        let n = self.len();
        assert!(n <= i32::MAX as usize);
        VarInt(n as i32).size() + self.len()
    }

    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), SerializeError> {
        let n = self.chars().count();
        assert!(n <= i32::MAX as usize);
        VarInt(n as i32).serialize(stream)?;
        stream.write_all(self.as_bytes())?;
        Ok(())
    }
}

impl Deserialize for String {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let n = VarInt::deserialize(stream)?.0;
        let n = if n >= 0 {
            n as usize
        } else {
            return Err(DeserializeError::MalformedPacket(
                "Negative String length".to_string(),
            ));
        };

        let mut buf = vec![0; n * 3];

        let mut initialized = 0;

        macro_rules! read {
            ($count: expr) => {{
                stream.read_exact(&mut buf[initialized..(initialized + $count)])?;
                initialized += $count;
            }};
        }

        for _ in 0..n {
            let i = initialized;
            read!(1);
            let width = utf8_char_width(buf[i]);
            if !(1..=3).contains(&width) {
                return Err(DeserializeError::MalformedPacket(
                    "Invalid UTF-8".to_string(),
                ));
            }
            read!(width - 1);
        }

        buf.truncate(initialized);
        let str = String::from_utf8(buf)
            .map_err(|_| DeserializeError::MalformedPacket("Invalid UTF-8".to_string()))?;
        Ok(str)
    }
}

// https://tools.ietf.org/html/rfc3629
const UTF8_CHAR_WIDTH: &[u8; 256] = &[
    // 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 1
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 2
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 3
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 4
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 5
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 6
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 7
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // A
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // B
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
];

/// Given a first byte, determines how many bytes are in this UTF-8 character.
#[inline]
const fn utf8_char_width(b: u8) -> usize {
    UTF8_CHAR_WIDTH[b as usize] as usize
}

impl<const N: usize, T: Serialize> Serialize for [T; N] {
    fn size(&self) -> usize {
        self.iter().map(Serialize::size).sum()
    }

    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), SerializeError> {
        for x in self {
            x.serialize(stream)?;
        }
        Ok(())
    }
}

impl<const N: usize, T: Deserialize> Deserialize for [T; N] {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        // FIXME: replace by array::try_stream_fn once stable

        let mut data = [const { MaybeUninit::uninit() }; N];

        for x in &mut data {
            x.write(T::deserialize(stream)?);
        }

        // FIXME: replace with MaybeUninit::array_assume_init once stable
        // Safety: each cell has been written
        let data: Self = unsafe { core::mem::transmute_copy(&data) };

        Ok(data)
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn size(&self) -> usize {
        assert!(self.len() <= i32::MAX as _);
        VarInt(self.len() as i32).size() + self.iter().map(Serialize::size).sum::<usize>()
    }

    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), SerializeError> {
        assert!(self.len() <= i32::MAX as _);
        VarInt(self.len() as i32).serialize(stream)?;
        for x in self {
            x.serialize(stream)?;
        }
        Ok(())
    }
}

impl<T: Deserialize + Debug> Deserialize for Vec<T> {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let len = VarInt::deserialize(stream)?.0 as usize;
        let data = (0..len)
            .map(|_| T::deserialize(stream))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(data)
    }
}

impl<T: Serialize> Serialize for Option<T> {
    fn size(&self) -> usize {
        match self {
            Some(v) => 1 + v.size(),
            None => 1,
        }
    }

    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), SerializeError> {
        self.is_some().serialize(stream)?;
        if let Some(val) = self {
            val.serialize(stream)?;
        }
        Ok(())
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let is_some = bool::deserialize(stream)?;
        if is_some {
            let val = T::deserialize(stream)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }
}

pub fn deserialize_slice<T: Deserialize>(
    stream: &mut crate::data::DataStream,
    length: usize,
) -> Result<Vec<T>, DeserializeError> {
    let mut data = Vec::with_capacity(length);

    for _ in 0..length {
        data.push(T::deserialize(stream)?);
    }

    Ok(data)
}

#[derive(Debug)]
pub struct LengthInferredArray<T>(pub Vec<T>);

impl<T: Deserialize> Deserialize for LengthInferredArray<T> {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let mut data = Vec::new();

        while stream.remaining_size() > 0 {
            let val = T::deserialize(stream)?;
            data.push(val);
        }

        Ok(Self(data))
    }
}

impl<T: Serialize> Serialize for LengthInferredArray<T> {
    fn size(&self) -> usize {
        self.0.iter().map(Serialize::size).sum::<usize>()
    }

    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), SerializeError> {
        for val in &self.0 {
            val.serialize(stream)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct LengthInferredByteArray(pub Vec<u8>);

impl Deserialize for LengthInferredByteArray {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let mut buf = vec![0; stream.remaining_size()];
        stream.read_exact(&mut buf)?;
        Ok(Self(buf))
    }
}

impl Serialize for LengthInferredByteArray {
    fn size(&self) -> usize {
        self.0.len()
    }

    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), SerializeError> {
        stream.write_all(&self.0)
    }
}

impl<T: Serialize> Serialize for Box<T> {
    fn size(&self) -> usize {
        self.deref().size()
    }

    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), SerializeError> {
        self.deref().serialize(stream)
    }
}

impl<T: Deserialize> Deserialize for Box<T> {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let inner = T::deserialize(stream)?;
        Ok(Box::new(inner))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BlockPos(pub Vec3i);

impl Deserialize for BlockPos {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let val = i64::deserialize(stream)?;

        Ok(Self(Vec3i {
            x: (val >> 38) as _,
            y: ((val << 52) >> 52) as _,
            z: ((val << 26) >> 38) as _,
        }))
    }
}

impl Deserialize for IdSet {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let type_ = VarInt::deserialize(stream)?.0;

        if type_ < 0 {
            return Err(DeserializeError::MalformedPacket(format!(
                "ID Set: Negative type ({})",
                type_
            )));
        }

        if type_ == 0 {
            let tag_name = String::deserialize(stream)?;
            Ok(Self::TagName(tag_name))
        } else {
            // FIXME: replace by array::try_stream_fn once stable
            let len = type_ - 1;

            let mut data = vec![MaybeUninit::uninit(); len as usize];

            for x in &mut data {
                x.write(VarInt::deserialize(stream)?);
            }

            // Safety: each cell has been written
            let data: Vec<VarInt> = unsafe { core::mem::transmute(data) };

            Ok(Self::Ids(data))
        }
    }
}

impl Deserialize for StructuredComponent {
    fn deserialize(_stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        todo!()
    }
}

impl Deserialize for Slot {
    fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
        let count = VarInt::deserialize(stream)?;
        if count.0 < 0 {
            return Err(DeserializeError::MalformedPacket(format!(
                "Slot: Negative item count ({})",
                count.0
            )));
        }
        if count.0 == 0 {
            return Ok(Self::Empty);
        }

        let id = VarInt::deserialize(stream)?;
        let components_to_add_count = VarInt::deserialize(stream)?;
        let components_to_remove_count = VarInt::deserialize(stream)?;

        if components_to_add_count.0 < 0 {
            return Err(DeserializeError::MalformedPacket(format!(
                "Slot: Negative components to add count ({})",
                components_to_add_count.0
            )));
        }
        if components_to_remove_count.0 < 0 {
            return Err(DeserializeError::MalformedPacket(format!(
                "Slot: Negative components to remove count ({})",
                components_to_remove_count.0
            )));
        }

        let components_to_add = (0..components_to_add_count.0 as usize)
            .map(|_| StructuredComponent::deserialize(stream))
            .collect::<Result<Vec<_>, _>>()?;
        let components_to_remove = (0..components_to_remove_count.0 as usize)
            .map(|_| VarInt::deserialize(stream))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self::NonEmpty {
            count,
            id,
            components_to_add,
            components_to_remove,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[enum_repr(bool)]
pub enum Or<X, Y> {
    Y(Y),
    X(X),
}
