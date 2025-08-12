use std::{collections::HashMap, marker::PhantomData};

use palette_config::PaletteConfig;

use crate::game::{LocalPos, world::data::ProtocolPalette};

pub mod palette_config {
    use std::fmt::Debug;

    pub trait PaletteConfig: Debug {
        const ENTRIES_PER_AXE: usize;

        const DIRECT_BPE: u32;

        const ENTRIES_COUNT: usize =
            Self::ENTRIES_PER_AXE * Self::ENTRIES_PER_AXE * Self::ENTRIES_PER_AXE;

        /// How much entries in an indirect palette before converting it to direct
        const MAX_INDIRECT_ENTRIES: usize;
    }

    #[derive(Debug)]
    pub struct Blocks;
    impl PaletteConfig for Blocks {
        const ENTRIES_PER_AXE: usize = 16;
        const DIRECT_BPE: u32 = 15;
        const MAX_INDIRECT_ENTRIES: usize = 256;
    }

    #[derive(Debug)]
    pub struct Biomes;
    impl PaletteConfig for Biomes {
        const ENTRIES_PER_AXE: usize = 4;
        const DIRECT_BPE: u32 = 6;
        const MAX_INDIRECT_ENTRIES: usize = 8;
    }
}

#[derive(Debug)]
pub enum Palette<CONFIG: PaletteConfig> {
    SingleValued {
        id: i32,
        _phantom: PhantomData<CONFIG>,
    },
    Indirect {
        bpe: u32,
        palette2id: Vec<i32>,
        id2palette: HashMap<i32, usize>,
        data: Vec<u64>,
    },
    Direct {
        bpe: u32,
        data: Vec<u64>,
    },
}

impl<CONFIG: PaletteConfig> From<ProtocolPalette<CONFIG>> for Palette<CONFIG> {
    fn from(value: ProtocolPalette<CONFIG>) -> Self {
        match value {
            ProtocolPalette::SingleValued { id, .. } => Self::SingleValued {
                id: id.0,
                _phantom: PhantomData,
            },
            ProtocolPalette::Indirect { bpe, palette, data } => Self::Indirect {
                bpe,
                palette2id: palette.iter().map(|v| v.0).collect(),
                id2palette: palette.iter().enumerate().map(|(a, b)| (b.0, a)).collect(),
                data,
            },
            ProtocolPalette::Direct { bpe, data } => Self::Direct { bpe, data },
        }
    }
}

impl<CONFIG: PaletteConfig> Palette<CONFIG> {
    pub fn get(&self, pos: LocalPos) -> i32 {
        let LocalPos { x, y, z } = pos;
        assert!((x as usize) < CONFIG::ENTRIES_PER_AXE);
        assert!((y as usize) < CONFIG::ENTRIES_PER_AXE);
        assert!((z as usize) < CONFIG::ENTRIES_PER_AXE);

        match self {
            Palette::SingleValued { id, .. } => *id,
            Palette::Indirect {
                palette2id,
                data,
                bpe,
                ..
            } => {
                let idx = Self::get_from_data(data, *bpe, pos);
                assert!(idx < palette2id.len());
                palette2id[idx]
            }
            Palette::Direct { data, bpe } => Self::get_from_data(data, *bpe, pos) as i32,
        }
    }

    fn get_from_data(data: &[u64], bpe: u32, pos: LocalPos) -> usize {
        let LocalPos { x, y, z } = pos;
        let idx = ((y as usize * CONFIG::ENTRIES_PER_AXE) + z as usize) * CONFIG::ENTRIES_PER_AXE
            + x as usize;
        let entries_per_long = (64 / bpe) as usize;
        let long_idx = idx / entries_per_long;
        let offset = bpe as usize * (idx - (entries_per_long * long_idx));

        let mask = (1 << bpe) - 1;

        ((data[long_idx] >> offset) & mask) as usize
    }

    /// Return the old value
    fn set_from_data(data: &mut [u64], bpe: u32, pos: LocalPos, value: i32) -> i32 {
        let LocalPos { x, y, z } = pos;
        let idx = ((y as usize * CONFIG::ENTRIES_PER_AXE) + z as usize) * CONFIG::ENTRIES_PER_AXE
            + x as usize;
        let entries_per_long = (64 / bpe) as usize;
        let long_idx = idx / entries_per_long;
        let offset = bpe as usize * (idx - (entries_per_long * long_idx));

        let mask = ((1 << bpe) - 1) << offset;
        let value = ((value as u64) << offset) & mask;

        let old_value = (data[long_idx] & mask) >> offset;

        data[long_idx] &= !mask;
        data[long_idx] |= value;
        
        old_value as i32
    }

