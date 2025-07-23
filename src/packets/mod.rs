#![allow(dead_code)]

mod receive;

pub use receive::*;

use macros::{Deserialize, Serialize};

use crate::{
    data::{DataStream, Deserialize, DeserializeError, ReadWrite, Serialize, SerializeError},
    datatypes::{LengthInferredByteArray, VarInt},
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

#[derive(Debug, Deserialize, Serialize)]
#[cb_id = 1]
#[sb_id = 2]
pub struct PluginMessage {
    channel: String,
    data: LengthInferredByteArray,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0x0C]
pub struct FeatureFlags {
    feature_flags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cb_id = 0x0E]
#[sb_id = 7]
pub struct KnownPacks {
    known_packs: Vec<KnownPack>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KnownPack {
    namespace: String,
    id: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[sb_id = 3]
#[cb_id = 3]
pub struct FinishConfiguration {}

#[derive(Debug, Deserialize)]
#[cb_id = 7]
pub struct RegistryData {
    registry_id: String,
    entries: Vec<RegistryDataEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryDataEntry {
    entry_id: String,
    data: Option<Nbt>,
}

#[derive(Debug, Deserialize)]
#[cb_id = 0xD]
pub struct UpdateTags {
    tags_array: Vec<(String, Tags)>,
}

#[derive(Debug, Deserialize)]
pub struct Tags {
    tags: Vec<(String, Vec<VarInt>)>,
}
