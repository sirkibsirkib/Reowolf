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
    fn test(&self, at: usize) -> bool {
        let [chunk_index, chunk_bit] = Self::index_decomposed(at);
        match self.0.get(chunk_index) {
            None => false,
            Some(&chunk) => (chunk & (1 << chunk_bit)) != 0,
        }
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
    Machine { state: ProtocolS, component_index: usize },
}

/// Invariant: every component is either:
///        in to_run = (to_run_r U to_run_w)
///     or in ONE of the ekeys (which means it is blocked by a get on that ekey)
///     or in sync_ended (because they reached the end of their sync block)
///     or in inconsistent (because they are inconsistent)
#[derive(Default)]
struct Ecs {
    component_info: Vec<(Arc<Protocol>, HashSet<ChannelId>)>,
    entities: Vec<Entity>,
    round_solution: Vec<(ChannelId, bool)>, // encodes an ASSIGNMENT
    ekey_channel_ids: Vec<ChannelId>,       // all channel Ids for local keys
    flags: EntityFlags,
    ekey_to_channel_id: HashMap<Key, ChannelId>,
}
#[derive(Default)]
struct EntityFlags {
    assignments: HashMap<(ChannelId, bool), BitSet>,
    payloads: BitSet,
    ekeys: HashMap<Key, BitSet>,
    inconsistent: BitSet,
    sync_ended: BitSet,
    to_run_r: BitSet, // read from and drained while...
    to_run_w: BitSet, // .. written to and populated. }
}
impl Debug for Ecs {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let elen = self.entities.len();

        write!(f, "{:<30}", "payloads")?;
        print_flag_bits(f, &self.flags.payloads, elen)?;

        write!(f, "{:<30}", "inconsistent")?;
        print_flag_bits(f, &self.flags.inconsistent, elen)?;
        write!(f, "{:<30}", "sync_ended")?;
        print_flag_bits(f, &self.flags.sync_ended, elen)?;
        write!(f, "{:<30}", "to_run_r")?;
        print_flag_bits(f, &self.flags.to_run_r, elen)?;
        write!(f, "{:<30}", "to_run_w")?;
        print_flag_bits(f, &self.flags.to_run_w, elen)?;

        for (assignment, bitset) in self.flags.assignments.iter() {
            write!(f, "{:<30?}", assignment)?;
            print_flag_bits(f, bitset, elen)?;
        }
        for (ekey, bitset) in self.flags.ekeys.iter() {
            write!(f, "Ekey {:<30?}", ekey)?;
            print_flag_bits(f, bitset, elen)?;
        }
        Ok(())
    }
}
fn print_flag_bits(
    f: &mut std::fmt::Formatter,
    bitset: &BitSet,
    ecs_keys_end: usize,
) -> std::fmt::Result {
    for i in 0..ecs_keys_end {
        f.pad(match bitset.test(i) {
            true => "1",
            false => "0",
        })?;
    }
    write!(f, "\n");
    Ok(())
}

struct Protocol {
    // TODO
}

struct Msg {
    assignments: Vec<(ChannelId, bool)>, // invariant: no two elements have same ChannelId value
    payload: Payload,
}

