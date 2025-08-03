pub mod entities;
mod entity;
mod player;
mod types;
mod gameloop;

pub use entities::{Entities, EntityRef};
pub use entity::*;
pub use player::Player;
use thiserror::Error;
pub use types::*;
pub use gameloop::*;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Unknown entity {0}")]
    UnkonwnEntity(EntityId),
}

#[derive(Debug, Default)]
pub struct Game {
    pub player: Player,
    pub entities: Entities,
}
