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
