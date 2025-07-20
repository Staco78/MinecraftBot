use core::assert_ne;
use std::{
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
};

use thiserror::Error;

use crate::{
    data::{DataStream, Deserialize, DeserializeError, ReadWrite, SerializeError},
    datatypes::{LengthInferredByteArray, VarInt},
    packets::{ClientboundPacket},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
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
    state: State,
    callbacks: HashMap<u32, RealCb<'a>>,
    _phantom: PhantomData<&'a ()>,
}

// type Cb<'a, T> = FnMut(T) -> Result<(), ReceiveError> ;
type RealCb<'a> = Box<(dyn FnMut(&mut DataStream) -> Result<(), ReceiveError> + 'a)>;

impl<'a> PacketReceiver<'a> {
    pub fn new() -> Self {
        Self {
            state: State::Handshaking,
            callbacks: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Panic if the state is the same as before.
    ///
    /// Clear all callbacks.
    pub fn change_state(&mut self, new_state: State) {
        assert_ne!(new_state, self.state);
        self.state = new_state;
        self.callbacks.clear();
    }

    pub fn set_callback<T: ClientboundPacket + Debug>(
        &mut self,
        mut cb: impl FnMut(T, &mut dyn ReadWrite) -> Result<(), ReceiveError> + 'a,
    ) {
        let real_cb: RealCb = Box::new(
            move |stream: &mut crate::data::DataStream| -> Result<(), ReceiveError> {
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
                cb(packet, stream)
            },
        );

        self.callbacks.insert(T::ID, real_cb);

        // // Safety: this way is safe
        // let cb: u128 = unsafe { core::mem::transmute(cb) };
        // self.callbacks.insert(T::ID, cb);
    }

    pub fn receive_packet(&mut self, stream: &mut dyn ReadWrite) -> Result<(), ReceiveError> {
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
        assert!(id >= 0);
        self.call_cb(&mut stream, id as u32)
    }

    fn call_cb(
        &mut self,
        stream: &mut DataStream,
        id: u32,
    ) -> Result<(), ReceiveError> {
        // let cb = *self
        //     .callbacks
        //     .get(&id)
        //     .ok_or(ReceiveError::UnknownPacketId(id))?;

        // Safety: transmuting the other way around
        // let cb: Cb<'a, T> = unsafe { core::mem::transmute(cb) };

        match self.callbacks.get_mut(&id) {
            Some(cb) => cb(stream),
            None => {
                LengthInferredByteArray::deserialize(stream)?;
                Err(ReceiveError::UnknownPacketId(id))
            }
        }
    }
}