#[test]
fn ecs_test() {
    let mut ecs = Ecs::default();
    println!("{:?}", &ecs);
}
impl Ecs {
    fn round(&mut self) {
        // 1. at the start of the round we throw away all assignments.
        //    we are going to shift entities around, so all bitsets need to be cleared anyway.
        self.flags.assignments.clear();
        self.flags.payloads.clear();
        self.flags.ekeys.clear();
        self.flags.inconsistent.clear();
        self.flags.to_run_r.clear();
        self.flags.to_run_w.clear();
        self.flags.sync_ended.clear();

        // 2. We discard all payloads; they are all stale now.
        //    All machines are contiguous in the vector
        self.entities
            .retain(move |entity| if let Entity::Machine { .. } = entity { true } else { false });

        // 3. initially, all the components need a chance to run in MONO mode
        self.flags.to_run_r.set_ones_until(self.entities.len());

        // 4. INVARIANT established:
        //    for all State variants in self.entities,
        //        exactly one bit throughout the fields of csb is set.

        // 5. Run all machines in (csb.to_run_r U csb.to_run_w).
        //    Single, logical set is broken into readable / writable parts to allow concurrent reads / writes safely.
        while !self.flags.to_run_r.is_empty() {
            for _eid in self.flags.to_run_r.iter() {
                // TODO run and possbibly manipulate self.to_run_w
            }
            self.flags.to_run_r.clear();
            std::mem::swap(&mut self.flags.to_run_r, &mut self.flags.to_run_w);
        }
        assert!(self.flags.to_run_w.is_empty());

        #[allow(unreachable_code)] // DEBUG
        'recv_loop: loop {
            let ekey: Key = todo!();
            let msg: Msg = todo!();
            // 1. check if this message is redundant, i.e., there is already an equivalent payload with predicate >= this one.
            //    ie. starting from all payloads

            // 2. try and find a payload whose predicate is the same or more general than this one
            //    if it exists, drop the message; it is uninteresting.
            let ekey_bitset = self.flags.ekeys.get(&ekey);
            if let Some(_eid) = ekey_bitset.map(move |ekey_bitset| {
                let mut slice_builder = vec![];
                // collect CONFLICTING assignments into slice_builder
                for &(channel_id, boolean) in msg.assignments.iter() {
                    if let Some(bitset) = self.flags.assignments.get(&(channel_id, !boolean)) {
                        slice_builder.push(bitset.as_slice());
                    }
                }
                let chunk_iter =
                    InNoneExceptIter::new(slice_builder.as_slice(), ekey_bitset.as_slice());
                BitChunkIter::new(chunk_iter).next()
            }) {
                // _eid is a payload whose predicate is at least as general
                // drop this message!
                continue 'recv_loop;
            }

            // 3. insert this payload as an entity, overwriting an existing LESS GENERAL payload if it exists.
            let payload_eid: usize = if let Some(eid) = ekey_bitset.and_then(move |ekey_bitset| {
                let mut slice_builder = vec![];
                slice_builder.push(ekey_bitset.as_slice());
                for assignment in msg.assignments.iter() {
                    if let Some(bitset) = self.flags.assignments.get(assignment) {
                        slice_builder.push(bitset.as_slice());
                    }
                }
                let chunk_iter = AndChunkIter::new(slice_builder.as_slice());
                BitChunkIter::new(chunk_iter).next()
            }) {
                // overwrite this entity index.
                eid
            } else {
                // nothing to overwrite. add a new payload entity.
                let eid = self.entities.len();
                self.entities.push(Entity::Payload(msg.payload));
                for &assignment in msg.assignments.iter() {
                    let mut bitset = self.flags.assignments.entry(assignment).or_default();
                    bitset.set(eid);
                }
                self.flags.payloads.set(eid);
                eid
            };

            self.feed_msg(payload_eid, ekey);
            // TODO run all in self.flags.to_run_w
        }
    }

    fn run_poly_p(&mut self, machine_eid: usize) {
        match self.entities.get_mut(machine_eid) {
            Some(Entity::Machine { component_index, state }) => {
                // TODO run the machine
                use PolyBlocker as Pb;
                let blocker: Pb = todo!();
                match blocker {
                    Pb::Inconsistent => self.flags.inconsistent.set(machine_eid),
                    Pb::CouldntCheckFiring(key) => {
                        // 1. clone the machine
                        let state_true = state.clone();
                        let machine_eid_true = self.entities.len();
                        self.entities.push(Entity::Machine {
                            state: state_true,
                            component_index: *component_index,
                        });
                        // 2. copy the assignments of the existing machine to the new one
                        for bitset in self.flags.assignments.values() {
                            if bitset.test(machine_eid) {
                                bitset.set(machine_eid_true);
                            }
                        }
                        // 3. give the old machine FALSE and the new machine TRUE
                        let &channel_id = self.ekey_to_channel_id.get(&key).unwrap();
                        self.flags
                            .assignments
                            .entry((channel_id, false))
                            .or_default()
                            .set(machine_eid);
                        self.flags
                            .assignments
                            .entry((channel_id, true))
                            .or_default()
                            .set(machine_eid_true);
                        self.run_poly_p(machine_eid);
                        self.run_poly_p(machine_eid_true);
                    }
                    _ => todo!(),
                }

                // 1. make the assignment of this machine concrete WRT its ports
                let component_info = self.component_info.get(*component_index).unwrap();
                for &channel_id in component_info.1.iter() {
                    let test = self
                        .flags
                        .assignments
                        .get(&(channel_id, true))
                        .map(move |bitset| bitset.test(machine_eid))
                        .unwrap_or(false);
                    if !test {
                        // TRUE assignment wasn't set
                        // so set FALSE assignment (no effect if already set)
                        self.flags
                            .assignments
                            .entry((channel_id, false))
                            .or_default()
                            .set(machine_eid);
                    }
                }
                // 2. this machine becomes solved
                self.flags.sync_ended.set(machine_eid);
                self.generate_new_solutions(machine_eid);
                // TODO run this machine to a poly blocker
                // potentially mark as inconsistent, blocked on some key, or solved
                // if solved
            }
            _ => unreachable!(),
        }
    }

    fn generate_new_solutions(&mut self, newly_solved_machine_eid: usize) {
        // this vector will be used to store assignments from self.ekey_channel_ids to elements in {true, false}
        let mut solution_prefix = vec![];
        self.generate_new_solutions_rec(newly_solved_machine_eid, &mut solution_prefix);
        // let all_channel_ids =
        // let mut slice_builder = vec![];
    }
    fn generate_new_solutions_rec(
        &mut self,
        newly_solved_machine_eid: usize,
        solution_prefix: &mut Vec<bool>,
    ) {
        let eid = newly_solved_machine_eid;
        let n = solution_prefix.len();
        if let Some(&channel_id) = self.ekey_channel_ids.get(n) {
            if let Some(assignment) = self.machine_assignment_for(eid, channel_id) {
                // this machine already gives an assignment
                solution_prefix.push(assignment);
                self.generate_new_solutions_rec(eid, solution_prefix);
            } else {
                // this machine does not give an assignment. try both branches!
                solution_prefix.push(false);
                self.generate_new_solutions_rec(eid, solution_prefix);
                solution_prefix.pop();
                solution_prefix.push(true);
                self.generate_new_solutions_rec(eid, solution_prefix);
            }
            solution_prefix.pop();
        } else {
            println!("SOLUTION:");
            for (channel_id, assignment) in self.ekey_channel_ids.iter().zip(solution_prefix.iter())
            {
                println!("{:?} => {:?}", channel_id, assignment);
            }
            // SOLUTION COMPLETE!
            return;
        }
    }

    fn machine_assignment_for(&self, machine_eid: usize, channel_id: ChannelId) -> Option<bool> {
        let test = move |bitset: &BitSet| bitset.test(machine_eid);
        self.flags
            .assignments
            .get(&(channel_id, true))
            .map(test)
            .or_else(move || self.flags.assignments.get(&(channel_id, false)).map(test))
    }

    fn feed_msg(&mut self, payload_eid: usize, ekey: Key) {
        // 1. identify the component who:
        //    * is blocked on this ekey,
        //    * and has a predicate at least as strict as that of this payload
        let mut slice_builder = vec![];
        let ekey_bitset =
            self.flags.ekeys.get_mut(&ekey).expect("Payload sets this => cannot be empty");
        slice_builder.push(ekey_bitset.as_slice());
        for bitset in self.flags.assignments.values() {
            // it doesn't matter which assignment! just that this payload sets it too
            if bitset.test(payload_eid) {
                slice_builder.push(bitset.as_slice());
            }
        }
        let chunk_iter =
            InAllExceptIter::new(slice_builder.as_slice(), self.flags.payloads.as_slice());
        let mut iter = BitChunkIter::new(chunk_iter);
        if let Some(machine_eid) = iter.next() {
            // TODO is it possible for there to be 2+ iterations? I'm thinking No
            // RUN THIS MACHINE
            ekey_bitset.unset(machine_eid);
            self.flags.to_run_w.set(machine_eid);
        }
    }
}

