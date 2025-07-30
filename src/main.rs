#![allow(clippy::uninlined_format_args)]

use std::{
    cell::{RefCell, RefMut},
    error::Error,
    net::TcpStream,
    process::exit,
};

use crate::{
    data::{DeserializeError, ReadWrite},
    datatypes::VarInt,
    game::{Entity, EntityId, EntityRef, Game, GameError, Rotation, Vec3d},
    packets::{
        AddEntity, ChangeDifficulty, ConfirmTeleportation, ConnectionState, EntityEvent,
        FeatureFlags, FinishConfiguration, Handshake, KnownPacks, Login, LoginAcknowledged,
        LoginStart, LoginSuccess, PacketReceiver, PlayerAbilities, PlayerPosFlags,
        PlayersInfoUpdate, PluginMessage, ReceiveError, RegistryData, SetHeldItem,
        SetPlayerRotation, SynchronizePlayerPosition, TeleportFlags, UpdateEntityPosition,
        UpdateEntityPositionRotation, UpdateRecipes, UpdateTags, Waypoint, send_packet,
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
        let player_entity = Entity {
            uuid: packet.uuid,
            ..Default::default()
        };

        game.player.entity = EntityRef::new(RefCell::new(player_entity)); // placeholder entity without id
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
        let entity = game.player.entity.take();
        let entity_ref = game.entities.add(packet.entity_id, entity);
        game.player.entity = entity_ref;

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
                    game.player.entity.borrow_mut().$($field).* += packet.$($field).*;
                }
                else {
                    game.player.entity.borrow_mut().$($field).* = packet.$($field).*;
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
    receiver.set_callback(|_packet: Waypoint, _, _| Ok(None));
    receiver.set_callback(|packet: UpdateEntityPosition, stream, game| {
        let entity_id = packet.entity_id.into();
        update_entity_pos(entity_id, packet.dx, packet.dy, packet.dz, stream, game)?;

        Ok(None)
    });
    receiver.set_callback(|packet: UpdateEntityPositionRotation, stream, game| {
        let entity_id = packet.entity_id.into();

        let mut entity =
            update_entity_pos(entity_id, packet.dx, packet.dy, packet.dz, stream, game)?;

        entity.rotation = Rotation::from_angles(packet.yaw, packet.pitch);

        Ok(None)
    });

    receiver.set_callback(|_packet: PlayersInfoUpdate, _, _| Ok(None));

    receiver.set_callback(|packet: AddEntity, _, game| {
        let entity = Entity {
            uuid: packet.uuid,
            position: packet.pos,
            rotation: Rotation::from_angles(packet.yaw, packet.pitch),
            speed: Vec3d::speed_from_entity_velocity(packet.vx, packet.vy, packet.vz),
            entity_type: packet.entity_type.into(),
        };
        game.entities.add(packet.entity_id.into(), entity);

        dbg!(&game.entities);

        Ok(None)
    });

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

fn update_entity_pos<'a>(
    id: EntityId,
    dx: i16,
    dy: i16,
    dz: i16,
    stream: &mut dyn ReadWrite,
    game: &'a mut Game,
) -> Result<RefMut<'a, Entity>, ReceiveError> {
    let mut entity = game
        .entities
        .get_mut(id)
        .ok_or::<ReceiveError>(GameError::UnkonwnEntity(id).into())?;
    let dpos = Vec3d {
        x: dx as f64 / 4096.,
        y: dy as f64 / 4096.,
        z: dz as f64 / 4096.,
    };
    entity.position += dpos;

    if entity.entity_type == 149 {
        let pos_diff = entity.position - game.player.entity.borrow().position;
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

    Ok(entity)
}
