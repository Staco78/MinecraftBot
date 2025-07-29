#![allow(clippy::uninlined_format_args)]

use std::{error::Error, net::TcpStream, process::exit};

use crate::{
    data::{DeserializeError, ReadWrite},
    datatypes::VarInt,
    game::{Rotation, Vec3d, Vec3i},
    packets::{
        ChangeDifficulty, ConfirmTeleportation, ConnectionState, EntityEvent, FeatureFlags,
        FinishConfiguration, Handshake, KnownPacks, Login, LoginAcknowledged, LoginStart,
        LoginSuccess, PacketReceiver, PlayerAbilities, PlayerPosFlags, PluginMessage, ReceiveError,
        RegistryData, SetHeldItem, SetPlayerRotation, SynchronizePlayerPosition, TeleportFlags,
        UpdateEntityPosition, UpdateEntityPositionRotation, UpdateRecipes, UpdateTags, Waypoint,
        WaypointData, send_packet,
    },
};

mod data;
mod datatypes;
mod game;
mod nbt;
mod packets;
mod utils;

const PROTOCOL_VERSION: i32 = 772;

fn main() -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:25565")?;
    // stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    let mut receiver = PacketReceiver::new();

    send_packet(
        &mut stream,
        Handshake {
            protocol_version: VarInt(PROTOCOL_VERSION),
            server_addr: "127.0.0.1".into(),
            server_port: 25565,
            intent: ConnectionState::Login.handshake_intent(),
        },
    )?;
    receiver.change_state(ConnectionState::Login);

    send_packet(
        &mut stream,
        LoginStart {
            username: "Coucou".to_string(),
            uuid: 0,
        },
    )?;

    receiver.set_callback(|packet: LoginSuccess, stream: &mut dyn ReadWrite, game| {
        game.player.uuid = packet.uuid;
        game.player.name = packet.username;

        send_packet(stream, LoginAcknowledged {})?;
        Ok(Some(ConnectionState::Configuration))
    });

    while receiver.get_state() != ConnectionState::Configuration {
        receiver.receive_packet(&mut stream)?;
    }

    receiver.set_callback(|_packet: PluginMessage, _stream: &mut dyn ReadWrite, _game| Ok(None));

    receiver.set_callback(|_packet: FeatureFlags, _stream: &mut dyn ReadWrite, _game| Ok(None));

    receiver.set_callback(|packet: KnownPacks, stream: &mut dyn ReadWrite, _game| {
        send_packet(stream, packet)?;
        Ok(None)
    });

    receiver.set_callback(
        |_packet: FinishConfiguration, stream: &mut dyn ReadWrite, _game| {
            send_packet(stream, FinishConfiguration {})?;
            Ok(Some(ConnectionState::Play))
        },
    );

    receiver.set_callback(|_packet: RegistryData, _stream, _game| Ok(None));

    receiver.set_callback(|_packet: UpdateTags, _stream, _game| Ok(None));

    while receiver.get_state() != ConnectionState::Play {
        receiver.receive_packet(&mut stream)?;
    }

    receiver.set_callback(|packet: Login, _stream, game| {
        game.player.entity.id = packet.entity_id;

        Ok(None)
    });
    receiver.set_callback(|_packet: ChangeDifficulty, _, _game| Ok(None));
    receiver.set_callback(|_packet: PlayerAbilities, _, _game| Ok(None));
    receiver.set_callback(|_packet: SetHeldItem, _, _game| Ok(None));
    receiver.set_callback(|_packet: UpdateRecipes, _, _game| Ok(None));
    receiver.set_callback(|_packet: EntityEvent, _, _game| Ok(None));
    receiver.set_callback(|packet: SynchronizePlayerPosition, stream, game| {
        macro_rules! synchronize_axis {
            ($($field:ident).*, $flag: ident) => {
                if packet.flags.contains(TeleportFlags:: $flag) {
                    game.player.entity.$($field).* += packet.$($field).*;
                }
                else {
                    game.player.entity.$($field).* = packet.$($field).*;
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

        if packet.flags.contains(TeleportFlags::ROTATE_BEFORE) {
            todo!()
        }

        dbg!(&game);

        send_packet(
            stream,
            ConfirmTeleportation {
                teleport_id: packet.teleport_id,
            },
        )?;
        Ok(None)
    });
    receiver.set_callback(|packet: Waypoint, stream, game| {
        if let WaypointData::Vec3i(pos) = packet.waypoint_data {
            let pos_diff = Vec3d::middle_of(Vec3i::from(pos)) - game.player.entity.position;
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

            dbg!(yaw, pitch);

            send_packet(
                stream,
                SetPlayerRotation {
                    rotation: Rotation { yaw, pitch },
                    flags: PlayerPosFlags::empty(),
                },
            )?;
        }

        Ok(None)
    });
    receiver.set_callback(|_packet: UpdateEntityPosition, _, _game| Ok(None));
    receiver.set_callback(|_packet: UpdateEntityPositionRotation, _, _game| Ok(None));

    loop {
        let r = receiver.receive_packet(&mut stream);
        match r {
            Err(ReceiveError::DeserializeError(DeserializeError::Io(e))) => {
                println!("IO ERROR: {e}");
                exit(-1);
            }
            e => println!("{:?}", e),
        }
    }
}
