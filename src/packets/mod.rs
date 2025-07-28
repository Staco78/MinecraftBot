#![allow(dead_code)]

mod receive;

pub use receive::*;

use macros::{Deserialize, Serialize};

use crate::{
    data::{DataStream, Deserialize, DeserializeError, ReadWrite, Serialize, SerializeError},
    datatypes::{Angle, Color, IdSet, LengthInferredByteArray, Or, Position, SlotDisplay, VarInt},
    nbt::Nbt,
};

pub trait ServerboundPacket: Serialize {
    const ID: u32;
}

pub trait ClientboundPacket: Deserialize {
    const ID: u32;
}

pub fn send_packet<T: ServerboundPacket>(
    stream: &mut dyn ReadWrite,
    packet: T,
) -> Result<(), SerializeError> {
    let mut stream = DataStream::new(stream, 0);
    let id = VarInt(T::ID as i32);
    let size = packet.size() + id.size();
    assert!(size <= i32::MAX as _);
    VarInt(size as i32).serialize(&mut stream)?;
    id.serialize(&mut stream)?;
    packet.serialize(&mut stream)
}

pub fn receive_packet<T: ClientboundPacket>(
    stream: &mut dyn ReadWrite,
) -> Result<T, DeserializeError> {
    let size = VarInt::read(stream)? as usize;
    let mut stream = DataStream::new(stream, size);
    let read_id = VarInt::deserialize(&mut stream)?;
    assert_eq!(read_id.0, T::ID as i32);
    T::deserialize(&mut stream)
}

#[derive(Debug, Serialize)]
#[sb_id = 0]
pub struct Handshake {
    pub protocol_version: VarInt,
    pub server_addr: String,
    pub server_port: u16,
    pub intent: VarInt,
}

/// State Status

#[derive(Debug, Serialize)]
#[sb_id = 0]
pub struct StatusRequest {}

#[derive(Debug, Deserialize)]
#[cb_id = 0]
pub struct StatusResponse {
    pub response: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[sb_id = 1]
#[cb_id = 1]
pub struct PingPong {
    pub timestamp: i64,
}

/// State Login

#[derive(Debug, Serialize)]
#[sb_id = 0]
pub struct LoginStart {
    // name length should be <= 16
    pub username: String,
    pub uuid: u128,
}

#[derive(Debug, Deserialize)]
#[cb_id = 2]
pub struct LoginSuccess {
    pub uuid: u128,
    pub username: String,
    pub property: Vec<LoginSuccessProperty>,
}

#[derive(Debug, Deserialize)]
pub struct LoginSuccessProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Debug, Serialize)]
#[sb_id = 3]
pub struct LoginAcknowledged {}

/// State Configuration

#[derive(Debug, Deserialize, Serialize)]
#[cb_id = 1]
#[sb_id = 2]
pub struct PluginMessage {
    pub channel: String,
    pub data: LengthInferredByteArray,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x0C]
pub struct FeatureFlags {
    pub feature_flags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cb_id = 0x0E]
#[sb_id = 7]
pub struct KnownPacks {
    pub known_packs: Vec<KnownPack>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[sb_id = 3]
#[cb_id = 3]
pub struct FinishConfiguration {}

#[derive(Debug, Deserialize)]
#[cb_id = 7]
pub struct RegistryData {
    pub registry_id: String,
    pub entries: Vec<RegistryDataEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryDataEntry {
    pub entry_id: String,
    pub data: Option<Nbt>,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0xD]
pub struct UpdateTags {
    pub tags_array: Vec<(String, Tags)>,
}

#[derive(Debug, Deserialize)]
pub struct Tags {
    pub tags: Vec<(String, Vec<VarInt>)>,
}

/// State Play

#[derive(Debug, Deserialize)]
#[cb_id = 0x2B]
pub struct Login {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub dimension_names: Vec<String>,
    pub max_players: VarInt,
    pub view_distance: VarInt,
    pub simulation_distance: VarInt,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub limited_crafting: bool,
    pub dimension_type: VarInt,
    pub dimesion_name: String,
    pub hashed_seed: i64,
    pub game_mode: u8,
    pub previous_game_mode: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub death_location: Option<DeathLocation>,
    pub portal_cooldown: VarInt,
    pub sea_level: VarInt,
    pub enforce_secure_chat: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeathLocation {
    pub dimension_name: String,
    pub location: Position,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0xA]
pub struct ChangeDifficulty {
    pub difficulty: u8,
    pub is_locked: bool,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x39]
pub struct PlayerAbilities {
    pub flags: i8,
    pub flying_speed: f32,
    pub fov_modified: f32,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x62]
pub struct SetHeldItem {
    pub slot: VarInt,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x7E]
pub struct UpdateRecipes {
    pub property_sets: Vec<(String, Vec<VarInt>)>,
    pub stonecutter_recipes: Vec<(IdSet, SlotDisplay)>,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x1E]
pub struct EntityEvent {
    pub entity_id: i32,
    pub entity_status: i8,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x41]
pub struct SynchronizePlayerPosition {
    pub teleport_id: VarInt,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: i32,
}

#[derive(Debug, Serialize)]
#[sb_id = 0]
pub struct ConfirmTeleportation {
    pub teleport_id: VarInt,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x83]
pub struct Waypoint {
    pub operation: VarInt,
    pub identifier: Or<u128, String>,
    pub icon_style: String,
    pub color: Option<Color>,
    pub waypoint_data: WaypointData,
}

#[derive(Debug)]
pub enum WaypointData {
    Empty,
    Vec3i { x: VarInt, y: VarInt, z: VarInt },
    Chunk { x: VarInt, z: VarInt },
    Azimuth(f32),
}

impl Deserialize for WaypointData {
    fn deserialize(stream: &mut DataStream) -> Result<Self, DeserializeError> {
        let waypoint_type = VarInt::deserialize(stream)?.0;
        let r = match waypoint_type {
            0 => Self::Empty,
            1 => Self::Vec3i {
                x: VarInt::deserialize(stream)?,
                y: VarInt::deserialize(stream)?,
                z: VarInt::deserialize(stream)?,
            },
            2 => Self::Chunk {
                x: VarInt::deserialize(stream)?,
                z: VarInt::deserialize(stream)?,
            },
            3 => Self::Azimuth(f32::deserialize(stream)?),
            other => {
                return Err(DeserializeError::MalformedPacket(format!(
                    "Unkonwn waypoint type {}",
                    other
                )));
            }
        };
        Ok(r)
    }
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x2E]
pub struct UpdateEntityPosition {
    entity_id: VarInt,
    dx: i16,
    dy: i16,
    dz: i16,
    on_ground: bool,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x2F]
pub struct UpdateEntityPositionRotation {
    entity_id: VarInt,
    dx: i16,
    dy: i16,
    dz: i16,
    yaw: Angle,
    pitch: Angle,
    on_ground: bool,
}
