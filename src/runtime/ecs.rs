use crate::common::*;
use crate::runtime::endpoint::EndpointExt;
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

/// Converts an iterator over contiguous u32 chunks into an iterator over usize
/// e.g. input [0b111000, 0b11] gives output [3, 4, 5, 32, 33]
/// observe that the bits per chunk are ordered from least to most significant bits, yielding smaller to larger usizes.
/// works by draining the inner u32 chunk iterator one u32 at a time, then draining that chunk until its 0.
struct BitChunkIter<I: Iterator<Item = u32>> {
    chunk_iter: I,
    next_bit_index: usize,
    cached: u32,
}

impl<I: Iterator<Item = u32>> BitChunkIter<I> {
    fn new(chunk_iter: I) -> Self {
        // first chunk is always a dummy zero, as if chunk_iter yielded Some(0).
        // Consequences:
        // 1. our next_bit_index is always off by 32 (we correct for it in Self::next) (no additional overhead)
        // 2. we cache u32 and not Option<u32>, because chunk_iter.next() is only called in Self::next.
        Self { chunk_iter, next_bit_index: 0, cached: 0 }
    }
}
impl<I: Iterator<Item = u32>> Iterator for BitChunkIter<I> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = self.cached;

        // loop until either:
        // 1. there are no more Items to return, or
        // 2. chunk encodes 1+ Items, one of which we will return.
        while chunk == 0 {
            // chunk is still empty! get the next one...
            chunk = self.chunk_iter.next()?;

            // ... and jump self.next_bit_index to the next multiple of 32.
            self.next_bit_index = (self.next_bit_index + 32) & !(32 - 1);
        }
        // assert(chunk > 0);

        // Shift the contents of chunk until the least significant bit is 1.
        // ... being sure to increment next_bit_index accordingly.
        #[inline(always)]
        fn skip_n_zeroes(chunk: &mut u32, n: usize, next_bit_index: &mut usize) {
            if *chunk & ((1 << n) - 1) == 0 {
                // n least significant bits are zero. skip n bits.
                *next_bit_index += n;
                *chunk >>= n;
            }
        }
        skip_n_zeroes(&mut chunk, 16, &mut self.next_bit_index);
        skip_n_zeroes(&mut chunk, 08, &mut self.next_bit_index);
        skip_n_zeroes(&mut chunk, 04, &mut self.next_bit_index);
        skip_n_zeroes(&mut chunk, 02, &mut self.next_bit_index);
        skip_n_zeroes(&mut chunk, 01, &mut self.next_bit_index);
        // least significant bit of chunk is 1.
        // assert(chunk & 1 == 1)

        // prepare our state for the next time Self::next is called.
        // Overwrite self.cached such that its shifted state is retained,
        // and jump over the bit whose index we are about to return.
        self.next_bit_index += 1;
        self.cached = chunk >> 1;

        // returned index is 32 smaller than self.next_bit_index because we use an
        // off-by-32 encoding to avoid having to cache an Option<u32>.
        Some(self.next_bit_index - 1 - 32)
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

#[test]
fn test_bit_iter() {
    static SETS: &[&[u32]] = &[
        //
        &[0b101001, 0b101001],
        &[0b100001, 0b101001],
    ];
    let iter = BitChunkIter::new(AndChunkIter::new(SETS));
    let indices = iter.collect::<Vec<_>>();
    println!("indices {:?}", indices);
}

enum Entity {
    Payload(Payload),
    Machine { state: ProtocolS, component_index: usize },
}

struct PortKey(usize);
struct EntiKey(usize);
struct CompKey(usize);

struct ComponentInfo {
    port_keyset: HashSet<PortKey>,
    protocol: Arc<Protocol>,
}
#[derive(Default)]
struct Connection {
    ecs: Ecs,
    round_solution: Vec<(ChannelId, bool)>, // encodes an ASSIGNMENT
    ekey_channel_ids: Vec<ChannelId>,       // all channel Ids for local keys
    component_info: Vec<ComponentInfo>,
    endpoint_exts: Vec<EndpointExt>,
}

