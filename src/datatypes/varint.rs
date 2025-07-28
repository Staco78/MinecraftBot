use std::io::{Read, Write};
use std::slice;

use crate::data::{Deserialize, DeserializeError, Serialize};

macro_rules! Var {
    ($SelfT: ident, $inner: ty, $max_len: literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $SelfT(pub $inner);

        impl $SelfT {
            fn read_(
                from: &mut dyn std::io::Read,
                max_size: Option<usize>,
            ) -> Result<$inner, DeserializeError> {
                let mut val = 0 as $inner;
                let mut i = 0;

                loop {
                    if i == $max_len {
                        return Err(DeserializeError::MalformedPacket(
                            concat!(stringify!($SelfT), " too long").to_string(),
                        ));
                    }
                    if Some(i) == max_size {
                        return Err(DeserializeError::MalformedPacket(
                            concat!("No bytes remaining to read ", stringify!($SelfT)).to_string(),
                        ));
                    }

                    let mut byte = 0;
                    from.read_exact(slice::from_mut(&mut byte))?;

                    val |= (byte as $inner & 0x7F) << (7 * i);

                    i += 1;

                    if byte & 0x80 == 0 {
                        break;
                    }
                }

                Ok(val)
            }

            pub fn read(from: &mut dyn std::io::Read) -> Result<$inner, DeserializeError> {
                Self::read_(from, None)
            }
        }

        impl Serialize for $SelfT {
            fn size(&self) -> usize {
                if self.0 == 0 {
                    1
                } else {
                    let bits = <$inner>::BITS.saturating_sub(self.0.leading_zeros());
                    bits.div_ceil(7) as usize
                }
            }

            fn serialize(
                &self,
                stream: &mut crate::data::DataStream,
            ) -> Result<(), crate::data::SerializeError> {
                if self.0 == 0 {
                    stream.write_all(&[0])?;
                    return Ok(());
                }
                let mut val = self.0;
                while val != 0 {
                    let mut data = (val & 0x7F) as u8;
                    val >>= 7;
                    if val != 0 {
                        data |= 0x80;
                    }
                    stream.write_all(slice::from_ref(&data))?;
                }

                Ok(())
            }
        }

        impl Deserialize for $SelfT {
            fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, DeserializeError> {
                let val = Self(Self::read(stream as &mut dyn Read)?);
                Ok(val)
            }
        }
    };
}

Var!(VarInt, i32, 5);
Var!(VarLong, i64, 10);
