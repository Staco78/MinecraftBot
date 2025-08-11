#![allow(dead_code)]

use std::marker::PhantomData;

use macros::Deserialize;

use crate::{
    data::Deserialize,
    datatypes::{BitSet, VarInt, deserialize_slice},
    game::{LocalBlockPos, world::data::palette_config::PaletteConfig},
    nbt::Nbt,
};

#[derive(Debug, Deserialize)]
pub struct ChunkData {
    pub heightmaps: Vec<Heightmap>,
    data_size: VarInt,
    pub chunk_sections: [ChunkSection; 24], // 24 only in overworld
    pub block_entities: Vec<BlockEntity>,
}

#[derive(Debug, Deserialize)]
pub struct BlockEntity {
    xz: u8,
    y: i16,
    block_entity_type: VarInt,
    data: Nbt,
}

#[derive(Debug, Deserialize)]
pub struct Heightmap {
    heightmap_type: HeightmapType,
    data: Vec<u64>,
}

#[derive(Debug, Deserialize)]
#[enum_repr(VarInt)]
pub enum HeightmapType {
    WorldSurface = 1,
    MotionBlocking = 4,
    MotionBlockingNoLeaves = 5,
}

#[derive(Debug, Deserialize)]
pub struct ChunkSection {
    pub block_count: u16,
    pub blocks: Palette<palette_config::Blocks>,
    pub biomes: Palette<palette_config::Biomes>,
}

mod palette_config {
    pub trait PaletteConfig {
        const ENTRIES_PER_AXE: usize;

        const DIRECT_BPE: i32;
    }

    #[derive(Debug)]
    pub struct Blocks;
    impl PaletteConfig for Blocks {
        const ENTRIES_PER_AXE: usize = 16;

        const DIRECT_BPE: i32 = 15;
    }

    #[derive(Debug)]
    pub struct Biomes;
    impl PaletteConfig for Biomes {
        const ENTRIES_PER_AXE: usize = 4;
        const DIRECT_BPE: i32 = 6;
    }
}

#[derive(Debug)]
pub enum Palette<CONFIG: PaletteConfig> {
    SingleValued {
        id: VarInt,
        _phantom: PhantomData<CONFIG>,
    },
    Indirect {
        bpe: i32,
        palette: Vec<VarInt>,
        data: Vec<u64>,
    },
    Direct {
        bpe: i32,
        data: Vec<u64>,
    },
}

impl<CONFIG: PaletteConfig> Deserialize for Palette<CONFIG> {
    fn deserialize(
        stream: &mut crate::data::DataStream,
    ) -> Result<Self, crate::data::DeserializeError> {
        let bpe = VarInt::deserialize(stream)?.0;

        // for indirect or direct palette
        let data_length = || {
            let entries_per_long = 64 / bpe;
            usize::div_ceil(
                usize::pow(CONFIG::ENTRIES_PER_AXE, 3),
                entries_per_long as usize,
            )
        };

        match bpe {
            0 => Ok(Self::SingleValued {
                id: VarInt::deserialize(stream)?,
                _phantom: PhantomData,
            }),
            _ if (0..CONFIG::DIRECT_BPE).contains(&bpe) => {
                let palette: Vec<VarInt> = Vec::deserialize(stream)?;
                let data_length = data_length();
                let data = deserialize_slice::<u64>(stream, data_length)?;
                Ok(Self::Indirect { bpe, palette, data })
            }
            _ if bpe == CONFIG::DIRECT_BPE => {
                let data_length = data_length();
                let data = deserialize_slice(stream, data_length)?;
                Ok(Self::Direct { bpe, data })
            }
            _ => panic!("Invalid bpe: {}", bpe),
        }
    }
}

impl<CONFIG: PaletteConfig> Palette<CONFIG> {
    pub fn get(&self, pos: LocalBlockPos) -> i32 {
        let LocalBlockPos { x, y, z } = pos;
        assert!((x as usize) < CONFIG::ENTRIES_PER_AXE);
        assert!((y as usize) < CONFIG::ENTRIES_PER_AXE);
        assert!((z as usize) < CONFIG::ENTRIES_PER_AXE);

        match self {
            Palette::SingleValued { id, .. } => id.0,
            Palette::Indirect { palette, data, bpe } => {
                let idx = Self::get_from_data(data, *bpe, pos);
                assert!(idx < palette.len());
                palette[idx].0
            }
            Palette::Direct { data, bpe } => Self::get_from_data(data, *bpe, pos) as i32,
        }
    }

    fn get_from_data(data: &[u64], bpe: i32, pos: LocalBlockPos) -> usize {
        let LocalBlockPos { x, y, z } = pos;
        let idx = ((y as usize * CONFIG::ENTRIES_PER_AXE) + z as usize) * CONFIG::ENTRIES_PER_AXE
            + x as usize;
        let entries_per_long = (64 / bpe) as usize;
        let long_idx = idx / entries_per_long;
        let offset = bpe as usize * (idx - (entries_per_long * long_idx));

        let mask = (1 << bpe) - 1;

        ((data[long_idx] >> offset) & mask) as usize
    }
}

#[derive(Debug, Deserialize)]
pub struct LightData {
    sky_light_mask: BitSet,
    block_light_mask: BitSet,
    empty_sky_light_mask: BitSet,
    empty_block_light_mask: BitSet,
    sky_light_arrays: Vec<Vec<u8>>,
    block_light_arrays: Vec<Vec<u8>>,
}
