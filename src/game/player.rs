use crate::game::entities::EntityRef;

#[derive(Debug, Default)]
pub struct Player {
    pub name: String,
    pub entity: EntityRef,
}
