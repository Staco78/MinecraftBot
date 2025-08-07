use macros::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BitSet(Vec<u64>);
