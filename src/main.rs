#![allow(clippy::uninlined_format_args)]

use std::{error::Error, net::TcpStream};

use crate::{
    data::{DataStream, Deserialize},
    datatypes::{LengthInferredByteArray, VarInt},
    packets::{
        receive_packet, send_packet, AcknowledgeFinishConfiguration, FeatureFlags, Handshake, KnownPacks, LoginAcknowledged, LoginStart, LoginSuccess, PluginMessage
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

    let success: LoginSuccess = receive_packet(&mut stream)?;
    dbg!(success);

    send_packet(&mut stream, LoginAcknowledged {})?;

    let plugin_message: PluginMessage = receive_packet(&mut stream)?;
    dbg!(plugin_message);

    let feature_flags: FeatureFlags = receive_packet(&mut stream)?;
    dbg!(feature_flags);

    let known_packs: KnownPacks = receive_packet(&mut stream)?;
    dbg!(&known_packs);

    send_packet(&mut stream, known_packs)?;

    send_packet(&mut stream, AcknowledgeFinishConfiguration {})?;

    loop {
        let size = match VarInt::read(&mut stream) {
            Ok(r) => r as usize,
            Err(e) => {
                println!("{:?}", e);
                continue;
            }
        };
        let mut stream = DataStream::new(&mut stream, size);
        let id = VarInt::deserialize(&mut stream)?;
        dbg!(id);
        LengthInferredByteArray::deserialize(&mut stream)?;
    }
}
