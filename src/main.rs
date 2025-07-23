#![allow(clippy::uninlined_format_args)]

use std::{error::Error, net::TcpStream};

use crate::{
    data::ReadWrite,
    datatypes::VarInt,
    packets::{
        ConnectionState, FeatureFlags, FinishConfiguration, Handshake, KnownPacks,
        LoginAcknowledged, LoginStart, LoginSuccess, PacketReceiver, PluginMessage, RegistryData,
        UpdateTags, send_packet,
    },
};

mod data;
mod datatypes;
mod nbt;
mod packets;

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

    receiver.set_callback(|_packet: LoginSuccess, stream: &mut dyn ReadWrite| {
        send_packet(stream, LoginAcknowledged {})?;
        Ok(Some(ConnectionState::Configuration))
    });

    while receiver.get_state() != ConnectionState::Configuration {
        receiver.receive_packet(&mut stream)?;
    }

    receiver.set_callback(|_packet: PluginMessage, _stream: &mut dyn ReadWrite| Ok(None));

    receiver.set_callback(|_packet: FeatureFlags, _stream: &mut dyn ReadWrite| Ok(None));

    receiver.set_callback(|packet: KnownPacks, stream: &mut dyn ReadWrite| {
        send_packet(stream, packet)?;
        Ok(None)
    });

    receiver.set_callback(|_packet: FinishConfiguration, stream: &mut dyn ReadWrite| {
        send_packet(stream, FinishConfiguration {})?;
        Ok(Some(ConnectionState::Play))
    });

    receiver.set_callback(|_packet: RegistryData, _stream| Ok(None));

    receiver.set_callback(|_packet: UpdateTags, _stream| Ok(None));

    while receiver.get_state() != ConnectionState::Play {
        receiver.receive_packet(&mut stream)?;
    }

    loop {
        println!("{:?}", receiver.receive_packet(&mut stream));
    }
}
