#![allow(dead_code)]

use std::io::{Read, Write};

use macros::{Deserialize, Serialize};

use crate::{
    data::{Deserialize, DeserializeError, Serialize, SerializeError},
    datatypes::{LengthInferredByteArray, VarInt},
};

pub trait ServerboundPacket: Serialize {
    const ID: u32;
}

pub trait ClientboundPacket: Deserialize {
    const ID: u32;
}

pub fn send_packet<T: ServerboundPacket>(
    to: &mut dyn Write,
    packet: T,
) -> Result<(), SerializeError> {
    let id = VarInt(T::ID as i32);
    let size = packet.size() + id.size();
    assert!(size <= i32::MAX as _);
    VarInt(size as i32).serialize(to)?;
    id.serialize(to)?;
    packet.serialize(to)
}

pub fn receive_packet<T: ClientboundPacket>(from: &mut dyn Read) -> Result<T, DeserializeError> {
    let mut size = VarInt::read(from)? as usize;
    let read_id = VarInt::deserialize(from, &mut size)?;
    assert_eq!(read_id.0, T::ID as i32);
    T::deserialize(from, &mut size)
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

#[derive(Debug, Serialize)]
#[sb_id = 3]
pub struct AcknowledgeFinishConfiguration {}
