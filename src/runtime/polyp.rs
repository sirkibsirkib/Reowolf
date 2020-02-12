use crate::common::*;
use crate::runtime::{Predicate, ProtocolS};
use core::ops::{Index, IndexMut};
use std::collections::HashMap;
use std::num::NonZeroU32;

// struct SpeculationBranch {
//     inbox: HashMap<Key, Payload>,
//     outbox: HashMap<Key, Payload>,
//     inner: SpeculationBranchInner,
//     known: HashMap<ChannelId, bool>,
// }
// enum SpeculationBranchInner {
//     Leaf(ProtocolS),

//     // invariant: channel_id branching is redundantly represented by true_false branches' known assignments
//     // => true_false[0].known[channel_id] == Some(true)
//     // => true_false[1].known[channel_id] == Some(false)
//     Fork { channel_id: ChannelId, true_false: Box<[SpeculationBranch; 2]> },
// }

// impl SpeculationBranch {
//     fn new_tree(init: ProtocolS) -> Self {
//         SpeculationBranch {
//             inbox: Default::default(),
//             outbox: Default::default(),
//             known: Default::default(),
//             inner: SpeculationBranchInner::Leaf(init),
//         }
//     }

//     fn feed_msg(
//         &mut self,
//         ekey: Key,
//         payload: &Payload,
//         predicate: &Predicate,
//         pred: Option<Pred>,
//     ) {
//         use SpeculationBranchInner as Sbi;
//         let next_pred = Some(Pred { known: &self.known, prev: pred.as_ref() });
//         match &mut self.inner {
//             Sbi::Leaf(_state) => {
//                 if self.inbox.insert(ekey, payload.clone()).is_none() {
//                     // run this machine
//                 }
//             }
//             Sbi::Fork { channel_id, true_false } => match predicate.query(*channel_id) {
//                 Some(true) => true_false[0].feed_msg(ekey, payload, predicate, next_pred), // feed true
//                 Some(false) => true_false[1].feed_msg(ekey, payload, predicate, next_pred), // feed false
//                 None => {
//                     // feed to both true and false branches
//                     for x in true_false.iter_mut() {
//                         x.feed_msg(ekey, payload, predicate, next_pred);
//                     }
//                 }
//             },
//         }
//     }
// }

// #[derive(Copy, Clone)]
// struct Pred<'a> {
//     known: &'a HashMap<ChannelId, bool>,
//     prev: Option<&'a Pred<'a>>,
// }

struct Branch {
    state: Option<StateKey>,
    speculation: Option<Speculation>,
}
struct Speculation {
    on: ChannelId,
    t: Option<BranchKey>,
    f: Option<BranchKey>,
}

struct Tree {
    branches: Vec<Branch>, // invariant: non-empty. root at index 0
    states: Vec<ProtocolS>,
}
impl Tree {
    /// determine where in the tree the given message should be inserted (based on the predicate).
    /// run all machines
    fn feed_and_run(&mut self, predicate: Predicate, payload: &Payload) {
        let q = Queryable::new(&predicate);
        let mut qs = QueryableSubset::new(&q);
        self.branches[0].feed_and_run(payload, &q, &mut qs);
    }
}

struct Queryable(Vec<(ChannelId, bool)>);
impl Queryable {
    fn new(predicate: &Predicate) -> Self {
        let mut vec: Vec<_> = predicate.assigned.iter().map(|(&k, &v)| (k, v)).collect();
        vec.sort_by(|(a, _), (b, _)| a.cmp(b));
        Self(vec)
    }
    fn query(&self, channel_id: ChannelId) -> Option<(usize, bool)> {
        self.0
            .binary_search_by(|(cid, _)| cid.cmp(&channel_id))
            .ok()
            .map(|index| (index, self.0[index].1))
    }
}
struct QueryableSubset {
    buf: Vec<usize>,
    prefix_end: usize,
}
impl QueryableSubset {
    fn new(q: &Queryable) -> Self {
        let prefix_end = q.0.len();
        Self { buf: (0..prefix_end).collect(), prefix_end }
    }
    fn remove(&mut self, at: usize) {
        self.prefix_end -= 1;
        self.buf.swap(self.prefix_end, at);
    }
    fn undo_remove(&mut self, at: usize) {
        self.buf.swap(self.prefix_end, at);
        self.prefix_end += 1;
    }
    fn iter_q<'a: 'b, 'b>(
        &'a self,
        q: &'b Queryable,
    ) -> impl Iterator<Item = &'b (ChannelId, bool)> {
        self.buf[..self.prefix_end].iter().map(move |&index| &q.0[index])
    }
}

