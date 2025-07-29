use std::ops::{Add, Neg, Sub};

use macros::{Deserialize, Serialize};

use crate::datatypes::VarInt;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

pub type Vec3i = Vec3<i32>;
pub type Vec3d = Vec3<f64>;

impl Vec3d {
    pub fn length(&self) -> f64 {
        let Vec3d { x, y, z } = self;
        (x * x + y * y + z * z).sqrt()
    }

    pub fn middle_of(block: Vec3i) -> Self {
        Self::from(block)
            + Vec3d {
                x: 0.5,
                y: 0.,
                z: 0.5,
            }
    }
}

macro_rules! impl_from {
    ($source: ty, $dest: ty) => {
        impl From<Vec3<$source>> for Vec3<$dest> {
            fn from(source: Vec3<$source>) -> Self {
                let Vec3::<_> { x, y, z } = source;
                Self {
                    x: x.into(),
                    y: y.into(),
                    z: z.into(),
                }
            }
        }
    };
}

impl_from!(i32, f64);
impl_from!(VarInt, i32);

impl<T: Add<Output = T>> Add for Vec3<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Vec3<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl<T: Neg<Output = T>> Neg for Vec3<T> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Default)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum IdSet {
    TagName(String),
    Ids(Vec<VarInt>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[repr(u32)]
#[enum_repr(VarInt)]
pub enum SlotDisplay {
    Empty = 0,
    AnyFuel,
    Item {
        item_type: VarInt,
    },
    ItemStack(Slot),
    Tag(String),
    SmithingTrim {
        base: Box<SlotDisplay>,
        material: Box<SlotDisplay>,
        patter: Box<SlotDisplay>,
    },
    WithRemainder {
        ingredient: Box<SlotDisplay>,
        remainder: Box<SlotDisplay>,
    },
    Composite(Vec<SlotDisplay>),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Slot {
    Empty,
    NonEmpty {
        count: VarInt,
        id: VarInt,
        components_to_add: Vec<StructuredComponent>,
        components_to_remove: Vec<VarInt>,
    },
}

#[derive(Debug)]
pub enum StructuredComponent {
    // TODO
}
