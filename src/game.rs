pub mod entities;
mod entity;
mod gameloop;
mod player;
mod types;
pub mod world;

pub use entities::{Entities, EntityRef};
pub use entity::*;
pub use gameloop::*;
pub use player::Player;
use thiserror::Error;
pub use types::*;

use crate::game::world::World;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Unknown entity {0}")]
    UnkonwnEntity(EntityId),
}

#[derive(Debug, Default)]
pub struct Game {
    pub player: Player,
    pub entities: Entities,
    pub world: World,
}
