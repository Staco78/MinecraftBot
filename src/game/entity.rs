use macros::{Deserialize, Serialize};

use crate::{
    datatypes::VarInt,
    game::{Rotation, Vec3d},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct EntityId(i32);

impl From<i32> for EntityId {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl From<VarInt> for EntityId {
    fn from(value: VarInt) -> Self {
        Self(value.0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Entity {
    pub id: EntityId,
    pub position: Vec3d,
    pub rotation: Rotation,
    pub speed: Vec3d,
}
