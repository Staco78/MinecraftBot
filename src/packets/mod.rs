#![allow(dead_code)]

mod receive;

pub use receive::*;

use macros::{Deserialize, Serialize};

use crate::{
    bitflags,
    data::{DataStream, Deserialize, DeserializeError, ReadWrite, Serialize, SerializeError},
    datatypes::{Angle, LengthInferredByteArray, Or, VarInt},
    game::{Color, EntityId, IdSet, Rotation, SlotDisplay, Vec3, Vec3d, Vec3i},
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
    pub property: Vec<PlayerProperty>,
}

#[derive(Debug, Deserialize)]
pub struct PlayerProperty {
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
pub struct FeatureFlags(Vec<String>);

#[derive(Debug, Serialize, Deserialize)]
#[cb_id = 0x0E]
#[sb_id = 7]
pub struct KnownPacks(Vec<KnownPack>);

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
pub struct Tags(Vec<(String, Vec<VarInt>)>);

/// State Play

#[derive(Debug, Deserialize)]
#[cb_id = 0x2B]
pub struct Login {
    pub entity_id: EntityId,
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
    pub location: Vec3i,
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
    pub flags: PlayerAbilitiesFlags,
    pub flying_speed: f32,
    pub fov_modified: f32,
}

bitflags! {
    #[derive(Debug)]
    pub struct PlayerAbilitiesFlags: u8 {
        const INVULNERABLE = 0x1;
        const FLYING = 0x2;
        const ALLOW_FLYING = 0x4;
        const CREATIVE_MODE = 0x8;
    }
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
    pub id: EntityId,
    pub entity_status: i8,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x41]
pub struct SynchronizePlayerPosition {
    pub teleport_id: VarInt,
    pub position: Vec3d,
    pub speed: Vec3d,
    pub rotation: Rotation,
    pub flags: TeleportFlags,
}

bitflags! {
    #[derive(Debug)]
    pub struct TeleportFlags: i32 {
        const RX = 0b1;
        const RY = 0b10;
        const RZ = 0b100;

        const RYAW = 0b1000;
        const RPITCH = 0b10000;

        const RVX = 0b100000;
        const RVY = 0b1000000;
        const RVZ = 0b10000000;

        const ROTATE_BEFORE = 0x100;
    }
}

#[derive(Debug, Serialize)]
#[sb_id = 0]
pub struct ConfirmTeleportation {
    pub teleport_id: VarInt,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x83]
pub struct Waypoint {
    pub operation: WaypointOperation,
    pub identifier: Or<u128, String>,
    pub icon_style: String,
    pub color: Option<Color>,
    pub waypoint_data: WaypointData,
}

#[derive(Debug, Deserialize)]
#[enum_repr(VarInt)]
pub enum WaypointOperation {
    Track = 0,
    Untrack = 1,
    Update = 2,
}

#[derive(Debug, Deserialize)]
#[enum_repr(VarInt)]
pub enum WaypointData {
    Empty,
    Vec3i(Vec3<VarInt>),
    Chunk { x: VarInt, z: VarInt },
    Azimuth(f32),
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x2E]
pub struct UpdateEntityPosition {
    pub entity_id: VarInt,
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
    pub on_ground: bool,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x2F]
pub struct UpdateEntityPositionRotation {
    pub entity_id: VarInt,
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
    pub yaw: Angle,
    pub pitch: Angle,
    pub on_ground: bool,
}

bitflags! {
    #[derive(Debug)]
    pub struct PlayerPosFlags: u8 {
        const ON_GROUND = 1;
        const PUSHING_WALL = 2;
    }
}

#[derive(Debug, Serialize)]
#[sb_id = 0x1F]
pub struct SetPlayerRotation {
    pub rotation: Rotation,
    pub flags: PlayerPosFlags,
}

#[derive(Debug)]
pub struct PlayersInfoUpdate {
    pub players: Vec<(u128, Vec<PlayerAction>)>,
}

impl ClientboundPacket for PlayersInfoUpdate {
    const ID: u32 = 0x3F;
}

impl Deserialize for PlayersInfoUpdate {
    fn deserialize(stream: &mut DataStream) -> Result<Self, DeserializeError> {
        let actions = PlayerActionFlag::deserialize(stream)?;
        let players_count = VarInt::deserialize(stream)?.0 as usize;

        let players = (0..players_count)
            .map(|_| {
                let uuid = u128::deserialize(stream)?;
                let player_actions = actions
                    .iter()
                    .map(|action| {
                        let r = match action {
                            PlayerActionFlag::ADD_PLAYER => PlayerAction::AddPlayer {
                                name: String::deserialize(stream)?,
                                properties: Vec::deserialize(stream)?,
                            },
                            PlayerActionFlag::INITIALIZE_CHAT => {
                                PlayerAction::InitializeChat(Option::deserialize(stream)?)
                            }
                            PlayerActionFlag::UPDATE_GAME_MODE => {
                                PlayerAction::UpdateGameMode(VarInt::deserialize(stream)?)
                            }
                            PlayerActionFlag::UPDATE_LISTED => {
                                PlayerAction::UpdateListed(bool::deserialize(stream)?)
                            }
                            PlayerActionFlag::UPDATE_LATENCY => {
                                PlayerAction::UpdateLatency(VarInt::deserialize(stream)?)
                            }
                            PlayerActionFlag::UPDATE_DISPLAY_NAME => {
                                PlayerAction::UpdateDisplayName(Option::deserialize(stream)?)
                            }
                            PlayerActionFlag::UPDATE_LIST_PRIORITY => {
                                PlayerAction::UpdateListPriority(VarInt::deserialize(stream)?)
                            }
                            PlayerActionFlag::UPDATE_HAT => {
                                PlayerAction::UpdateHat(bool::deserialize(stream)?)
                            }
                            _ => unimplemented!(),
                        };
                        Ok(r)
                    })
                    .collect::<Result<Vec<_>, DeserializeError>>()?;
                Ok((uuid, player_actions))
            })
            .collect::<Result<Vec<_>, DeserializeError>>()?;

        Ok(PlayersInfoUpdate { players })
    }
}

bitflags! {
    #[derive(Debug, PartialEq)]
    pub struct PlayerActionFlag: u8 {
        const ADD_PLAYER = 1;
        const INITIALIZE_CHAT = 2;
        const UPDATE_GAME_MODE = 4;
        const UPDATE_LISTED = 8;
        const UPDATE_LATENCY = 16;
        const UPDATE_DISPLAY_NAME = 32;
        const UPDATE_LIST_PRIORITY = 64;
        const UPDATE_HAT = 128;
    }
}

#[derive(Debug)]
pub enum PlayerAction {
    AddPlayer {
        name: String,
        properties: Vec<PlayerProperty>,
    },
    InitializeChat(Option<InitializeChatData>),
    UpdateGameMode(VarInt),
    UpdateListed(bool),
    UpdateLatency(VarInt),
    UpdateDisplayName(Option<String>),
    UpdateListPriority(VarInt),
    UpdateHat(bool),
}

#[derive(Debug, Deserialize)]
pub struct InitializeChatData {
    pub uuid: u128,
    pub key_expiry_time: i64,
    pub public_key: Vec<u8>,
    pub key_signature: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x1]
pub struct AddEntity {
    pub entity_id: VarInt,
    pub uuid: u128,
    pub entity_type: VarInt,
    pub pos: Vec3d,
    pub pitch: Angle,
    pub yaw: Angle,
    pub head_yaw: Angle,
    pub data: VarInt,
    pub vx: i16,
    pub vy: i16,
    pub vz: i16,
}

#[derive(Debug, Serialize, Deserialize)]
#[cb_id = 0x26]
#[sb_id = 0x1B]
pub struct KeepAlive(pub i64);

#[derive(Debug, Deserialize)]
#[cb_id = 0x1F]
pub struct TeleportEntity {
    pub entity_id: VarInt,
    pub pos: Vec3d,
    pub speed: Vec3d,
    pub rotation: Rotation,
    pub on_ground: bool,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x5E]
pub struct SetEntityVelocity {
    pub entity_id: VarInt,
    pub vx: i16,
    pub vy: i16,
    pub vz: i16
}