    pub const fn empty() -> Self {
        Self::SingleValued {
            id: 0,
            _phantom: PhantomData,
        }
    }

    pub fn set(&mut self, pos: LocalPos, id: i32) -> i32 {
        match self {
            Palette::Direct { bpe, data } => Self::set_from_data(&mut *data, *bpe, pos, id),
            Palette::SingleValued { id: old_id, .. } => {
                let (bpe, mut data) = Self::single_to_direct(*old_id);
                let old = Self::set_from_data(&mut data, bpe, pos, id);
                *self = Palette::Direct { bpe, data };
                old
            }
            Palette::Indirect {
                bpe,
                id2palette,
                palette2id,
                data,
            } => {
                if id2palette.contains_key(&id) {
                    Self::set_indirect(*bpe, id2palette, palette2id, data, pos, id)
                } else if id2palette.len() < CONFIG::MAX_INDIRECT_ENTRIES {
                    let palette_id = palette2id.len();
                    palette2id.push(id);
                    id2palette.insert(id, palette_id);
                    let new_bpe = usize::BITS - palette_id.leading_zeros();
                    if new_bpe != *bpe {
                        Self::indirect_rebuild_new_bpe(*bpe, new_bpe, data);
                        *bpe = new_bpe;
                    }
                    Self::set_indirect(*bpe, id2palette, palette2id, data, pos, id)
                } else {
                    let mut data = Self::indirect_to_direct(*bpe, palette2id, data);
                    let old = Self::set_from_data(&mut data, CONFIG::DIRECT_BPE, pos, id);
                    *self = Palette::Direct {
                        bpe: CONFIG::DIRECT_BPE,
                        data,
                    };
                    old
                }
            }
        }
    }

    fn single_to_direct(id: i32) -> (u32, Vec<u64>) {
        let bpe = CONFIG::DIRECT_BPE;
        let entries_per_long = (64 / bpe) as usize;
        let data_length = usize::div_ceil(CONFIG::ENTRIES_COUNT, entries_per_long);
        let long: u64 = {
            let mut val = 0;
            for _ in 0..entries_per_long {
                val <<= bpe;
                val |= id as u64;
            }
            val
        };
        let data = vec![long; data_length];
        (bpe, data)
    }

    fn indirect_to_direct(bpe: u32, palette2id: &[i32], data: &[u64]) -> Vec<u64> {
        Self::indirect_rebuild_new_bpe_map(bpe, CONFIG::DIRECT_BPE, data, |palette_id| {
            palette2id[palette_id as usize]
        })
    }

    fn indirect_rebuild_new_bpe(old_bpe: u32, new_bpe: u32, data: &mut Vec<u64>) {
        *data = Self::indirect_rebuild_new_bpe_map(old_bpe, new_bpe, data, |a| a);
    }

    fn indirect_rebuild_new_bpe_map<F: Fn(i32) -> i32>(
        old_bpe: u32,
        new_bpe: u32,
        data: &[u64],
        map: F,
    ) -> Vec<u64> {
        let new_data_length = usize::div_ceil(CONFIG::ENTRIES_COUNT, (64 / new_bpe) as usize);
        let mut new_data = vec![0; new_data_length];

        let mut old_long_idx = 0;
        let mut old_offset = 0;
        let old_mask = (1 << old_bpe) - 1;

        let mut new_long_idx = 0;
        let mut new_offset = 0;
        let new_mask = (1 << new_bpe) - 1;

        for _ in 0..CONFIG::ENTRIES_COUNT {
            if old_offset + old_bpe > 64 {
                old_offset = 0;
                old_long_idx += 1;
            }
            if new_offset + new_bpe > 64 {
                new_offset = 0;
                new_long_idx += 1;
            }

            let val = (data[old_long_idx] >> old_offset) & old_mask;
            let val = map(val as i32) as u64;

            let mask = new_mask << new_offset;
            new_data[new_long_idx] &= !mask;
            new_data[new_long_idx] |= val << new_offset;

            old_offset += old_bpe;
            new_offset += new_bpe;
        }

        new_data
    }

    fn set_indirect(
        bpe: u32,
        id2palette: &HashMap<i32, usize>,
        palette2id: &[i32],
        data: &mut [u64],
        pos: LocalPos,
        id: i32,
    ) -> i32 {
        let old_palette_id = Self::set_from_data(
            data,
            bpe,
            pos,
            *id2palette.get(&id).expect("Palette should contains id") as i32,
        );
        palette2id[old_palette_id as usize]
    }
}