struct InAllExceptIter<'a> {
    next_chunk_index: usize,
    in_all: &'a [&'a [u32]],
    except: &'a [u32],
}
impl<'a> InAllExceptIter<'a> {
    fn new(in_all: &'a [&'a [u32]], except: &'a [u32]) -> Self {
        Self { in_all, except, next_chunk_index: 0 }
    }
}
impl<'a> Iterator for InAllExceptIter<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        let i = self.next_chunk_index;
        self.next_chunk_index += 1;
        let init = self.except.get(i).map(move |&x| !x).or(Some(1));
        self.in_all.iter().fold(init, move |folding, slice| {
            let a = folding?;
            let b = slice.get(i).copied().unwrap_or(0);
            Some(a & !b)
        })
    }
}

struct InNoneExceptIter<'a> {
    next_chunk_index: usize,
    in_none: &'a [&'a [u32]],
    except: &'a [u32],
}
impl<'a> InNoneExceptIter<'a> {
    fn new(in_none: &'a [&'a [u32]], except: &'a [u32]) -> Self {
        Self { in_none, except, next_chunk_index: 0 }
    }
}
impl<'a> Iterator for InNoneExceptIter<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        let i = self.next_chunk_index;
        self.next_chunk_index += 1;
        let init = self.except.get(i).copied()?;
        Some(self.in_none.iter().fold(init, move |folding, slice| {
            let a = folding;
            let b = slice.get(i).copied().unwrap_or(0);
            a & !b
        }))
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
