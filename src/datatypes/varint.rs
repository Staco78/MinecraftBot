use std::slice;

use crate::data::{Deserialize, DeserializeError, Serialize};

macro_rules! Var {
    ($SelfT: ident, $inner: ty, $max_len: literal) => {
        #[derive(Debug)]
        pub struct $SelfT(pub $inner);

        impl $SelfT {
            pub fn read(from: &mut dyn std::io::Read) -> Result<$inner, DeserializeError> {
                let mut val = 0 as $inner;
                let mut i = 0;

                loop {
                    let mut byte = 0;
                    from.read_exact(slice::from_mut(&mut byte))?;

                    val |= (byte as $inner & 0x7F) << (7 * i);

                    i += 1;

                    if byte & 0x80 == 0 {
                        break;
                    }
                    if i == $max_len {
                        return Err(DeserializeError::InvalidData(format!(concat!(
                            stringify!($SelfT),
                            " too long"
                        ))));
                    }
                }

                Ok(val)
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
                to: &mut dyn std::io::Write,
            ) -> Result<(), crate::data::SerializeError> {
                if self.0 == 0 {
                    to.write_all(&[0])?;
                    return Ok(());
                }
                let mut val = self.0;
                while val != 0 {
                    let mut data = (val & 0x7F) as u8;
                    val >>= 7;
                    if val != 0 {
                        data |= 0x80;
                    }
                    to.write_all(slice::from_ref(&data))?;
                }

                Ok(())
            }
        }

        impl Deserialize for $SelfT {
            fn deserialize(
                from: &mut dyn std::io::Read,
                remaining_size: &mut usize,
            ) -> Result<Self, DeserializeError> {
                let val = Self(Self::read(from)?);
                *remaining_size -= val.size();
                Ok(val)
            }
        }
    };
}

Var!(VarInt, i32, 5);
Var!(VarLong, i64, 10);