/// Invariant: every component is either:
///        in to_run = (to_run_r U to_run_w)
///     or in ONE of the ekeys (which means it is blocked by a get on that ekey)
///     or in sync_ended (because they reached the end of their sync block)
///     or in inconsistent (because they are inconsistent)
#[derive(Default)]
struct Ecs {
    entities: Vec<Entity>, // machines + payloads
    assignments: HashMap<(ChannelId, bool), BitSet>,
    payloads: BitSet,
    ekeys: HashMap<usize, BitSet>,
    inconsistent: BitSet,
    sync_ended: BitSet,
    to_run_r: BitSet, // read from and drained while...
    to_run_w: BitSet, // .. written to and populated. }
}
impl Debug for Ecs {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let elen = self.entities.len();

        write!(f, "{:<30}", "payloads")?;
        print_flag_bits(f, &self.payloads, elen)?;

        write!(f, "{:<30}", "inconsistent")?;
        print_flag_bits(f, &self.inconsistent, elen)?;
        write!(f, "{:<30}", "sync_ended")?;
        print_flag_bits(f, &self.sync_ended, elen)?;
        write!(f, "{:<30}", "to_run_r")?;
        print_flag_bits(f, &self.to_run_r, elen)?;
        write!(f, "{:<30}", "to_run_w")?;
        print_flag_bits(f, &self.to_run_w, elen)?;

        for (assignment, bitset) in self.assignments.iter() {
            write!(f, "{:<30?}", assignment)?;
            print_flag_bits(f, bitset, elen)?;
        }
        for (ekey, bitset) in self.ekeys.iter() {
            write!(f, "Ekey {:<30?}", ekey)?;
            print_flag_bits(f, bitset, elen)?;
        }
        Ok(())
    }
}
fn print_flag_bits(f: &mut Formatter, bitset: &BitSet, elen: usize) -> std::fmt::Result {
    for i in 0..elen {
        f.pad(match bitset.test(i) {
            true => "1",
            false => "0",
        })?;
    }
    write!(f, "\n")
}

struct Protocol {
    // TODO
}

struct Msg {
    assignments: Vec<(ChannelId, bool)>, // invariant: no two elements have same ChannelId value
    payload: Payload,
}

