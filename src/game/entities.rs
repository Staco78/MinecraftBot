use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    rc::Rc,
};

use crate::game::{Entity, EntityId};

#[derive(Debug, Default)]
pub struct Entities(HashMap<EntityId, EntityRef>);

pub type EntityRef = Rc<RefCell<Entity>>;

impl Entities {
    /// Add a new entity.
    ///
    /// Panic if an entity with this `id` already exists
    pub fn add(&mut self, id: EntityId, entity: Entity) -> EntityRef {
        let ptr = Rc::new(RefCell::new(entity));
        let r = self.0.insert(id, Rc::clone(&ptr));
        assert!(r.is_none());
        ptr
    }

    pub fn get(&self, id: EntityId) -> Option<Ref<Entity>> {
        self.0.get(&id).map(|r| r.borrow())
    }

    pub fn get_mut(&self, id: EntityId) -> Option<RefMut<Entity>> {
        self.0.get(&id).map(|r| r.borrow_mut())
    }

    pub fn take(&self, id: EntityId) -> Option<EntityRef> {
        self.0.get(&id).cloned()
    }
}
