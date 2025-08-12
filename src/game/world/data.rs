#![allow(dead_code)]

use std::marker::PhantomData;

use super::palette::palette_config::{self, PaletteConfig};

use macros::Deserialize;

use crate::{
    data::Deserialize,
    datatypes::{BitSet, VarInt, deserialize_slice},
    nbt::Nbt,
};

#[derive(Debug, Deserialize)]
pub struct ChunkData {
    pub heightmaps: Vec<Heightmap>,
    data_size: VarInt,
    pub chunk_sections: [ProtocolChunkSection; 24], // 24 only in overworld
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
pub struct ProtocolChunkSection {
    pub block_count: u16,
    pub blocks: ProtocolPalette<palette_config::Blocks>,
    pub biomes: ProtocolPalette<palette_config::Biomes>,
}

#[derive(Debug)]
pub enum ProtocolPalette<CONFIG: PaletteConfig> {
    SingleValued {
        id: VarInt,
        _phantom: PhantomData<CONFIG>,
    },
    Indirect {
        bpe: u32,
        palette: Vec<VarInt>,
        data: Vec<u64>,
    },
    Direct {
        bpe: u32,
        data: Vec<u64>,
    },
}

impl<CONFIG: PaletteConfig> Deserialize for ProtocolPalette<CONFIG> {
    fn deserialize(
        stream: &mut crate::data::DataStream,
    ) -> Result<Self, crate::data::DeserializeError> {
        let bpe = VarInt::deserialize(stream)?.0 as u32;

        // for indirect or direct palette
        let data_length = || {
            let entries_per_long = 64 / bpe;
            usize::div_ceil(CONFIG::ENTRIES_COUNT, entries_per_long as usize)
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

#[derive(Debug, Deserialize)]
pub struct LightData {
    sky_light_mask: BitSet,
    block_light_mask: BitSet,
    empty_sky_light_mask: BitSet,
    empty_block_light_mask: BitSet,
    sky_light_arrays: Vec<Vec<u8>>,
    block_light_arrays: Vec<Vec<u8>>,
}
