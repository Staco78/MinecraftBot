use std::collections::{HashMap, hash_map::Entry};

use parking_lot::RwLock;

use crate::{
    datatypes::BlockPos,
    game::{
        ChunkPos, ChunkSectionPos, LocalPos,
        world::{
            data::{ChunkData, ProtocolChunkSection},
            palette::{Palette, palette_config},
        },
    },
};

pub mod data;
mod palette;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Chunk {
    heightmaps: Vec<data::Heightmap>,
    sections: [ChunkSection; 24],
    block_entities: Vec<data::BlockEntity>,
}

impl Chunk {
    fn empty() -> Self {
        Self {
            heightmaps: Vec::new(),
            sections: [const { ChunkSection::empty() }; 24],
            block_entities: Vec::new(),
        }
    }
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
            sections: chunk_sections.map(|v| v.into()),
            block_entities,
        }
    }
}

#[derive(Debug)]
pub struct ChunkSection {
    pub block_count: u16,
    pub blocks: Palette<palette_config::Blocks>,
    #[allow(dead_code)]
    pub biomes: Palette<palette_config::Biomes>,
}

impl From<ProtocolChunkSection> for ChunkSection {
    fn from(value: ProtocolChunkSection) -> Self {
        Self {
            block_count: value.block_count,
            blocks: value.blocks.into(),
            biomes: value.biomes.into(),
        }
    }
}

impl ChunkSection {
    const fn empty() -> Self {
        Self {
            block_count: 0,
            blocks: Palette::empty(),
            biomes: Palette::empty(),
        }
    }

    fn set_block(&mut self, pos: LocalPos, block: i32) {
        let old = self.blocks.set(pos, block);
        if old == block {
            return;
        }
        if old == 0 {
            self.block_count += 1;
        }
        if block == 0 {
            self.block_count -= 1;
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
        let local_pos = LocalPos::from_global_block_pos(pos);
        Some(section.blocks.get(local_pos))
    }

    pub fn set_block(&self, pos: BlockPos, block: i32) {
        let section_pos = ChunkSectionPos::from_block_pos(pos);
        let local_pos = LocalPos::from_global_block_pos(pos);

        let mut chunks = self.chunks.write();
        let chunk = match chunks.entry(section_pos.into()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                if block == 0 {
                    // air
                    return;
                }

                e.insert(Chunk::empty())
            }
        };

        chunk.sections[(section_pos.y + 4) as usize].set_block(local_pos, block);
    }

    pub fn register_chunk_data(&self, pos: ChunkPos, data: Chunk) {
        self.chunks.write().insert(pos, data);
    }
}
