use crate::game::entity::Entity;

#[derive(Debug, Default)]
pub struct Player {
    pub name: String,
    pub uuid: u128,
    pub entity: Entity,
}
