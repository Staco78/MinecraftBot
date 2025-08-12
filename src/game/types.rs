use std::{
    hash::Hash,
    ops::{Add, AddAssign, Mul, Neg, Sub},
};

use macros::{Deserialize, Serialize};

use crate::datatypes::{Angle, BlockPos, VarInt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Vec2<T> {
    pub x: T,
    pub z: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

pub type Vec3i = Vec3<i32>;
pub type Vec3d = Vec3<f64>;

pub type Vec2i = Vec2<i32>;

pub type ChunkPos = Vec2i;
pub type ChunkSectionPos = Vec3i;
pub type LocalPos = Vec3<u8>;

impl From<ChunkSectionPos> for ChunkPos {
    fn from(value: ChunkSectionPos) -> Self {
        Self {
            x: value.x,
            z: value.z,
        }
    }
}

fn calc_axis_chunk_pos(x: i32) -> i32 {
    if x >= 0 || x % 16 == 0 {
        x / 16
    } else {
        x / 16 - 1
    }
}

impl ChunkPos {
    #[allow(dead_code)]
    /// Convert a block pos to a chunk pos
    pub fn from_block_pos(pos: BlockPos) -> Self {
        Self {
            x: calc_axis_chunk_pos(pos.0.x),
            z: calc_axis_chunk_pos(pos.0.z),
        }
    }
}

impl ChunkSectionPos {
    /// Convert a block pos to a chunk section pos
    pub fn from_block_pos(pos: BlockPos) -> Self {
        Self {
            x: calc_axis_chunk_pos(pos.0.x),
            y: calc_axis_chunk_pos(pos.0.y),
            z: calc_axis_chunk_pos(pos.0.z),
        }
    }
}

impl LocalPos {
    pub fn from_global_block_pos(pos: BlockPos) -> Self {
        fn calc(x: i32) -> u8 {
            let r = x % 16;
            if r < 0 { (r + 16) as u8 } else { r as u8 }
        }

        Self {
            x: calc(pos.0.x),
            y: calc(pos.0.y),
            z: calc(pos.0.z),
        }
    }
}

macro_rules! impl_ops {
    ($name: ident, {$($field:ident),*}) => {
        macro_rules! impl_from {
            ($source: ty, $dest: ty) => {
                impl From<$name<$source>> for $name<$dest> {
                    fn from(source: $name<$source>) -> Self {
                        let $name::<_> { $($field),* } = source;
                        Self {
                           $($field: $field.into()),*
                        }
                    }
                }
            };
        }

        impl_from!(i32, f64);
        impl_from!(VarInt, i32);

        impl<T: Add<Output = T>> Add for $name<T> {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                Self {
                    $($field: self.$field + rhs.$field),*
                }
            }
        }

        impl<T: AddAssign> AddAssign for $name<T> {
            fn add_assign(&mut self, rhs: Self) {
                $(self.$field += rhs.$field;)*
            }
        }

        impl<T: Sub<Output = T>> Sub for $name<T> {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                Self {
                    $($field: self.$field - rhs.$field),*
                }
            }
        }

        impl<T: Neg<Output = T>> Neg for $name<T> {
            type Output = Self;
            fn neg(self) -> Self::Output {
                Self {
                    $($field: -self.$field),*
                }
            }
        }

        impl<T: Mul<Output = T> + Copy> Mul<T> for $name<T> {
            type Output = $name<T>;

            fn mul(self, rhs: T) -> Self::Output {
                Self {
                    $($field: rhs * self.$field),*
                }
            }
        }

        impl $name<f64> {
            #[allow(dead_code)]
            pub fn length(&self) -> f64 {
                let $name { $($field),* } = self;
                ($($field * $field +)* 0.).sqrt()
            }
        }
    };
}

impl_ops!(Vec2, {x, z});
impl_ops!(Vec3, {x, y, z});

impl Vec3d {
    pub fn speed_from_entity_velocity(vx: i16, vy: i16, vz: i16) -> Self {
        Self {
            x: vx as f64 / 400.,
            y: vy as f64 / 400.,
            z: vz as f64 / 400.,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Default)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
}

impl Rotation {
    pub fn from_angles(yaw: Angle, pitch: Angle) -> Self {
        Self {
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }
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
