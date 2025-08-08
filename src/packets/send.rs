use std::{
    fmt::Debug,
    io::{self},
    sync::{
        OnceLock,
        mpsc::{Receiver, Sender, TryRecvError, channel},
    },
};

use log::info;

use crate::{
    data::{ReadWrite, Serialize, SerializeError},
    datatypes::VarInt,
};

pub trait ServerboundPacket: Serialize {
    const ID: u32;
}

pub fn send_packet<T: ServerboundPacket>(
    stream: &mut dyn ReadWrite,
    packet: T,
) -> Result<(), SerializeError> {
    info!("Sending packet {}", T::ID);

    let id = VarInt(T::ID as _);
    let packet_size = packet.size();
    let size = packet_size + id.size();

    VarInt(size as i32).serialize(stream)?;
    id.serialize(stream)?;
    packet.serialize(stream)?;

    Ok(())
}

static SENDER: OnceLock<Sender<Vec<u8>>> = OnceLock::new();

pub fn init_multithread() -> Receiver<Vec<u8>> {
    let (sender, receiver) = channel();

    SENDER.set(sender).expect("Multithread already initialized");

    receiver
}

pub fn send_packet_from_thread<T: ServerboundPacket + Debug>(packet: T) -> Result<(), SerializeError> {
    info!("Sending packet {}", T::ID);

    let id = VarInt(T::ID as _);
    let packet_size = packet.size();
    let size = packet_size + id.size();

    let mut vec: Vec<u8> = Vec::with_capacity(size);
    let stream = &mut vec;

    VarInt(size as i32).serialize(stream)?;
    id.serialize(stream)?;
    packet.serialize(stream)?;

    SENDER
        .get()
        .expect("Multithread not initialized")
        .send(vec)
        .expect("RECEIVER is closed");

    Ok(())
}

pub fn send_collected_packets(
    receiver: &Receiver<Vec<u8>>,
    stream: &mut dyn ReadWrite,
) -> Result<(), io::Error> {
    while let Ok(data) = match receiver.try_recv() {
        Ok(o) => Ok(o),
        Err(TryRecvError::Disconnected) => panic!("Disconnected"),
        Err(e) => Err(e),
    } {
        stream.write_all(&data)?;
    }

    Ok(())
}
