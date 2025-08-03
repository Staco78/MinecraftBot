use crate::{
    data::{Deserialize, Serialize},
    datatypes::VarInt,
};

pub trait EnumRepr: Copy + Serialize + Deserialize {
    type Inner;

    fn from_value(v: Self::Inner) -> Self;
    fn to_value(self) -> Self::Inner;
}

macro_rules! impl_transparent {
    ($SelfT: ty) => {
        impl EnumRepr for $SelfT {
            type Inner = $SelfT;

            #[inline(always)]
            fn from_value(v: Self::Inner) -> Self {
                v
            }

            #[inline(always)]
            fn to_value(self) -> Self::Inner {
                self
            }
        }
    };
}

impl_transparent!(u8);
impl_transparent!(i8);

impl_transparent!(u16);
impl_transparent!(i16);

impl_transparent!(i32);

impl_transparent!(i64);

impl_transparent!(u128);

impl EnumRepr for VarInt {
    type Inner = i32;

    #[inline(always)]
    fn from_value(v: Self::Inner) -> Self {
        Self(v)
    }

    #[inline(always)]
    fn to_value(self) -> Self::Inner {
        self.0
    }
}

impl EnumRepr for bool {
    type Inner = u8;

    #[inline(always)]
    fn from_value(v: Self::Inner) -> Self {
        match v {
            0 => false,
            1 => true,
            o => panic!("{o} cannot be transformed to bool"),
        }
    }

    #[inline(always)]
    fn to_value(self) -> Self::Inner {
        self as Self::Inner
    }
}

#[macro_export]
macro_rules! bitflags {
    (
        $(#[$outer:meta])*
        $vis:vis struct $BitFlags:ident: $T:ty {
            $(
                $(#[$inner:ident $($args:tt)*])*
                const $Flag:tt = $value:expr;
            )*
        }

        $($t:tt)*
    ) => {
        bitflags::bitflags! {
            $(#[$outer])*
            $vis struct $BitFlags: $T {
                $(
                    $(#[$inner $($args)*])*
                    const $Flag = $value;
                )*
            }
        }

        impl Serialize for $BitFlags {
            fn size(&self) -> usize {
                size_of::<$T>()
            }

            fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), $crate::data::SerializeError> {
                self.bits().serialize(stream)
            }
        }

        impl Deserialize for $BitFlags {
            fn deserialize(stream: &mut $crate::data::DataStream) -> Result<Self, $crate::data::DeserializeError> {
                let inner = <$T>::deserialize(stream)?;
                Ok(Self::from_bits_retain(inner))
            }
        }
    };
}
