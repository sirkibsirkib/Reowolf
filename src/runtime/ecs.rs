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

struct FlagMatrix {
    bytes: *mut u32,
    u32s_per_row: usize,
    rows: usize,
    columns: usize, // conceptually: a column of BITS
                    // invariant: bytes.len() == rows * columns
}
impl Debug for FlagMatrix {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        for r in 0..self.rows {
            write!(f, "|")?;
            for c in 0..self.columns {
                write!(
                    f,
                    "{}",
                    match self.test(r, c) {
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

#[inline(always)]
fn ceiling_to_mul_32(value: usize) -> usize {
    (value + 31) & !31
}
impl Drop for FlagMatrix {
    fn drop(&mut self) {
        let layout = Self::layout_for(self.rows, self.u32s_per_row);
        unsafe {
            //?
            std::alloc::dealloc(self.bytes as *mut u8, layout);
        }
    }
}
impl FlagMatrix {
    fn layout_for(rows: usize, u32s_per_row: usize) -> std::alloc::Layout {
        let u32s = u32s_per_row * rows;
        unsafe {
            // this layout is ALWAYS valid:
            // 1. size is always nonzero
            // 2. size is always a multiple of 4 and 4-aligned
            std::alloc::Layout::from_size_align_unchecked(4 * u32s.max(1), 4)
        }
    }
    fn new(rows: usize, columns: usize) -> Self {
        let u32s_per_row = ceiling_to_mul_32(columns) / 32;
        let layout = Self::layout_for(rows, u32s_per_row);
        let bytes = unsafe {
            // ?
            std::alloc::alloc(layout)
        } as *mut u32;
        Self { bytes, u32s_per_row, rows, columns }
    }
    fn assert_within_bounds(&self, row: usize, column: usize) {
        assert!(column < self.columns);
        assert!(row < self.rows);
    }
    fn chunk_for(&self, row: usize, column: usize) -> usize {
        row * self.u32s_per_row + column / 32
    }
    #[inline]
    //given: [row, column], return [bytes_index, u32_bit]
    fn vec_addr(&self, row: usize, column: usize) -> [usize; 2] {
        let bytes_index = self.chunk_for(row, column);
        let u32_bit = column % 32;
        [bytes_index, u32_bit]
    }
    fn set(&mut self, row: usize, column: usize) {
        self.assert_within_bounds(row, column);
        let [bytes_index, u32_bit] = self.vec_addr(row, column);
        unsafe { *self.bytes.offset(bytes_index as isize) |= 1 << u32_bit };
    }
    fn unset(&mut self, row: usize, column: usize) {
        self.assert_within_bounds(row, column);
        let [bytes_index, u32_bit] = self.vec_addr(row, column);
        unsafe { *self.bytes.offset(bytes_index as isize) &= !(1 << u32_bit) };
    }
    fn test(&self, row: usize, column: usize) -> bool {
        self.assert_within_bounds(row, column);
        let [bytes_index, u32_bit] = self.vec_addr(row, column);
        unsafe { *self.bytes.offset(bytes_index as isize) & (1 << u32_bit) != 0 }
    }
    fn clear(&mut self) {
        self.rows = 0;
        self.columns = 0;
    }
    unsafe fn copy_chunk_unchecked(&self, row: usize, nth_col_chunk: usize) -> u32 {
        let i = (self.u32s_per_row * row) + nth_col_chunk;
        *self.bytes.offset(i as isize)
    }
}

#[derive(Debug, Copy, Clone)]
enum ColumnCombinator<'a> {
    Row(usize),
    True,
    False,
    And(&'a ColumnCombinator<'a>, &'a ColumnCombinator<'a>),
    Or(&'a ColumnCombinator<'a>, &'a ColumnCombinator<'a>),
    Not(&'a ColumnCombinator<'a>),
}
struct FlaggedColumnIter<'a> {
    flag_matrix: &'a FlagMatrix,
    next_column_chunk: usize,
    combinator: &'a ColumnCombinator<'a>,
}
impl<'a> FlaggedColumnIter<'a> {
    fn new(flag_matrix: &'a FlagMatrix, combinator: &'a ColumnCombinator<'a>) -> Self {
        Self { flag_matrix, combinator, next_column_chunk: 0 }
    }
    /// #Safety: bounds on self.next_column_chunk have been checked with self.flag_matrix
    /// retrieves the column chunk at self.next_column_chunk
    unsafe fn combine(&self, c: &ColumnCombinator) -> u32 {
        use ColumnCombinator as Cc;
        match c {
            Cc::Row(row) => self.flag_matrix.copy_chunk_unchecked(*row, self.next_column_chunk),
            Cc::False => 0u32,
            Cc::True => !0u32,
            Cc::And(a, b) => self.combine(a) & self.combine(b),
            Cc::Or(a, b) => self.combine(a) | self.combine(b),
            Cc::Not(a) => !self.combine(a),
        }
    }
}
impl<'a> Iterator for FlaggedColumnIter<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        struct CombineCtx<'a> {
            flag_matrix: &'a FlagMatrix,
            nth_col_chunk: usize,
        }
        if self.next_column_chunk >= self.flag_matrix.u32s_per_row {
            None
        } else {
            let x = unsafe { self.combine(self.combinator) };
            self.next_column_chunk += 1;
            Some(x)
        }
    }
}

struct ColumnIter<'a> {
    bit_chunk_iter: BitChunkIter<FlaggedColumnIter<'a>>,
}
impl<'a> ColumnIter<'a> {
    fn new(m: &'a FlagMatrix, combinator: &'a ColumnCombinator) -> Self {
        let iter = FlaggedColumnIter::new(m, combinator);
        let bit_chunk_iter = BitChunkIter::new(iter);
        Self { bit_chunk_iter }
    }
}
impl<'a> Iterator for ColumnIter<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let v: Option<usize> = self.bit_chunk_iter.next();
        v.filter(|&x| x < self.bit_chunk_iter.chunk_iter.flag_matrix.columns)
    }
}

#[test]
fn matrix() {
    let mut m = FlagMatrix::new(2, 10);
    m.set(0, 1);
    m.set(0, 2);
    m.set(1, 2);
    m.set(1, 2);
    use ColumnCombinator as Cc;
    let combinator = Cc::Or(&Cc::Row(0), &Cc::True);
    let iter = ColumnIter::new(&m, &combinator);
    for c in iter {
        println!("{:?}", c);
    }
    println!("{:?}", &m);
}
