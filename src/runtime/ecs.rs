use crate::common::*;
use crate::runtime::ProtocolS;

use std::collections::HashMap;

/// invariant: last element is not zero.
/// => all values out of bounds are implicitly absent.
/// i.e., &[0,1] means {1<<32, 0} while &[0,1] is identical to &[1] and means {1}.

#[derive(Debug, Default)]
struct BitSet(Vec<u32>);
impl BitSet {
    fn as_slice(&self) -> &[u32] {
        self.0.as_slice()
    }
    fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().copied()
    }
    fn is_empty(&self) -> bool {
        // relies on the invariant: no trailing zero u32's
        self.0.is_empty()
    }
    fn clear(&mut self) {
        self.0.clear();
    }
    fn set_ones_until(&mut self, mut end: usize) {
        self.0.clear();
        loop {
            if end >= 32 {
                // full 32 bits of 1
                self.0.push(!0u32);
            } else {
                if end > 0 {
                    // #end ones, with a (32-end) prefix of zeroes
                    self.0.push(!0u32 >> (32 - end));
                }
                return;
            }
        }
    }
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

#[derive(Default)]
struct Ecs {
    entities: Vec<Entity>,
    assignments: HashMap<(ChannelId, bool), BitSet>,
    ekeys: HashMap<Key, BitSet>,
    csb: ComponentStatusBits,
}

#[derive(Default)]
struct ComponentStatusBits {
    inconsistent: BitSet,
    blocked: BitSet,
    sync_ended: BitSet,
    to_run_r: BitSet, // read from and drained while...
    to_run_w: BitSet, // .. written to and populated.
}
impl ComponentStatusBits {
    fn clear_all(&mut self) {
        self.blocked.clear();
        self.inconsistent.clear();
        self.to_run_r.clear();
        self.to_run_w.clear();
        self.sync_ended.clear();
    }
}
struct Msg {
    assignments: Vec<(ChannelId, bool)>, // invariant: no two elements have same ChannelId value
    payload: Payload,
}
impl Ecs {
    fn round(&mut self) {
        // 1. at the start of the round we throw away all assignments.
        //    we are going to shift entities around, so all bitsets need to be cleared anyway.
        self.assignments.clear();
        self.csb.clear_all();
        self.ekeys.clear();

        // 2. We discard all payloads; they are all stale now.
        //    All components are now contiguous in the vector.
        self.entities.retain(|entity| if let Entity::State(_) = entity { true } else { false });

        // 3. initially, all the components need a chance to run in MONO mode
        self.csb.to_run_r.set_ones_until(self.entities.len());

        // 4. INVARIANT established:
        //    for all State variants in self.entities,
        //        exactly one bit throughout the fields of csb is set.

        // 5. Run all machines in (csb.to_run_r U csb.to_run_w).
        //    Single, logical set is broken into readable / writable parts to allow concurrent reads / writes safely.
        while !self.csb.to_run_r.is_empty() {
            for _entity_index in self.csb.to_run_r.iter() {
                // TODO run and possbibly manipulate self.to_run_w
            }
            self.csb.to_run_r.clear();
            std::mem::swap(&mut self.csb.to_run_r, &mut self.csb.to_run_w);
        }
        assert!(self.csb.to_run_w.is_empty());

        #[allow(unreachable_code)] // DEBUG
        'recv_loop: loop {
            let ekey: Key = todo!();
            let msg: Msg = todo!();
            // 1. check if this message is redundant, i.e., there is already an equivalent payload with predicate >= this one.
            //    ie. starting from all payloads

            // 2. try and find a payload whose predicate subsumes this one
            if let Some(_entity_index) = self.ekeys.get(&ekey).map(|ekey_bitset| {
                let mut slice_builder = vec![];
                for &(channel_id, boolean) in msg.assignments.iter() {
                    if let Some(bitset) = self.assignments.get(&(channel_id, !boolean)) {
                        slice_builder.push(bitset.as_slice());
                    }
                }
                NoStricterPayloadIter {
                    next_chunk_index: 0,
                    in_here: ekey_bitset.as_slice(),
                    but_in_none_of: slice_builder.as_slice(),
                }
                .next()
            }) {
                continue 'recv_loop;
            }

            // receive incoming messages
        }
    }
}

struct NoStricterPayloadIter<'a> {
    next_chunk_index: usize,
    in_here: &'a [u32],
    but_in_none_of: &'a [&'a [u32]],
}
impl<'a> Iterator for NoStricterPayloadIter<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        let i = self.next_chunk_index;
        self.next_chunk_index += 1;
        let init = self.in_here.get(i).copied();
        self.but_in_none_of.iter().fold(init, |folding, slice| {
            let a = folding?;
            let b = slice.get(i).copied()?;
            Some(a & !b)
        })
    }
}

/*
The idea is we have a set of component machines that fork whenever they reflect on the oracle to make concrete their predicates.
their speculative execution procedure BLOCKS whenever they reflect on the contents of a message that has not yet arrived.
the promise is, therefore, never to forget about these blocked machines.
the only event that unblocks a machine

operations needed:
1. FORK
given a component and a predicate,
create and retain a clone of the component, and the predicate, with one additional assignment

2. GET
when running a machine with {state S, predicate P}, it may try to get a message at K.
IF there exists a payload at K with predicate P2 s.t. P2 >= P, feed S the message and continue.
ELSE list (S,P,K) as BLOCKED and...
for all payloads X at K with predicate P2 s.t. P2 < P, fork S to create S2. store it with predicate P2 and feed it X and continue

2. RECV
when receiving a payload at key K with predicate P,
IF there exists a payload at K with predicate P2 where P2 >= P, discard the new one and continue.
ELSE if there exists a payload at K with predicate P2 where P2 < P, assert their contents are identical, overwrite P2 with P try feed this payload to any BLOCKED machines
ELSE insert this payload with P and K as a new payload, and feed it to any compatible machines blocked on K



====================
EXTREME approach:
1. entities: {states} U {payloads}
2. flags: {}

==================
*/
