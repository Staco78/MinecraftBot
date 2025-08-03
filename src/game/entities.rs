use std::{collections::HashMap, sync::Arc};

use parking_lot::{ArcRwLockReadGuard, ArcRwLockWriteGuard, RwLock};

use crate::game::{Entity, EntityId};

pub type ReadGuard = ArcRwLockReadGuard<parking_lot::RawRwLock, Entity>;
pub type WriteGuard = ArcRwLockWriteGuard<parking_lot::RawRwLock, Entity>;

#[derive(Debug, Default)]
pub struct Entities(RwLock<HashMap<EntityId, EntityRef>>);

pub type EntityRef = Arc<RwLock<Entity>>;

impl Entities {
    pub fn add(&self, id: EntityId, entity: Entity) -> EntityRef {
        let ptr = Arc::new(RwLock::new(entity));
        self.0.write().insert(id, Arc::clone(&ptr));
        ptr
    }

    #[allow(dead_code)]
    pub fn get(&self, id: EntityId) -> Option<ReadGuard> {
        self.take(id).map(|ptr| ptr.read_arc())
    }

    pub fn get_mut(&self, id: EntityId) -> Option<WriteGuard> {
        self.take(id).map(|ptr| ptr.write_arc())
    }

    pub fn take(&self, id: EntityId) -> Option<EntityRef> {
        let map = self.0.read();
        map.get(&id).cloned()
    }
}