impl Connection {
    fn new_channel(&mut self) -> [PortKey; 2] {
        todo!()
    }
    fn round(&mut self) {
        // 1. at the start of the round we throw away all assignments.
        //    we are going to shift entities around, so all bitsets need to be cleared anyway.
        self.ecs.assignments.clear();
        self.ecs.payloads.clear();
        self.ecs.ekeys.clear();
        self.ecs.inconsistent.clear();
        self.ecs.to_run_r.clear();
        self.ecs.to_run_w.clear();
        self.ecs.sync_ended.clear();

        // 2. We discard all payloads; they are all stale now.
        //    All machines are contiguous in the vector
        self.ecs
            .entities
            .retain(move |entity| if let Entity::Machine { .. } = entity { true } else { false });

        // 3. initially, all the components need a chance to run in MONO mode
        self.ecs.to_run_r.set_ones_until(self.ecs.entities.len());

        // 4. INVARIANT established:
        //    for all State variants in self.entities,
        //        exactly one bit throughout the fields of csb is set.

        // 5. Run all machines in (csb.to_run_r U csb.to_run_w).
        //    Single, logical set is broken into readable / writable parts to allow concurrent reads / writes safely.
        while !self.ecs.to_run_r.is_empty() {
            for _eid in self.ecs.to_run_r.iter() {
                // TODO run and possbibly manipulate self.to_run_w
            }
            self.ecs.to_run_r.clear();
            std::mem::swap(&mut self.ecs.to_run_r, &mut self.ecs.to_run_w);
        }
        assert!(self.ecs.to_run_w.is_empty());

        #[allow(unreachable_code)] // DEBUG
        'recv_loop: loop {
            let ekey: usize = todo!();
            let msg: Msg = todo!();
            // 1. check if this message is redundant, i.e., there is already an equivalent payload with predicate >= this one.
            //    ie. starting from all payloads

            // 2. try and find a payload whose predicate is the same or more general than this one
            //    if it exists, drop the message; it is uninteresting.
            let ekey_bitset = self.ecs.ekeys.get(&ekey);
            if let Some(_eid) = ekey_bitset.map(move |ekey_bitset| {
                let mut slice_builder = vec![];
                // collect CONFLICTING assignments into slice_builder
                for &(channel_id, boolean) in msg.assignments.iter() {
                    if let Some(bitset) = self.ecs.assignments.get(&(channel_id, !boolean)) {
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
                    if let Some(bitset) = self.ecs.assignments.get(assignment) {
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
                let eid = self.ecs.entities.len();
                self.ecs.entities.push(Entity::Payload(msg.payload));
                for &assignment in msg.assignments.iter() {
                    let mut bitset = self.ecs.assignments.entry(assignment).or_default();
                    bitset.set(eid);
                }
                self.ecs.payloads.set(eid);
                eid
            };

            self.feed_msg(payload_eid, ekey);
            // TODO run all in self.ecs.to_run_w
        }
    }

    fn run_poly_p(&mut self, machine_eid: usize) {
        match self.ecs.entities.get_mut(machine_eid) {
            Some(Entity::Machine { component_index, state }) => {
                // TODO run the machine
                use PolyBlocker as Pb;
                let blocker: Pb = todo!();
                match blocker {
                    Pb::Inconsistent => self.ecs.inconsistent.set(machine_eid),
                    Pb::CouldntCheckFiring(key) => {
                        // 1. clone the machine
                        let state_true = state.clone();
                        let machine_eid_true = self.ecs.entities.len();
                        self.ecs.entities.push(Entity::Machine {
                            state: state_true,
                            component_index: *component_index,
                        });
                        // 2. copy the assignments of the existing machine to the new one
                        for bitset in self.ecs.assignments.values() {
                            if bitset.test(machine_eid) {
                                bitset.set(machine_eid_true);
                            }
                        }
                        // 3. give the old machine FALSE and the new machine TRUE
                        let channel_id =
                            self.endpoint_exts.get(key.to_raw() as usize).unwrap().info.channel_id;
                        self.ecs
                            .assignments
                            .entry((channel_id, false))
                            .or_default()
                            .set(machine_eid);
                        self.ecs
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
                for &ekey in component_info.port_keyset.iter() {
                    let channel_id = self.endpoint_exts.get(ekey.0).unwrap().info.channel_id;
                    let test = self
                        .ecs
                        .assignments
                        .get(&(channel_id, true))
                        .map(move |bitset| bitset.test(machine_eid))
                        .unwrap_or(false);
                    if !test {
                        // TRUE assignment wasn't set
                        // so set FALSE assignment (no effect if already set)
                        self.ecs
                            .assignments
                            .entry((channel_id, false))
                            .or_default()
                            .set(machine_eid);
                    }
                }
                // 2. this machine becomes solved
                self.ecs.sync_ended.set(machine_eid);
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
                solution_prefix.pop();
            } else {
                // this machine does not give an assignment. try both branches!
                solution_prefix.push(false);
                self.generate_new_solutions_rec(eid, solution_prefix);
                solution_prefix.pop();
                solution_prefix.push(true);
                self.generate_new_solutions_rec(eid, solution_prefix);
                solution_prefix.pop();
            }
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
        self.ecs
            .assignments
            .get(&(channel_id, true))
            .map(test)
            .or_else(move || self.ecs.assignments.get(&(channel_id, false)).map(test))
    }

    fn feed_msg(&mut self, payload_eid: usize, ekey: usize) {
        // 1. identify the component who:
        //    * is blocked on this ekey,
        //    * and has a predicate at least as strict as that of this payload
        let mut slice_builder = vec![];
        let ekey_bitset =
            self.ecs.ekeys.get_mut(&ekey).expect("Payload sets this => cannot be empty");
        slice_builder.push(ekey_bitset.as_slice());
        for bitset in self.ecs.assignments.values() {
            // it doesn't matter which assignment! just that this payload sets it too
            if bitset.test(payload_eid) {
                slice_builder.push(bitset.as_slice());
            }
        }
        let chunk_iter =
            InAllExceptIter::new(slice_builder.as_slice(), self.ecs.payloads.as_slice());
        let mut iter = BitChunkIter::new(chunk_iter);
        if let Some(machine_eid) = iter.next() {
            // TODO is it possible for there to be 2+ iterations? I'm thinking No
            // RUN THIS MACHINE
            ekey_bitset.unset(machine_eid);
            self.ecs.to_run_w.set(machine_eid);
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
2. ecs: {}

==================
*/

impl Debug for FlagMatrix {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        for r in 0..self.dims[0] {
            write!(f, "|")?;
            for c in 0..self.dims[1] {
                write!(
                    f,
                    "{}",
                    match self.test([r, c]) {
                        false => '0',
                        true => '1',
                    }
                )?;
            }
            write!(f, "|\n")?;
        }
        Ok(())
    }
}

// invariant: all bits outside of 0..columns and 0..rows BUT in the allocated space are ZERO
struct FlagMatrix {
    bytes: *mut u32,
    u32s_total: usize,
    u32s_per_row: usize,
    dims: [usize; 2],
}
#[inline(always)]
fn ceiling_to_mul_32(value: usize) -> usize {
    (value + 31) & !31
}
impl Drop for FlagMatrix {
    fn drop(&mut self) {
        let layout = Self::layout_for(self.u32s_total);
        unsafe {
            // ?
            std::alloc::dealloc(self.bytes as *mut u8, layout);
        }
    }
}
impl FlagMatrix {
    fn get_dims(&self) -> &[usize; 2] {
        &self.dims
    }

    fn set_entire_row(&mut self, row: usize) {
        assert!(row < self.dims[0]);
        let mut cols_left = self.dims[1];
        unsafe {
            let mut ptr = self.bytes.add(self.offset_of_chunk_unchecked([row, 0]));
            while cols_left >= 32 {
                *ptr = !0u32;
                cols_left -= 32;
                ptr = ptr.add(1);
            }
            if cols_left > 0 {
                // jagged chunk!
                *ptr |= (!0) >> (32 - cols_left);
            }
        }
    }
    fn unset_entire_row(&mut self, row: usize) {
        assert!(row < self.dims[0]);
        let mut cols_left = self.dims[1];
        unsafe {
            let mut ptr = self.bytes.add(self.offset_of_chunk_unchecked([row, 0]));
            while cols_left > 0 {
                *ptr = 0u32;
                cols_left -= 32;
                ptr = ptr.add(1);
            }
        }
    }

    fn reshape(&mut self, new_dims: [usize; 2]) {
        dbg!(self.u32s_total, self.u32s_per_row);

        // 1. calc new u32s_per_row
        let new_u32s_per_row = match ceiling_to_mul_32(new_dims[1]) / 32 {
            min if min > self.u32s_per_row => Some(min * 2),
            _ => None,
        };

        // 2. calc new u32s_total
        let new_u32s_total = match new_u32s_per_row.unwrap_or(self.u32s_per_row) * new_dims[0] {
            min if min > self.u32s_total => Some(min * 2),
            _ => None,
        };

        // 3. set any bits no longer in columns to zero
        let new_last_chunk_zero_prefix = new_dims[1] % 32;
        if new_dims[1] < self.dims[1] {
            let old_min_u32_per_row = ceiling_to_mul_32(new_dims[1]) / 32;
            let new_min_u32_per_row = ceiling_to_mul_32(self.dims[1]) / 32;
            let common_rows = self.dims[0].min(new_dims[0]);
            if old_min_u32_per_row < new_min_u32_per_row {
                // zero chunks made entirely of removed columns
                for row in 0..common_rows {
                    unsafe {
                        self.bytes
                            .add(self.offset_of_chunk_unchecked([row, old_min_u32_per_row]))
                            .write_bytes(0u8, new_min_u32_per_row - old_min_u32_per_row);
                    }
                }
            }
            if new_last_chunk_zero_prefix > 0 {
                // wipe out new_last_chunk_zero_prefix-most significant bits of all new last column chunks
                let mask: u32 = !0u32 >> new_last_chunk_zero_prefix;
                for row in 0..common_rows {
                    let o_of = self.offset_of_chunk_unchecked([row, new_min_u32_per_row - 1]);
                    unsafe { *self.bytes.add(o_of) &= mask };
                }
            }
        }

        // 4. if we won't do a new allocation, zero any bit no longer in rows
        if new_dims[0] < self.dims[0] && new_u32s_total.is_none() {
            // zero all bytes from beginning of first removed row,
            // to end of last removed row
            unsafe {
                self.bytes
                    .add(self.offset_of_chunk_unchecked([new_dims[0], 0]))
                    .write_bytes(0u8, self.u32s_per_row * (self.dims[0] - new_dims[0]));
            }
        }

        dbg!(new_u32s_per_row, new_u32s_total);
        match [new_u32s_per_row, new_u32s_total] {
            [None, None] => { /* do nothing */ }
            [None, Some(new_u32s_total)] => {
                // realloc only! column alignment is still OK
                // assert!(new_u32s_total > self.u32s_total);
                let old_layout = Self::layout_for(self.u32s_total);
                let new_layout = Self::layout_for(new_u32s_total);
                let new_bytes = unsafe {
                    let new_bytes = std::alloc::alloc(new_layout) as *mut u32;
                    // copy the previous total
                    self.bytes.copy_to_nonoverlapping(new_bytes, self.u32s_total);
                    // and zero the remainder
                    new_bytes
                        .add(self.u32s_total)
                        .write_bytes(0u8, new_u32s_total - self.u32s_total);
                    // drop the previous buffer
                    std::alloc::dealloc(self.bytes as *mut u8, old_layout);
                    new_bytes
                };
                self.bytes = new_bytes;
                println!("AFTER {:?}", self.bytes);
                self.u32s_total = new_u32s_total;
            }
            [Some(new_u32s_per_row), None] => {
                // shift only!
                // assert!(new_u32s_per_row > self.u32s_per_row);
                for r in (0..self.dims[0]).rev() {
                    // iterate in REVERSE order because new row[n] may overwrite old row[n+m]
                    unsafe {
                        let src = self.bytes.add(r * self.u32s_per_row);
                        let dest = self.bytes.add(r * new_u32s_per_row);
                        // copy the used prefix
                        src.copy_to(dest, self.u32s_per_row);
                        // and zero the remainder
                        dest.add(self.u32s_per_row)
                            .write_bytes(0u8, new_u32s_per_row - self.u32s_per_row);
                    }
                }
                self.u32s_per_row = new_u32s_per_row;
            }
            [Some(new_u32s_per_row), Some(new_u32s_total)] => {
                // alloc AND shift!
                // assert!(new_u32s_total > self.u32s_total);
                // assert!(new_u32s_per_row > self.u32s_per_row);
                let old_layout = Self::layout_for(self.u32s_total);
                let new_layout = Self::layout_for(new_u32s_total);
                let new_bytes = unsafe { std::alloc::alloc(new_layout) as *mut u32 };
                for r in 0..self.dims[0] {
                    // iterate forwards over rows!
                    unsafe {
                        let src = self.bytes.add(r * self.u32s_per_row);
                        let dest = new_bytes.add(r * new_u32s_per_row);
                        // copy the used prefix
                        src.copy_to_nonoverlapping(dest, self.u32s_per_row);
                        // and zero the remainder
                        dest.add(self.u32s_per_row)
                            .write_bytes(0u8, new_u32s_per_row - self.u32s_per_row);
                    }
                }
                let fresh_rows_at = self.dims[0] * new_u32s_per_row;
                unsafe {
                    new_bytes.add(fresh_rows_at).write_bytes(0u8, new_u32s_total - fresh_rows_at);
                }
                unsafe { std::alloc::dealloc(self.bytes as *mut u8, old_layout) };
                self.u32s_per_row = new_u32s_per_row;
                self.bytes = new_bytes;
                self.u32s_total = new_u32s_total;
            }
        }
        self.dims = new_dims;
    }

    fn layout_for(u32s_total: usize) -> std::alloc::Layout {
        unsafe {
            // this layout is ALWAYS valid:
            // 1. size is always nonzero
            // 2. size is always a multiple of 4 and 4-aligned
            std::alloc::Layout::from_size_align_unchecked(4 * u32s_total.max(1), 4)
        }
    }
    fn new(dims: [usize; 2], extra_dim_space: [usize; 2]) -> Self {
        let u32s_per_row = ceiling_to_mul_32(dims[1] + extra_dim_space[1]) / 32;
        let u32s_total = u32s_per_row * (dims[0] + extra_dim_space[0]);
        let layout = Self::layout_for(u32s_total);
        let bytes = unsafe {
            // allocate
            let bytes = std::alloc::alloc(layout) as *mut u32;
            // and zero
            bytes.write_bytes(0u8, u32s_total);
            bytes
        };
        Self { bytes, u32s_total, u32s_per_row, dims }
    }
    fn assert_within_bounds(&self, at: [usize; 2]) {
        assert!(at[0] < self.dims[0]);
        assert!(at[1] < self.dims[1]);
    }
    #[inline(always)]
    fn offset_of_chunk_unchecked(&self, at: [usize; 2]) -> usize {
        (self.u32s_per_row * at[0]) + at[1] / 32
    }
    #[inline(always)]
    fn offsets_unchecked(&self, at: [usize; 2]) -> [usize; 2] {
        let of_chunk = self.offset_of_chunk_unchecked(at);
        let in_chunk = at[1] % 32;
        [of_chunk, in_chunk]
    }
    fn set(&mut self, at: [usize; 2]) {
        self.assert_within_bounds(at);
        let [o_of, o_in] = self.offsets_unchecked(at);
        unsafe { *self.bytes.add(o_of) |= 1 << o_in };
    }
    fn unset(&mut self, at: [usize; 2]) {
        self.assert_within_bounds(at);
        let [o_of, o_in] = self.offsets_unchecked(at);
        unsafe { *self.bytes.add(o_of) &= !(1 << o_in) };
    }
    fn test(&self, at: [usize; 2]) -> bool {
        self.assert_within_bounds(at);
        let [o_of, o_in] = self.offsets_unchecked(at);
        unsafe { *self.bytes.add(o_of) & (1 << o_in) != 0 }
    }
    unsafe fn copy_chunk_unchecked(&self, row: usize, col_chunk_index: usize) -> u32 {
        let o_of = (self.u32s_per_row * row) + col_chunk_index;
        *self.bytes.add(o_of)
    }

    /// return an efficient interator over column indices c in the range 0..self.dims[1]
    /// where self.test([t_row, c]) && f_rows.iter().all(|&f_row| !self.test([f_row, c]))
    fn col_iter_t1fn<'a, 'b: 'a>(
        &'a self,
        t_row: usize,
        f_rows: &'b [usize],
    ) -> impl Iterator<Item = usize> + 'a {
        // 1. ensure all ROWS indices are in range.
        assert!(t_row < self.dims[0]);
        for &f_row in f_rows.iter() {
            assert!(f_row < self.dims[0]);
        }

        // 2. construct an unsafe iterator over chunks
        // column_chunk_range ensures all col_chunk_index values are in range.
        let column_chunk_range = 0..ceiling_to_mul_32(self.dims[1]) / 32;
        let chunk_iter = column_chunk_range.map(move |col_chunk_index| {
            // SAFETY: all rows and columns have already been bounds-checked.
            let t_chunk = unsafe { self.copy_chunk_unchecked(t_row, col_chunk_index) };
            f_rows.iter().fold(t_chunk, |chunk, &f_row| {
                let f_chunk = unsafe { self.copy_chunk_unchecked(f_row, col_chunk_index) };
                chunk & !f_chunk
            })
        });

        // 3. yield columns indices from the chunk iterator
        BitChunkIter::new(chunk_iter)
    }
}

// trait RwMatrixBits {
//     fn set(&mut self, at: [usize;2]);
//     fn unset(&mut self, at: [usize;2]);
//     fn set_entire_row(&mut self, row: usize);
//     fn unset_entire_row(&mut self, row: usize);
// }

// struct MatrixRefW<'a> {
//     _inner: usize,
// }
// impl<'a> MatrixRefW<'a> {

// }

#[test]
fn matrix() {
    let mut m = FlagMatrix::new([6, 6], [0, 0]);
    for i in 0..5 {
        m.set([0, i]);
        m.set([i, i]);
    }
    m.set_entire_row(5);
    println!("{:?}", &m);
    m.reshape([6, 40]);
    let iter = m.col_iter_t1fn(0, &[1, 2, 3]);
    for c in iter {
        println!("{:?}", c);
    }
    println!("{:?}", &m);
}
