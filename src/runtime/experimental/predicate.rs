use crate::common::*;
use crate::runtime::ProtocolS;
use core::ops::Index;
use core::ops::IndexMut;

use std::collections::BTreeMap;

// we assume a dense ChannelIndex domain!

enum CommonSatisfier<T> {
    FormerNotLatter,
    LatterNotFormer,
    Equivalent,
    New(T),
    Nonexistant,
}

type ChunkType = u16;
const MASK_BITS: ChunkType = 0x_AA_AA; // 101010...

#[test]
fn mask_ok() {
    assert_eq!(!0, MASK_BITS | (MASK_BITS >> 1));
    assert_eq!(0, MASK_BITS & (MASK_BITS >> 1));
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct TernChunk(ChunkType); // invariant: no pair is 01

impl TernChunk {
    fn overwrite(&mut self, index: usize, value: bool) -> Option<bool> {
        assert!(index < Self::vars_per_chunk());
        let mask_bit_mask = 1 << (index * 2 + 1);
        let bool_bit_mask = 1 << (index * 2);
        let ret = if self.0 & mask_bit_mask != 0 {
            let was_value = self.0 & bool_bit_mask != 0;
            if was_value != value {
                // flip the value bit
                self.0 ^= bool_bit_mask;
            }
            Some(was_value)
        } else {
            if value {
                // set the value bit
                self.0 |= bool_bit_mask;
            }
            None
        };
        // set the mask bit
        self.0 |= mask_bit_mask;
        ret
    }
    fn new_singleton(index: usize, value: bool) -> Self {
        assert!(index < Self::vars_per_chunk());
        let mask_bits = 1 << (index * 2 + 1);
        let maybe_bit: ChunkType = value as ChunkType;
        assert_eq!(maybe_bit == 1, value);
        assert!(maybe_bit <= 1);
        let bool_bits = maybe_bit << (index * 2);
        Self(mask_bits | bool_bits)
    }
    const fn vars_per_chunk() -> usize {
        std::mem::size_of::<ChunkType>() / 2
    }
    #[inline]
    fn query(self, index: usize) -> Option<bool> {
        assert!(index < Self::vars_per_chunk());
        let mask_bit_mask = 1 << (index * 2 + 1);
        let bool_bit_mask = 1 << (index * 2);
        if self.0 & mask_bit_mask != 0 {
            Some(self.0 & bool_bit_mask != 0)
        } else {
            None
        }
    }
    fn mutual_satisfaction(self, othe: Self) -> [bool; 2] {
        let s_mask = self.0 & MASK_BITS;
        let o_mask = othe.0 & MASK_BITS;
        let both_mask = s_mask & o_mask;
        let diff = self.0 ^ othe.0;
        let masked_diff = diff & (both_mask >> 1);
        if masked_diff != 0 {
            [false; 2]
        } else {
            let s_sat_o = s_mask & !o_mask == 0;
            let o_sat_s = o_mask & !s_mask == 0;
            [s_sat_o, o_sat_s]
        }
    }

    /// Returns whether self satisfies other
    /// false iff either:
    /// 1. there exists a pair which you specify and I dont.
    //    i.e., self has 00, othe has 1?
    /// 2. we both specify a variable with different values.
    ///    i.e., self has 10, othe has 11 or vice versa.
    fn satisfies(self, othe: Self) -> bool {
        let s_mask = self.0 & MASK_BITS;
        let o_mask = othe.0 & MASK_BITS;
        let both_mask = s_mask & o_mask;
        let diff = self.0 ^ othe.0;

        // FALSE if othe has a 1X pair where self has a 1(!X) pair
        let masked_diff = diff & (both_mask >> 1);

        // FALSE if othe has a 1X pair where self has a 0Y pair.
        let o_not_s_mask = o_mask & !s_mask;

        o_not_s_mask | masked_diff == 0
    }

    fn common_satisfier(self, othe: Self) -> Option<Self> {
        let s_mask = self.0 & MASK_BITS;
        let o_mask = othe.0 & MASK_BITS;
        let both_mask = s_mask & o_mask;
        let diff = self.0 ^ othe.0;
        let masked_diff = diff & (both_mask >> 1);
        if masked_diff != 0 {
            // an inconsistency exists
            None
        } else {
            let s_vals = (s_mask >> 1) & self.0;
            let o_vals = (o_mask >> 1) & othe.0;
            let new = s_mask | o_mask | s_vals | o_vals;
            Some(Self(new))
        }
    }
}

struct TernSet(Vec<TernChunk>); // invariant: last byte != 00
impl TernSet {
    fn new_singleton(index: ChannelIndex, value: bool) -> Self {
        let which_chunk = index as usize / TernChunk::vars_per_chunk();
        let inner_index = index as usize % TernChunk::vars_per_chunk();
        let it = std::iter::repeat(TernChunk(0))
            .take(which_chunk)
            .chain(std::iter::once(TernChunk::new_singleton(inner_index, value)));
        Self(it.collect())
    }
    fn overwrite(&mut self, index: ChannelIndex, value: bool) -> Option<bool> {
        let which_chunk = index as usize / TernChunk::vars_per_chunk();
        let inner_index = index as usize % TernChunk::vars_per_chunk();
        if let Some(tern_chunk) = self.0.get_mut(which_chunk) {
            tern_chunk.overwrite(inner_index, value)
        } else {
            self.0.reserve(which_chunk - self.0.len());
            self.0.resize(which_chunk, TernChunk(0));
            self.0.push(TernChunk::new_singleton(inner_index, value));
            None
        }
    }

    fn query(&self, index: ChannelIndex) -> Option<bool> {
        let which_chunk = index as usize / TernChunk::vars_per_chunk();
        self.0.get(which_chunk).copied().and_then(move |tern_chunk| {
            tern_chunk.query(index as usize % TernChunk::vars_per_chunk())
        })
    }
    fn satisfies(&self, othe: &Self) -> bool {
        self.0.len() >= othe.0.len() && self.0.iter().zip(&othe.0).all(|(s, o)| s.satisfies(*o))
    }
    fn common_satisfier(&self, othe: &Self) -> CommonSatisfier<Self> {
        use CommonSatisfier as Cs;
        let [slen, olen] = [self.0.len(), othe.0.len()];
        let [mut s_sat_o, mut o_sat_s] = [slen >= olen, slen <= olen];
        for (s, o) in self.0.iter().zip(&othe.0) {
            let [s2, o2] = s.mutual_satisfaction(*o);
            s_sat_o &= s2;
            o_sat_s &= o2;
        }
        match [s_sat_o, o_sat_s] {
            [true, true] => Cs::Equivalent,
            [true, false] => Cs::FormerNotLatter,
            [false, true] => Cs::LatterNotFormer,
            [false, false] => Cs::New(Self(
                self.0.iter().zip(&othe.0).map(|(s, o)| s.common_satisfier(*o).unwrap()).collect(),
            )),
        }
    }
    #[inline]
    fn restore_invariant(&mut self) {
        while self.0.iter().copied().last() == Some(TernChunk(0)) {
            self.0.pop();
        }
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

struct Predicate(BTreeMap<ControllerId, TernSet>);
impl Predicate {
    pub fn overwrite(&mut self, channel_id: ChannelId, value: bool) -> Option<bool> {
        let ChannelId { controller_id, channel_index } = channel_id;
        use std::collections::btree_map::Entry;
        match self.0.entry(controller_id) {
            Entry::Occupied(mut x) => x.get_mut().overwrite(channel_index, value),
            Entry::Vacant(x) => {
                x.insert(TernSet::new_singleton(channel_index, value));
                None
            }
        }
    }
    pub fn query(&self, channel_id: ChannelId) -> Option<bool> {
        let ChannelId { controller_id, channel_index } = channel_id;
        self.0.get(&controller_id).and_then(move |tern_set| tern_set.query(channel_index))
    }
    pub fn satisfies(&self, other: &Self) -> bool {
        let mut s_it = self.0.iter();
        let mut s = if let Some(s) = s_it.next() {
            s
        } else {
            return other.0.is_empty();
        };
        for (oid, ob) in other.0.iter() {
            while s.0 < oid {
                s = if let Some(s) = s_it.next() {
                    s
                } else {
                    return false;
                };
            }
            if s.0 > oid || !s.1.satisfies(ob) {
                return false;
            }
        }
        true
    }

    pub fn common_satisfier(&self, othe: &Self) -> CommonSatisfier<Self> {
        // use CommonSatisfier as Cs;
        // let [slen, olen] = [self.0.len(), othe.0.len()];
        // let [mut s_sat_o, mut o_sat_s] = [slen >= olen, slen <= olen];
        // let [mut s_it, mut o_it] = [self.0.iter(), othe.0.iter()];
        // let [mut s, mut o] = [s_it.next(), o_it.next()];
        todo!()
    }
}

////////////////////////////
