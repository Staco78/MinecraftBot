#![allow(clippy::uninlined_format_args)]

use std::{error::Error, net::TcpStream, process::exit};

use log::{LevelFilter, error, warn};

use crate::{
    data::DeserializeError,
    datatypes::VarInt,
    game::start_gameloop,
    packets::{
        ConnectionState, Handshake, LoginStart, PacketReceiver, ReceiveError, init_multithread,
        send_collected_packets, send_packet,
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
    env_logger::builder()
        .filter_level(LevelFilter::Trace)
        .format_timestamp(None)
        .format_source_path(false)
        .format_file(true)
        .init();

    let mut stream = TcpStream::connect("127.0.0.1:25565")?;
    stream.set_nodelay(true)?;
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
    receiver.set_state(ConnectionState::Login);

    send_packet(
        &mut stream,
        LoginStart {
            username: "Coucou".to_string(),
            uuid: 0,
        },
    )?;

    while receiver.get_state() != ConnectionState::Configuration {
        receiver.receive_packet(&mut stream)?;
    }

    while receiver.get_state() != ConnectionState::Play {
        receiver.receive_packet(&mut stream)?;
    }

    let inter_threads_receiver = init_multithread();
    start_gameloop(receiver.game());

    loop {
        let r = receiver.receive_packet(&mut stream);
        match r {
            Err(ReceiveError::DeserializeError(DeserializeError::Io(e))) => {
                error!("IO ERROR: {e}");
                exit(-1);
            }
            Err(ReceiveError::UnknownPacketId(id)) => warn!("Packet {id} ignored"),
            Err(e) => error!("{:?}", e),
            Ok(()) => (),
        }

        send_collected_packets(&inter_threads_receiver, &mut stream)?;
    }
}
