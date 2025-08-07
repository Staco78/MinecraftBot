mod receive;
mod send;

use std::ops::Deref;

use parking_lot::RwLock;
pub use receive::*;
pub use send::*;

use macros::{Deserialize, Serialize};

use crate::{
    bitflags,
    data::{DataStream, Deserialize, DeserializeError, ReadWrite, Serialize, SerializeError},
    datatypes::{Angle, LengthInferredByteArray, Or, VarInt},
    game::{
        Color, Entity, EntityId, EntityRef, Game, GameError, IdSet, Rotation, SlotDisplay, Vec3,
        Vec3d, Vec3i, entities,
        world::data::{ChunkData, LightData},
    },
    nbt::Nbt,
};

#[derive(Debug, Serialize)]
#[sb_id = 0]
pub struct Handshake {
    pub protocol_version: VarInt,
    pub server_addr: String,
    pub server_port: u16,
    pub intent: VarInt,
}

#[derive(Debug, Serialize)]
#[sb_id = 0]
pub struct StatusRequest {}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StatusResponse {
    pub response: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[sb_id = 1]
pub struct PingPong {
    pub timestamp: i64,
}

// State Login

#[derive(Debug, Serialize)]
#[sb_id = 0]
pub struct LoginStart {
    // name length should be <= 16
    pub username: String,
    pub uuid: u128,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LoginSuccess {
    pub uuid: u128,
    pub username: String,
    pub property: Vec<PlayerProperty>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PlayerProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

impl ClientboundPacket for LoginSuccess {
    const ID: u32 = 2;
    const STATE: ConnectionState = ConnectionState::Login;
    const NEW_STATE: Option<ConnectionState> = Some(ConnectionState::Configuration);

    fn receive(self, stream: &mut dyn ReadWrite, game: &RwLock<Game>) -> Result<(), ReceiveError> {
        let player_entity = Entity {
            uuid: self.uuid,
            ..Default::default()
        };

        let mut game = game.write();

        game.player.entity = EntityRef::new(RwLock::new(player_entity)); // placeholder entity without id
        game.player.name = self.username;

        drop(game);

        send_packet(stream, LoginAcknowledged {})?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[sb_id = 3]
pub struct LoginAcknowledged {}

// State Configuration

#[derive(Debug, Deserialize, Serialize)]
#[sb_id = 2]
pub struct PluginMessage {
    pub channel: String,
    pub data: LengthInferredByteArray,
}

impl ClientboundPacket for PluginMessage {
    const ID: u32 = 1;
    const STATE: ConnectionState = ConnectionState::Configuration;
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct FeatureFlags(Vec<String>);

impl ClientboundPacket for FeatureFlags {
    const ID: u32 = 0x0C;
    const STATE: ConnectionState = ConnectionState::Configuration;
}

#[derive(Debug, Serialize, Deserialize)]
#[sb_id = 7]
pub struct KnownPacks(Vec<KnownPack>);

impl ClientboundPacket for KnownPacks {
    const ID: u32 = 0x0E;
    const STATE: ConnectionState = ConnectionState::Configuration;

    fn receive(self, stream: &mut dyn ReadWrite, _game: &RwLock<Game>) -> Result<(), ReceiveError> {
        send_packet(stream, self)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[sb_id = 3]
pub struct FinishConfiguration {}

impl ClientboundPacket for FinishConfiguration {
    const ID: u32 = 3;
    const STATE: ConnectionState = ConnectionState::Configuration;
    const NEW_STATE: Option<ConnectionState> = Some(ConnectionState::Play);

    fn receive(self, stream: &mut dyn ReadWrite, _game: &RwLock<Game>) -> Result<(), ReceiveError> {
        send_packet(stream, FinishConfiguration {})?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RegistryData {
    pub registry_id: String,
    pub entries: Vec<RegistryDataEntry>,
}

impl ClientboundPacket for RegistryData {
    const ID: u32 = 7;
    const STATE: ConnectionState = ConnectionState::Configuration;
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RegistryDataEntry {
    pub entry_id: String,
    pub data: Option<Nbt>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UpdateTags {
    pub tags_array: Vec<(String, Tags)>,
}

impl ClientboundPacket for UpdateTags {
    const ID: u32 = 0x0D;
    const STATE: ConnectionState = ConnectionState::Configuration;
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Tags(Vec<(String, Vec<VarInt>)>);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct DeathLocation {
    pub dimension_name: String,
    pub location: Vec3i,
}

impl ClientboundPacket for Login {
    const ID: u32 = 0x2B;
    const STATE: ConnectionState = ConnectionState::Play;

    fn receive(self, _stream: &mut dyn ReadWrite, game: &RwLock<Game>) -> Result<(), ReceiveError> {
        let mut game = game.write();

        let entity = game.player.entity.read().clone();
        let entity_ref = game.entities.add(self.entity_id, entity);
        game.player.entity = entity_ref;

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ChangeDifficulty {
    pub difficulty: u8,
    pub is_locked: bool,
}

impl ClientboundPacket for ChangeDifficulty {
    const ID: u32 = 0xA;
    const STATE: ConnectionState = ConnectionState::Play;
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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

impl ClientboundPacket for PlayerAbilities {
    const ID: u32 = 0x39;
    const STATE: ConnectionState = ConnectionState::Play;
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SetHeldItem {
    pub slot: VarInt,
}

impl ClientboundPacket for SetHeldItem {
    const ID: u32 = 0x62;
    const STATE: ConnectionState = ConnectionState::Play;
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UpdateRecipes {
    pub property_sets: Vec<(String, Vec<VarInt>)>,
    pub stonecutter_recipes: Vec<(IdSet, SlotDisplay)>,
}

impl ClientboundPacket for UpdateRecipes {
    const ID: u32 = 0x7E;
    const STATE: ConnectionState = ConnectionState::Play;
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct EntityEvent {
    pub id: EntityId,
    pub entity_status: i8,
}

impl ClientboundPacket for EntityEvent {
    const ID: u32 = 0x1E;
    const STATE: ConnectionState = ConnectionState::Play;
}

#[derive(Debug, Deserialize)]
pub struct SynchronizePlayerPosition {
    pub teleport_id: VarInt,
    pub position: Vec3d,
    pub speed: Vec3d,
    pub rotation: Rotation,
    pub flags: TeleportFlags,
}

impl ClientboundPacket for SynchronizePlayerPosition {
    const ID: u32 = 0x41;
    const STATE: ConnectionState = ConnectionState::Play;

    fn receive(self, stream: &mut dyn ReadWrite, game: &RwLock<Game>) -> Result<(), ReceiveError> {
        let game = game.read();
        let mut entity = game.player.entity.write_arc();
        drop(game);

        macro_rules! synchronize_axis {
            ($($field:ident).*, $flag: ident) => {
                if self.flags.contains(TeleportFlags:: $flag) {
                    entity.$($field).* += self.$($field).*;
                }
                else {
                    entity.$($field).* = self.$($field).*;
                }
            };
        }

        synchronize_axis!(position.x, RX);
        synchronize_axis!(position.y, RY);
        synchronize_axis!(position.z, RZ);

        synchronize_axis!(rotation.yaw, RYAW);
        synchronize_axis!(rotation.pitch, RPITCH);

        synchronize_axis!(speed.x, RVX);
        synchronize_axis!(speed.y, RVY);
        synchronize_axis!(speed.z, RVZ);

        drop(entity);

        if self.flags.contains(TeleportFlags::ROTATE_BEFORE) {
            todo!()
        }

        send_packet(
            stream,
            ConfirmTeleportation {
                teleport_id: self.teleport_id,
            },
        )?;
        Ok(())
    }
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Waypoint {
    pub operation: WaypointOperation,
    pub identifier: Or<u128, String>,
    pub icon_style: String,
    pub color: Option<Color>,
    pub waypoint_data: WaypointData,
}

impl ClientboundPacket for Waypoint {
    const ID: u32 = 0x83;
    const STATE: ConnectionState = ConnectionState::Play;
}

#[derive(Debug, Deserialize)]
#[enum_repr(VarInt)]
pub enum WaypointOperation {
    Track = 0,
    Untrack = 1,
    Update = 2,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[enum_repr(VarInt)]
pub enum WaypointData {
    Empty,
    Vec3i(Vec3<VarInt>),
    Chunk { x: VarInt, z: VarInt },
    Azimuth(f32),
}

fn update_entity_pos(
    id: EntityId,
    dx: i16,
    dy: i16,
    dz: i16,
    game: &Game,
) -> Result<entities::WriteGuard, ReceiveError> {
    let mut entity = game
        .entities
        .get_mut(id)
        .ok_or(GameError::UnkonwnEntity(id))?;

    let dpos = Vec3d {
        x: dx as f64 / 4096.,
        y: dy as f64 / 4096.,
        z: dz as f64 / 4096.,
    };
    entity.position += dpos;

    Ok(entity)
}

// FIXME: Move it elsewhere
fn entity_moved(
    entity: &Entity,
    stream: &mut dyn ReadWrite,
    game: impl Deref<Target = Game>,
) -> Result<(), SerializeError> {
    if entity.entity_type == 149 {
        let pos_diff = entity.position - game.player.entity.read().position;

        let mut yaw = -f64::atan2(pos_diff.x, pos_diff.z).to_degrees() as f32;
        if yaw < 0. {
            yaw += 360.;
        }
        let dist = Vec3d {
            x: pos_diff.x,
            y: 0.,
            z: pos_diff.z,
        }
        .length();
        let pitch = -f64::atan(pos_diff.y / dist).to_degrees() as f32;

        let new_rotation = Rotation { yaw, pitch };

        game.player.entity.write().rotation = new_rotation;
        drop(game);

        send_packet(
            stream,
            SetPlayerRotation {
                rotation: new_rotation,
                flags: PlayerPosFlags::empty(),
            },
        )?;
    }
    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UpdateEntityPosition {
    pub entity_id: VarInt,
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
    pub on_ground: bool,
}

impl ClientboundPacket for UpdateEntityPosition {
    const ID: u32 = 0x2E;
    const STATE: ConnectionState = ConnectionState::Play;

    fn receive(self, stream: &mut dyn ReadWrite, game: &RwLock<Game>) -> Result<(), ReceiveError> {
        let entity_id = self.entity_id.into();

        let game = game.read();

        let entity = update_entity_pos(entity_id, self.dx, self.dy, self.dz, &game)?;
        entity_moved(&entity, stream, game)?;

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UpdateEntityPositionRotation {
    pub entity_id: VarInt,
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
    pub yaw: Angle,
    pub pitch: Angle,
    pub on_ground: bool,
}

impl ClientboundPacket for UpdateEntityPositionRotation {
    const ID: u32 = 0x2F;
    const STATE: ConnectionState = ConnectionState::Play;

    fn receive(self, stream: &mut dyn ReadWrite, game: &RwLock<Game>) -> Result<(), ReceiveError> {
        let entity_id = self.entity_id.into();

        let game = game.read();

        let mut entity = update_entity_pos(entity_id, self.dx, self.dy, self.dz, &game)?;
        entity_moved(&entity, stream, game)?;

        entity.rotation = Rotation::from_angles(self.yaw, self.pitch);

        Ok(())
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct PlayerPosFlags: u8 {
        const ON_GROUND = 1;
        const PUSHING_WALL = 2;
    }
}

#[derive(Debug, Serialize)]
#[sb_id = 0x1D]
pub struct SetPlayerPosition {
    pub pos: Vec3d, // Y is feet Y
    pub flags: PlayerPosFlags,
}

#[derive(Debug, Serialize)]
#[sb_id = 0x1E]
pub struct SetPlayerPositionRotation {
    pub pos: Vec3d, // Y is feet Y
    pub rotation: Rotation,
    pub flags: PlayerPosFlags,
}

#[derive(Debug, Serialize)]
#[sb_id = 0x1F]
pub struct SetPlayerRotation {
    pub rotation: Rotation,
    pub flags: PlayerPosFlags,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PlayersInfoUpdate {
    pub players: Vec<(u128, Vec<PlayerAction>)>,
}

impl ClientboundPacket for PlayersInfoUpdate {
    const ID: u32 = 0x3F;
    const STATE: ConnectionState = ConnectionState::Play;
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

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct InitializeChatData {
    pub uuid: u128,
    pub key_expiry_time: i64,
    pub public_key: Vec<u8>,
    pub key_signature: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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

impl ClientboundPacket for AddEntity {
    const ID: u32 = 0x1;
    const STATE: ConnectionState = ConnectionState::Play;

    fn receive(self, _stream: &mut dyn ReadWrite, game: &RwLock<Game>) -> Result<(), ReceiveError> {
        let entity = Entity {
            uuid: self.uuid,
            position: self.pos,
            rotation: Rotation::from_angles(self.yaw, self.pitch),
            speed: Vec3d::speed_from_entity_velocity(self.vx, self.vy, self.vz),
            entity_type: self.entity_type.into(),
        };
        game.read().entities.add(self.entity_id.into(), entity);

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[sb_id = 0x1B]
pub struct KeepAlive(pub i64);

impl ClientboundPacket for KeepAlive {
    const ID: u32 = 0x26;
    const STATE: ConnectionState = ConnectionState::Play;

    fn receive(self, stream: &mut dyn ReadWrite, _game: &RwLock<Game>) -> Result<(), ReceiveError> {
        send_packet(stream, self)?;
        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct TeleportEntity {
    pub entity_id: VarInt,
    pub pos: Vec3d,
    pub speed: Vec3d,
    pub rotation: Rotation,
    pub on_ground: bool,
}

impl ClientboundPacket for TeleportEntity {
    const ID: u32 = 0x1F;
    const STATE: ConnectionState = ConnectionState::Play;

    fn receive(self, stream: &mut dyn ReadWrite, game: &RwLock<Game>) -> Result<(), ReceiveError> {
        let id = self.entity_id.into();

        let game = game.read();

        let mut entity = game
            .entities
            .get_mut(id)
            .ok_or(GameError::UnkonwnEntity(id))?;

        entity.position = self.pos;
        entity.speed = self.speed;
        entity.rotation = self.rotation;

        entity_moved(&entity, stream, game)?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct SetEntityVelocity {
    pub entity_id: VarInt,
    pub vx: i16,
    pub vy: i16,
    pub vz: i16,
}

impl ClientboundPacket for SetEntityVelocity {
    const ID: u32 = 0x5E;
    const STATE: ConnectionState = ConnectionState::Play;

    fn receive(self, _stream: &mut dyn ReadWrite, game: &RwLock<Game>) -> Result<(), ReceiveError> {
        let id = self.entity_id.into();
        let mut entity = game
            .read()
            .entities
            .get_mut(id)
            .ok_or(GameError::UnkonwnEntity(id))?;

        entity.speed = Vec3d::speed_from_entity_velocity(self.vx, self.vy, self.vz);

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ChunkDataWithLight {
    x: i32,
    y: i32,
    data: ChunkData,
    light: LightData,
}

impl ClientboundPacket for ChunkDataWithLight {
    const ID: u32 = 0x27;
    const STATE: ConnectionState = ConnectionState::Play;
}