impl Branch {
    // invariant: q.0 is sorted
    //
    // invariant: qs.buf[0..qs.prefix_end] is a slice that encodes the set of INDICES in q.0
    // which the path to this branch has NOT queried.
    //
    // ie. for a given predicate {X=>true, Z=>true, Y=>false}
    // => q is [(X,true), (Y,false), (Z,true)]
    // => qs is initially [0,1,2]
    // and if this branch queries 1, the subtree will receive qs as [0,2]
    fn feed_and_run(&mut self, payload: &Payload, q: &Queryable, qs: &mut QueryableSubset) {
        match &mut self.speculation {
            Some(Speculation { on, t, f }) => {
                if let Some((index, assignment)) = q.query(*on) {
                    // if assignment
                } else {
                }
                todo!()
            }
            None => {
                //
                todo!()
            }
        }
    }
}
impl Index<BranchKey> for Tree {
    type Output = Branch;
    fn index(&self, k: BranchKey) -> &Self::Output {
        &self.branches[(k.index_plus_one.get() - 1) as usize]
    }
}
impl IndexMut<BranchKey> for Tree {
    fn index_mut(&mut self, k: BranchKey) -> &mut Self::Output {
        &mut self.branches[(k.index_plus_one.get() - 1) as usize]
    }
}
impl Index<StateKey> for Tree {
    type Output = ProtocolS;
    fn index(&self, k: StateKey) -> &Self::Output {
        &self.states[(k.index_plus_one.get() - 1) as usize]
    }
}
impl IndexMut<StateKey> for Tree {
    fn index_mut(&mut self, k: StateKey) -> &mut Self::Output {
        &mut self.states[(k.index_plus_one.get() - 1) as usize]
    }
}

struct BranchKey {
    index_plus_one: NonZeroU32,
}
struct StateKey {
    index_plus_one: NonZeroU32,
}

struct Bitset {
    bits: Vec<u32>,
}

struct Polyp {
    inbox: Vec<Payload>,
    inbox_masks: BitMasks,
    states: Vec<ProtocolS>,
    states_masks: BitMasks,
}

// invariant: last element is not zero.
// => all values out of bounds are implicitly absent
#[derive(Debug, Default)]
struct BitSet(Vec<u32>);

#[derive(Debug, Default)]
struct BitMasks(HashMap<(ChannelId, bool), BitSet>);

struct BitSetAndIter<'a> {
    // this value is immutable
    // invariant: !sets.is_empty()
    sets: &'a [&'a [u32]],
    next_u32_index: usize, // invariant: in 0..32 while iterating
    next_bit_index: usize,
    cached: Option<u32>, // None <=> iterator is done
}
impl<'a> BitSetAndIter<'a> {
    fn new(sets: &'a [&'a [u32]]) -> Self {
        const EMPTY_SINGLETON: &[&[u32]] = &[&[]];
        let sets = if sets.is_empty() { EMPTY_SINGLETON } else { sets };
        Self { sets, next_u32_index: 0, next_bit_index: 0, cached: Self::nth_u32(sets, 0) }
    }
    fn nth_u32(sets: &'a [&'a [u32]], index: usize) -> Option<u32> {
        sets.iter().fold(Some(!0), |a, b| {
            let b = b.get(index)?;
            Some(a? & b)
        })
    }
    fn next_chunk(&mut self) {
        self.next_bit_index = 0;
        self.next_u32_index += 1;
        self.cached = Self::nth_u32(self.sets, self.next_u32_index);
    }
}
impl Iterator for BitSetAndIter<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // get cached chunk. If none exists, iterator is done.
            let mut chunk = self.cached?;
            if chunk == 0 {
                self.next_chunk();
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
            }
            shifty(&mut chunk, 16, &mut self.next_bit_index);
            shifty(&mut chunk, 08, &mut self.next_bit_index);
            shifty(&mut chunk, 04, &mut self.next_bit_index);
            shifty(&mut chunk, 02, &mut self.next_bit_index);
            shifty(&mut chunk, 01, &mut self.next_bit_index);
            // assert(chunk & 1 == 1)
            let index = self.next_u32_index * 32 + self.next_bit_index;
            self.next_bit_index += 1;
            self.cached = Some(chunk >> 1);
            if chunk > 0 {
                // assert(self.next_bit_index <= 32)
                // because index was calculated with self.next_bit_index - 1
                return Some(index);
            }
        }
    }
}

#[test]
fn test_bit_iter() {
    static SETS: &[&[u32]] = &[
        //
        &[0b100011000000100101101],
        &[0b100001000000000110100],
        &[0b100001000100010100110],
    ];
    let indices = BitSetAndIter::new(SETS).collect::<Vec<_>>();
    println!("indices {:?}", indices);
}
