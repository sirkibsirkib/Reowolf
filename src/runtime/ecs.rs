use crate::common::*;
use crate::runtime::ProtocolS;

use std::collections::HashMap;

/// invariant: last element is not zero.
/// => all values out of bounds are implicitly absent.
/// i.e., &[0,1] means {1<<32, 0} while &[0,1] is identical to &[1] and means {1}.

#[derive(Debug, Default)]
struct BitSet(Vec<u32>);
impl BitSet {
    #[inline(always)]
    fn index_decomposed(index: usize) -> [usize; 2] {
        // [chunk_index, chunk_bit]
        [index / 32, index % 32]
    }
    fn set(&mut self, at: usize) {
        let [chunk_index, chunk_bit] = Self::index_decomposed(at);
        if chunk_index >= self.0.len() {
            self.0.resize(chunk_index + 1, 0u32);
        }
        let chunk = unsafe {
            // SAFE! previous line ensures sufficient size
            self.0.get_unchecked_mut(chunk_index)
        };
        *chunk |= 1 << chunk_bit;
    }
    fn unset(&mut self, at: usize) {
        let [chunk_index, chunk_bit] = Self::index_decomposed(at);
        if chunk_index < self.0.len() {
            let chunk = unsafe {
                // SAFE! previous line ensures sufficient size
                self.0.get_unchecked_mut(chunk_index)
            };
            *chunk &= !(1 << chunk_bit);
            while let Some(0u32) = self.0.iter().copied().last() {
                self.0.pop();
            }
        }
    }
}

#[derive(Debug, Default)]
struct BitMasks(HashMap<(ChannelId, bool), BitSet>);

struct BitChunkIter<I: Iterator<Item = u32>> {
    chunk_iter: I,
    next_bit_index: usize,
    cached: Option<u32>, // None <=> iterator is done
}

impl<I: Iterator<Item = u32>> BitChunkIter<I> {
    fn new(mut chunk_iter: I) -> Self {
        let cached = chunk_iter.next();
        Self { chunk_iter, next_bit_index: 0, cached }
    }
}
impl<I: Iterator<Item = u32>> Iterator for BitChunkIter<I> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            println!("LOOP");
            // get cached chunk. If none exists, iterator is done.
            let mut chunk = self.cached?;
            if chunk == 0 {
                // self.next_bit_index jumps to next multiple of 32
                self.next_bit_index = (self.next_bit_index + 32) & !(32 - 1);
                self.cached = self.chunk_iter.next();
                continue;
            }
            // this chunk encodes 1+ Items to yield
            // shift the contents of chunk until the least significant bit is 1

            #[inline(always)]
            fn shifty(chunk: &mut u32, shift_by: usize, next_bit_index: &mut usize) {
                if *chunk & ((1 << shift_by) - 1) == 0 {
                    *next_bit_index += shift_by;
                    *chunk >>= shift_by;
                }
                println!("{:#032b}", *chunk);
            }
            shifty(&mut chunk, 16, &mut self.next_bit_index);
            shifty(&mut chunk, 08, &mut self.next_bit_index);
            shifty(&mut chunk, 04, &mut self.next_bit_index);
            shifty(&mut chunk, 02, &mut self.next_bit_index);
            shifty(&mut chunk, 01, &mut self.next_bit_index);
            // assert(chunk & 1 == 1)

            self.next_bit_index += 1;
            self.cached = Some(chunk >> 1);
            if chunk > 0 {
                return Some(self.next_bit_index - 1);
            }
        }
    }
}

/// Returns an iterator over chunks of bits where ALL of the given
/// bitsets have 1.
struct AndChunkIter<'a> {
    // this value is not overwritten during iteration
    // invariant: !sets.is_empty()
    sets: &'a [&'a [u32]],

    next_chunk_index: usize,
}
impl<'a> AndChunkIter<'a> {
    fn new(sets: &'a [&'a [u32]]) -> Self {
        let sets = if sets.is_empty() { &[&[] as &[_]] } else { sets };
        Self { sets, next_chunk_index: 0 }
    }
}
impl Iterator for AndChunkIter<'_> {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        let old_chunk_index = self.next_chunk_index;
        self.next_chunk_index += 1;
        self.sets.iter().fold(Some(!0u32), move |a, b| {
            let a = a?;
            let b = *b.get(old_chunk_index)?;
            Some(a & b)
        })
    }
}

/// Returns an iterator over chunks for bits in range 0..bits_to_go but skipping
/// indices for which ANY of the given bitsets has a 1
struct NoneChunkIter<'a> {
    // this value is not overwritten during iteration
    // invariant: !sets.is_empty()
    sets: &'a [&'a [u32]],
    next_chunk_index: usize,
    bits_to_go: usize,
}
impl<'a> NoneChunkIter<'a> {
    /// a set of bitsets. the u32s of each are in ascending order of significant digits
    /// i.e., &[0,1] means {1<<32, 0} while &[0,1] is identical to &[1] and means {1}.
    fn new(sets: &'a [&'a [u32]], max_bit: usize) -> Self {
        let sets = if sets.is_empty() { &[&[] as &[_]] } else { sets };
        Self { sets, next_chunk_index: 0, bits_to_go: max_bit }
    }
}
impl Iterator for NoneChunkIter<'_> {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        let neutral = match self.bits_to_go {
            0 => None,
            x @ 1..=31 => Some(!0u32 >> (32 - x)),
            _ => Some(!0u32),
        };
        self.bits_to_go = self.bits_to_go.saturating_sub(32);

        let old_chunk_index = self.next_chunk_index;
        self.next_chunk_index += 1;

        self.sets.iter().fold(neutral, move |a, b| {
            let a = a?;
            let b = *b.get(old_chunk_index)?;
            Some(a & !b)
        })
    }
}

#[test]
fn test_bit_iter() {
    static SETS: &[&[u32]] = &[
        //
        &[0b101001, 0b101001],
        &[0b100001, 0b101001],
    ];
    let _ = BitChunkIter::new(AndChunkIter::new(SETS));
    let iter = BitChunkIter::new(NoneChunkIter::new(SETS, 9));
    let indices = iter.collect::<Vec<_>>();
    println!("indices {:?}", indices);
}

enum Entity {
    Payload(Payload),
    State(ProtocolS),
}

fn ecs() {
    let entities: Vec<Entity> = Default::default();
    // invariant: for all ChannelId c, assignments[(c, true)] & assignments[(c, false)] == 0;
    let assignments: HashMap<(ChannelId, bool), BitSet> = Default::default();
    // invariant: for all Keys k0 != k1, keys[k0] & keys[k1] == 0;
    let keys: HashMap<Key, BitSet> = Default::default();
    // invariant: for all Keys k, keys[k] & components == 0;
    let components: BitSet = Default::default();
    // invariant: to_run &!components = 0 i.e. to_run is a subset
    let to_run: BitSet = Default::default();

    // 1.
}

/*
needed operations:

1. insert a payload and overwrite / insert its predicate assignments
2. run all machines that
*/
