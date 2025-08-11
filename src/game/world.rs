use std::collections::HashMap;

use parking_lot::RwLock;

use crate::{
    datatypes::BlockPos,
    game::{ChunkPos, ChunkSectionPos, LocalBlockPos, world::data::ChunkData},
};

pub mod data;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Chunk {
    heightmaps: Vec<data::Heightmap>,
    sections: [data::ChunkSection; 24],
    block_entities: Vec<data::BlockEntity>,
}

impl From<ChunkData> for Chunk {
    fn from(
        ChunkData {
            heightmaps,
            chunk_sections,
            block_entities,
            ..
        }: ChunkData,
    ) -> Self {
        Self {
            heightmaps,
            sections: chunk_sections,
            block_entities,
        }
    }
}

#[derive(Debug, Default)]
pub struct World {
    chunks: RwLock<HashMap<ChunkPos, Chunk>>,
}

impl World {
    pub fn block_at(&self, pos: BlockPos) -> Option<i32> {
        let section_pos = ChunkSectionPos::from_block_pos(pos);
        let chunk_pos = ChunkPos::from(section_pos);

        let chunks = self.chunks.read();
        let chunk = chunks.get(&chunk_pos)?;

        let section = &chunk.sections[(section_pos.y + 4) as usize];
        let local_pos = LocalBlockPos::from_global_block_pos(pos);
        Some(section.blocks.get(local_pos))
    }

    pub fn register_chunk_data(&self, pos: ChunkPos, data: Chunk) {
        dbg!(pos);
        self.chunks.write().insert(pos, data);
    }
}
