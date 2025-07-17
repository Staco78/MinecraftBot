mod varint;

pub use varint::*;

use core::slice;
use std::{io, mem::MaybeUninit};

use crate::data::{Deserialize, DeserializeError, Serialize, SerializeError};

impl Serialize for bool {
    fn size(&self) -> usize {
        1
    }

    fn serialize(&self, to: &mut dyn io::Write) -> Result<(), SerializeError> {
        match *self {
            true => to.write_all(&[1]),
            false => to.write_all(&[0]),
        }
    }
}

impl Deserialize for bool {
    fn deserialize(
        from: &mut dyn io::Read,
        remaining_size: &mut usize,
    ) -> Result<Self, DeserializeError> {
        let mut byte = 0;
        from.read_exact(slice::from_mut(&mut byte))?;
        *remaining_size -= 1;
        match byte {
            0 => Ok(false),
            1 => Ok(true),
            k => Err(DeserializeError::InvalidData(format!(
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
                to: &mut dyn std::io::Write,
            ) -> Result<(), $crate::data::SerializeError> {
                to.write_all(&self.to_be_bytes())
            }
        }
    };
}

macro_rules! DeserializeNbr {
    ($SelfT: ty) => {
        impl Deserialize for $SelfT {
            fn deserialize(
                from: &mut dyn std::io::Read,
                remaining_size: &mut usize,
            ) -> Result<Self, $crate::data::DeserializeError> {
                let mut buf = [0; core::mem::size_of::<Self>()];
                from.read_exact(&mut buf)?;
                let val = <$SelfT>::from_be_bytes(buf);
                *remaining_size -= core::mem::size_of::<Self>();
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

SerializeNbr!(u128);
DeserializeNbr!(u128);

SerializeNbr!(f32);
DeserializeNbr!(f32);

SerializeNbr!(f64);
DeserializeNbr!(f64);

impl Serialize for String {
    fn size(&self) -> usize {
        let n = self.len();
        assert!(n <= i32::MAX as usize);
        VarInt(n as i32).size() + self.len()
    }

    fn serialize(&self, to: &mut dyn io::Write) -> Result<(), SerializeError> {
        let n = self.chars().count();
        assert!(n <= i32::MAX as usize);
        VarInt(n as i32).serialize(to)?;
        to.write_all(self.as_bytes())?;
        Ok(())
    }
}

impl Deserialize for String {
    fn deserialize(
        from: &mut dyn io::Read,
        remaining_size: &mut usize,
    ) -> Result<Self, DeserializeError> {
        let n = VarInt::deserialize(from, remaining_size)?.0;
        let n = if n >= 0 {
            n as usize
        } else {
            return Err(DeserializeError::InvalidData(
                "Negative String length".to_string(),
            ));
        };

        let mut buf = vec![0; n * 3];

        let mut initialized = 0;

        macro_rules! read {
            ($count: expr) => {{
                from.read_exact(&mut buf[initialized..(initialized + $count)])?;
                initialized += $count;
            }};
        }

        for _ in 0..n {
            let i = initialized;
            read!(1);
            let width = utf8_char_width(buf[i]);
            if !(1..=3).contains(&width) {
                return Err(DeserializeError::InvalidData("Invalid UTF-8".to_string()));
            }
            read!(width - 1);
        }

        buf.truncate(initialized);
        *remaining_size -= initialized;
        let str = String::from_utf8(buf)
            .map_err(|_| DeserializeError::InvalidData("Invalid UTF-8".to_string()))?;
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

    fn serialize(&self, to: &mut dyn io::Write) -> Result<(), SerializeError> {
        for x in self {
            x.serialize(to)?;
        }
        Ok(())
    }
}

impl<const N: usize, T: Deserialize> Deserialize for [T; N] {
    fn deserialize(
        from: &mut dyn io::Read,
        remaining_size: &mut usize,
    ) -> Result<Self, DeserializeError> {
        // FIXME: replace by array::try_from_fn once stable

        let mut data = [const { MaybeUninit::uninit() }; N];

        for x in &mut data {
            x.write(T::deserialize(from, remaining_size)?);
        }

        // FIXME: replace with MaybeUninit::array_assume_init once stable
        // Safety: each cell has been written to
        let data: Self = unsafe { core::mem::transmute_copy(&data) };

        Ok(data)
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn size(&self) -> usize {
        assert!(self.len() <= i32::MAX as _);
        VarInt(self.len() as i32).size() + self.iter().map(Serialize::size).sum::<usize>()
    }

    fn serialize(&self, to: &mut dyn io::Write) -> Result<(), SerializeError> {
        assert!(self.len() <= i32::MAX as _);
        VarInt(self.len() as i32).serialize(to)?;
        for x in self {
            x.serialize(to)?;
        }
        Ok(())
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize(
        from: &mut dyn io::Read,
        remaining_size: &mut usize,
    ) -> Result<Self, DeserializeError> {
        let len = VarInt::deserialize(from, remaining_size)?.0 as usize;
        let data = (0..len)
            .map(|_| T::deserialize(from, remaining_size))
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

    fn serialize(&self, to: &mut dyn io::Write) -> Result<(), SerializeError> {
        self.is_some().serialize(to)?;
        if let Some(val) = self {
            val.serialize(to)?;
        }
        Ok(())
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    fn deserialize(
        from: &mut dyn io::Read,
        remaining_size: &mut usize,
    ) -> Result<Self, DeserializeError> {
        let is_some = bool::deserialize(from, remaining_size)?;
        if is_some {
            let val = T::deserialize(from, remaining_size)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
pub struct LengthInferredByteArray(pub Box<[u8]>);

impl Deserialize for LengthInferredByteArray {
    fn deserialize(
        from: &mut dyn io::Read,
        remaining_size: &mut usize,
    ) -> Result<Self, DeserializeError> {
        let mut buf = vec![0; *remaining_size];
        from.read_exact(&mut buf)?;
        *remaining_size = 0;
        Ok(Self(buf.into_boxed_slice()))
    }
}

impl Serialize for LengthInferredByteArray {
    fn size(&self) -> usize {
        self.0.len()
    }

    fn serialize(&self, to: &mut dyn io::Write) -> Result<(), SerializeError> {
        to.write_all(&self.0)
    }
}
