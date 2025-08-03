#![allow(clippy::uninlined_format_args)]

use std::{error::Error, net::TcpStream, ops::Deref, process::exit};

use parking_lot::RwLock;

use crate::{
    data::{DeserializeError, ReadWrite, SerializeError},
    datatypes::VarInt,
    game::{
        Entity, EntityId, EntityRef, Game, GameError, Rotation, Vec3d, entities, start_gameloop,
    },
    packets::{
        AddEntity, ChangeDifficulty, ConfirmTeleportation, ConnectionState, EntityEvent,
        FeatureFlags, FinishConfiguration, Handshake, KeepAlive, KnownPacks, Login,
        LoginAcknowledged, LoginStart, LoginSuccess, PacketReceiver, PlayerAbilities,
        PlayerPosFlags, PlayersInfoUpdate, PluginMessage, ReceiveError, RegistryData,
        SetEntityVelocity, SetHeldItem, SetPlayerRotation, SynchronizePlayerPosition,
        TeleportEntity, TeleportFlags, UpdateEntityPosition, UpdateEntityPositionRotation,
        UpdateRecipes, UpdateTags, Waypoint, init_multithread, send_collected_packets, send_packet,
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

        let mut game = game.write();

        game.player.entity = EntityRef::new(RwLock::new(player_entity)); // placeholder entity without id
        game.player.name = packet.username;

        drop(game);

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

    receiver.set_callback(|packet: KeepAlive, stream, _| {
        send_packet(stream, packet)?;
        Ok(None)
    });

    receiver.set_callback(|packet: Login, _stream, game| {
        let mut game = game.write();

        let entity = game.player.entity.read().clone();
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
        let game = game.read();
        let mut entity = game.player.entity.write_arc();
        drop(game);

        macro_rules! synchronize_axis {
            ($($field:ident).*, $flag: ident) => {
                if packet.flags.contains(TeleportFlags:: $flag) {
                    entity.$($field).* += packet.$($field).*;
                }
                else {
                    entity.$($field).* = packet.$($field).*;
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

        if packet.flags.contains(TeleportFlags::ROTATE_BEFORE) {
            todo!()
        }

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

        let game = game.read();

        let entity = update_entity_pos(entity_id, packet.dx, packet.dy, packet.dz, &game)?;
        entity_moved(&entity, stream, game)?;

        Ok(None)
    });
    receiver.set_callback(|packet: UpdateEntityPositionRotation, stream, game| {
        let entity_id = packet.entity_id.into();

        let game = game.read();

        let mut entity = update_entity_pos(entity_id, packet.dx, packet.dy, packet.dz, &game)?;
        entity_moved(&entity, stream, game)?;

        entity.rotation = Rotation::from_angles(packet.yaw, packet.pitch);

        Ok(None)
    });

    receiver.set_callback(|packet: TeleportEntity, stream, game| {
        let id = packet.entity_id.into();

        let game = game.read();

        let mut entity = game
            .entities
            .get_mut(id)
            .ok_or(GameError::UnkonwnEntity(id))?;

        entity.position = packet.pos;
        entity.speed = packet.speed;
        entity.rotation = packet.rotation;

        entity_moved(&entity, stream, game)?;

        Ok(None)
    });

    receiver.set_callback(|packet: SetEntityVelocity, _, game| {
        let id = packet.entity_id.into();
        let mut entity = game
            .read()
            .entities
            .get_mut(id)
            .ok_or(GameError::UnkonwnEntity(id))?;

        entity.speed = Vec3d::speed_from_entity_velocity(packet.vx, packet.vy, packet.vz);

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
        game.read().entities.add(packet.entity_id.into(), entity);

        Ok(None)
    });

    let inter_threads_receiver = init_multithread();
    start_gameloop(receiver.game());

    loop {
        let r = receiver.receive_packet(&mut stream);
        match r {
            Err(ReceiveError::DeserializeError(DeserializeError::Io(e))) => {
                println!("IO ERROR: {e}");
                exit(-1);
            }
            e => println!("{:?}", e),
        }

        send_collected_packets(&inter_threads_receiver, &mut stream)?;
    }
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
