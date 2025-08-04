use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use thiserror::Error;

use crate::{
    data::{DataStream, Deserialize, DeserializeError, ReadWrite, SerializeError},
    datatypes::{LengthInferredByteArray, VarInt},
    game::{Game, GameError},
    packets::{
        AddEntity, ChangeDifficulty, EntityEvent, FeatureFlags, FinishConfiguration, KeepAlive,
        KnownPacks, Login, LoginSuccess, PlayerAbilities, PlayersInfoUpdate, PluginMessage,
        RegistryData, SetEntityVelocity, SetHeldItem, SynchronizePlayerPosition, TeleportEntity,
        UpdateEntityPosition, UpdateEntityPositionRotation, UpdateRecipes, UpdateTags, Waypoint,
    },
};

pub trait ClientboundPacket: Deserialize {
    const ID: u32;
    const STATE: ConnectionState;
    const NEW_STATE: Option<ConnectionState> = None;

    fn receive(
        self,
        _stream: &mut dyn ReadWrite,
        _game: &RwLock<Game>,
    ) -> Result<(), ReceiveError> {
        Ok(())
    }

    fn receive_(stream: &mut DataStream, game: &RwLock<Game>) -> Result<(), ReceiveError> {
        let packet = match Self::deserialize(stream) {
            Ok(packet) => packet,
            Err(DeserializeError::Io(e)) => return Err(DeserializeError::Io(e).into()),
            Err(e) => {
                // Read the remaining bytes
                LengthInferredByteArray::deserialize(stream)?;
                return Err(e.into());
            }
        };
        packet.receive(stream, game)?;
        if stream.remaining_size() > 0 {
            println!("WARN: Packet still has data to read");
            println!(
                "data: {:?}",
                LengthInferredByteArray::deserialize(stream)?.0
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Handshaking,
    #[allow(unused)]
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

    #[error("Game error: {0}")]
    GameError(#[from] GameError),
}

pub struct PacketReceiver<'a> {
    state: ConnectionState,
    _phantom: PhantomData<&'a ()>,
    game: Arc<RwLock<Game>>,
}

impl<'a> PacketReceiver<'a> {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Handshaking,
            _phantom: PhantomData,
            game: Arc::default(),
        }
    }

    pub fn game(&self) -> Arc<RwLock<Game>> {
        Arc::clone(&self.game)
    }

    pub fn set_state(&mut self, new_state: ConnectionState) {
        assert_ne!(new_state, self.state);
        self.state = new_state;
    }

    pub fn get_state(&self) -> ConnectionState {
        self.state
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
        self.receive_packet_(&mut stream, id as u32)
    }

    fn receive_packet_(&mut self, stream: &mut DataStream, id: u32) -> Result<(), ReceiveError> {
        macro_rules! receive {
            ($($type: ty),*) => {
                match id {
                    $(<$type>::ID if self.state == <$type>::STATE => {
                        <$type>::receive_(stream, &self.game)?;
                        if let Some(state) = <$type>::NEW_STATE {
                            self.set_state(state);
                        }
                        Ok(())
                    })*
                    _ => {
                        LengthInferredByteArray::deserialize(stream)?;
                        Err(ReceiveError::UnknownPacketId(id))
                    }
                }
            };
        }

        receive!(
            LoginSuccess,
            PluginMessage,
            FeatureFlags,
            KnownPacks,
            FinishConfiguration,
            RegistryData,
            UpdateTags,
            Login,
            ChangeDifficulty,
            PlayerAbilities,
            SetHeldItem,
            UpdateRecipes,
            EntityEvent,
            SynchronizePlayerPosition,
            Waypoint,
            UpdateEntityPosition,
            UpdateEntityPositionRotation,
            PlayersInfoUpdate,
            AddEntity,
            KeepAlive,
            TeleportEntity,
            SetEntityVelocity
        )
    }
}
