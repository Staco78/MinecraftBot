#![allow(clippy::uninlined_format_args)]

use std::{error::Error, net::TcpStream};

use crate::{
    data::ReadWrite,
    datatypes::VarInt,
    packets::{
        AcknowledgeFinishConfiguration, FeatureFlags, Handshake, KnownPacks, LoginAcknowledged,
        LoginStart, LoginSuccess, PacketReceiver, PluginMessage, send_packet,
    },
};

mod data;
mod datatypes;
mod packets;

fn main() -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:25565")?;
    // stream.set_read_timeout(Some(Duration::from_secs(1)))?;

    send_packet(
        &mut stream,
        Handshake {
            protocol_version: VarInt(772),
            server_addr: "127.0.0.1".into(),
            server_port: 25565,
            intent: VarInt(2),
        },
    )?;

    send_packet(
        &mut stream,
        LoginStart {
            username: "Coucou".to_string(),
            uuid: 0,
        },
    )?;

    let mut receiver = PacketReceiver::new();
    receiver.set_callback(|_packet: LoginSuccess, stream: &mut dyn ReadWrite| {
        send_packet(stream, LoginAcknowledged {})?;
        Ok(())
    });

    receiver.set_callback(|_packet: PluginMessage, _stream: &mut dyn ReadWrite| Ok(()));

    receiver.set_callback(|_packet: FeatureFlags, _stream: &mut dyn ReadWrite| Ok(()));

    receiver.set_callback(|packet: KnownPacks, stream: &mut dyn ReadWrite| {
        send_packet(stream, packet)?;
        send_packet(stream, AcknowledgeFinishConfiguration {})?;
        Ok(())
    });

    loop {
        println!("{:?}", receiver.receive_packet(&mut stream));
    }
}
