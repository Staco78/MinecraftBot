mod entity;
mod player;
mod types;

pub use entity::*;
pub use player::Player;
pub use types::*;

#[derive(Debug, Default)]
pub struct Game {
    pub player: Player,
    pub players: Vec<Player>,
}
