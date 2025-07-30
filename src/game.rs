mod entities;
mod entity;
mod player;
mod types;

pub use entities::*;
pub use entity::*;
pub use player::Player;
use thiserror::Error;
pub use types::*;



#[derive(Debug, Error)]
pub enum GameError {
    #[error("Unknown entity {0}")]
    UnkonwnEntity(EntityId)
}

#[derive(Debug, Default)]
pub struct Game {
    pub player: Player,
    pub entities: Entities
}
