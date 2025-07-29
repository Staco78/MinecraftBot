use core::assert_ne;
use std::{collections::HashMap, fmt::Debug, marker::PhantomData};

use thiserror::Error;

use crate::{
    data::{DataStream, Deserialize, DeserializeError, ReadWrite, SerializeError},
    datatypes::{LengthInferredByteArray, VarInt},
    packets::ClientboundPacket,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
}

impl ConnectionState {
    pub fn handshake_intent(&self) -> VarInt {
        match self {
            ConnectionState::Status => VarInt(1),
            ConnectionState::Login => VarInt(2),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ReceiveError {
    #[error("Received an packet of unknown id {0}")]
    UnknownPacketId(u32),

    #[error("Error while serializing: {0}")]
    SerializeError(#[from] SerializeError),

    #[error("Error while deserializing: {0}")]
    DeserializeError(#[from] DeserializeError),
}

// #[derive(Debug)]
pub struct PacketReceiver<'a> {
    state: ConnectionState,
    callbacks: HashMap<u32, RealCb<'a>>,
    _phantom: PhantomData<&'a ()>,
}

type RealCb<'a> =
    Box<(dyn FnMut(&mut DataStream) -> Result<Option<ConnectionState>, ReceiveError> + 'a)>;

impl<'a> PacketReceiver<'a> {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Handshaking,
            callbacks: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Clear all callbacks.
    ///
    /// Panic if the state is the same as before.
    pub fn change_state(&mut self, new_state: ConnectionState) {
        assert_ne!(new_state, self.state);
        self.state = new_state;
        self.callbacks.clear();
    }

    pub fn get_state(&self) -> ConnectionState {
        self.state
    }

    pub fn set_callback<T: ClientboundPacket + Debug>(
        &mut self,
        mut cb: impl FnMut(T, &mut dyn ReadWrite) -> Result<Option<ConnectionState>, ReceiveError> + 'a,
    ) {
        let real_cb: RealCb = Box::new(move |stream: &mut crate::data::DataStream| {
            let packet = match T::deserialize(stream) {
                Ok(packet) => packet,
                Err(DeserializeError::Io(e)) => return Err(DeserializeError::Io(e).into()),
                Err(e) => {
                    // Read the remaining bytes
                    LengthInferredByteArray::deserialize(stream)?;
                    return Err(e.into());
                }
            };
            dbg!(&packet);
            let r = cb(packet, stream)?;
            if stream.remaining_size() > 0 {
                println!("WARN: Packet still has data to read");
            }
            Ok(r)
        });

        self.callbacks.insert(T::ID, real_cb);
    }

    pub fn receive_packet(&mut self, stream: &mut dyn ReadWrite) -> Result<(), ReceiveError> {
        dbg!(self.state);
        let size = VarInt::read(stream)?;
        if size <= 0 {
            return Err(DeserializeError::MalformedPacket(format!(
                "Negative packet size (found {})",
                size
            ))
            .into());
        }
        let size = size as usize;
        let mut stream = DataStream::new(stream, size);

        let id = VarInt::deserialize(&mut stream)?.0;
        dbg!(id);
        assert!(id >= 0);
        self.call_cb(&mut stream, id as u32)
    }

    fn call_cb(&mut self, stream: &mut DataStream, id: u32) -> Result<(), ReceiveError> {
        match self.callbacks.get_mut(&id) {
            Some(cb) => {
                let state = cb(stream)?;
                if let Some(new_state) = state {
                    self.change_state(new_state);
                }
                Ok(())
            }
            None => {
                LengthInferredByteArray::deserialize(stream)?;
                Err(ReceiveError::UnknownPacketId(id))
            }
        }
    }
}
