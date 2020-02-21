use super::bits::{usize_bits, BitChunkIter};
use crate::common::*;
use core::mem::MaybeUninit;

#[derive(Default)]
struct Bitvec(Vec<usize>);
impl Bitvec {
    #[inline(always)]
    fn offsets_of(i: usize) -> [usize; 2] {
        [i / usize_bits(), i % usize_bits()]
    }
    // assumes read will not go out of bounds
    unsafe fn insert(&mut self, i: usize) {
        let [o_of, o_in] = Self::offsets_of(i);
        let chunk = self.0.get_unchecked_mut(o_of);
        *chunk |= 1 << o_in;
    }
    // assumes read will not go out of bounds
    unsafe fn remove(&mut self, i: usize) -> bool {
        let [o_of, o_in] = Self::offsets_of(i);
        let chunk = self.0.get_unchecked_mut(o_of);
        let singleton_mask = 1 << o_in;
        let was = (*chunk & singleton_mask) != 0;
        *chunk &= !singleton_mask;
        was
    }
    // assumes read will not go out of bounds
    #[inline]
    unsafe fn contains(&self, i: usize) -> bool {
        let [o_of, o_in] = Self::offsets_of(i);
        (*self.0.get_unchecked(o_of) & (1 << o_in)) != 0
    }
    fn pop_first(&mut self) -> Option<usize> {
        let i = self.first()?;
        unsafe { self.remove(i) };
        Some(i)
    }
    fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        BitChunkIter::new(self.0.iter().copied()).map(|x| x as usize)
    }
    fn first(&self) -> Option<usize> {
        self.iter().next()
    }
}

// A T-type arena which:
// 1. does not check for the ABA problem
// 2. imposes the object keys on the user
// 3. allows the reservation of a space (getting the key) to precede the value being provided.
// 4. checks for user error
//
// Data contains values in one of three states:
// 1. occupied: ininitialized. will be dropped.
// 2. vacant: uninitialized. may be reused implicitly. won't be dropped.
// 2. reserved: uninitialized. may be occupied implicitly. won't be dropped.
//
// element access is O(1)
// removal is O(1) amortized.
// insertion is O(N) with a small constant factor,
//    doing at worst one linear probe through N/(word_size) contiguous words
//
// invariant A: data elements are inititalized <=> occupied bit is set
// invariant B: occupied and vacant have an empty intersection
// invariant C: (vacant U occupied) subset of (0..data.len)
// invariant D: last element of data is not in VACANT state
// invariant E: number of allocated bits in vacant and occupied >= data.len()
// invariant F: vacant_bit_count == vacant.iter().count()
pub struct VecStorage<T> {
    data: Vec<MaybeUninit<T>>,
    occupied: Bitvec,
    vacant: Bitvec,
    occupied_bit_count: usize,
}
impl<T> Default for VecStorage<T> {
    fn default() -> Self {
        Self {
            data: Default::default(),
            vacant: Default::default(),
            occupied: Default::default(),
            occupied_bit_count: 0,
        }
    }
}
impl<T: Debug> Debug for VecStorage<T> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        enum FmtT<'a, T> {
            Vacant(usize),
            Reserved(usize),
            Occupied(usize, &'a T),
        };
        impl<T: Debug> Debug for FmtT<'_, T> {
            fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
                match self {
                    FmtT::Vacant(i) => write!(f, "{} => Vacant", i),
                    FmtT::Reserved(i) => write!(f, "{} =>Reserved", i),
                    FmtT::Occupied(i, t) => {
                        write!(f, "{} => Occupied(", i)?;
                        t.fmt(f)?;
                        write!(f, ")")
                    }
                }
            }
        }

        let iter = (0..self.data.len()).map(|i| unsafe {
            // 1. data bounds are checked by construction of i.
            // 2. occupied index => valid data is read.
            // 3. bitset bounds are ensured by invariant E.
            if self.occupied.contains(i) {
                FmtT::Occupied(i, &*self.data.get_unchecked(i).as_ptr())
            } else if self.vacant.contains(i) {
                FmtT::Vacant(i)
            } else {
                FmtT::Reserved(i)
            }
        });
        f.debug_list().entries(iter).finish()
    }
}
impl<T> Drop for VecStorage<T> {
    fn drop(&mut self) {
        self.clear();
    }
}
impl<T> VecStorage<T> {
    // ASSUMES that i in 0..self.data.len()
    unsafe fn get_occupied_unchecked(&self, i: usize) -> Option<&T> {
        if self.occupied.contains(i) {
            // 2. Invariant A => reading valid ata
            Some(&*self.data.get_unchecked(i).as_ptr())
        } else {
            None
        }
    }
    //////////////
    pub fn len(&self) -> usize {
        self.occupied_bit_count
    }
    pub fn with_reserved_range(range_end: usize) -> Self {
        let mut data = Vec::with_capacity(range_end);
        unsafe {
            // data is uninitialized, as intended
            data.set_len(range_end);
        }
        let bitset_len = (range_end + (usize_bits() - 1)) / usize_bits();
        let chunk_iter = std::iter::repeat(0usize).take(bitset_len);
        Self {
            data,
            vacant: Bitvec(chunk_iter.clone().collect()),
            occupied: Bitvec(chunk_iter.collect()),
            occupied_bit_count: 0,
        }
    }
    pub fn clear(&mut self) {
        for i in 0..self.data.len() {
            // SAFE: bitvec bounds ensured by invariant E
            if unsafe { self.occupied.contains(i) } {
                // invariant A: this element is OCCUPIED
                unsafe {
                    // 1. by construction, i is in bounds
                    // 2. i is occupied => initialized data is being dropped
                    drop(self.data.get_unchecked_mut(i).as_ptr().read());
                }
            }
        }
        self.vacant.0.clear();
        self.occupied.0.clear();
        self.occupied_bit_count = 0;
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.data.len()).filter_map(move |i| unsafe { self.get_occupied_unchecked(i) })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        (0..self.data.len()).filter_map(move |i| unsafe {
            // SAFE: bitvec bounds ensured by invariant E
            if self.occupied.contains(i) {
                // Invariant A => reading valid data
                Some(&mut *self.data.get_unchecked_mut(i).as_mut_ptr())
            } else {
                None
            }
        })
    }
    pub fn get_occupied(&self, i: usize) -> Option<&T> {
        if i >= self.data.len() {
            None
        } else {
            unsafe {
                // index is within bounds
                self.get_occupied_unchecked(i)
            }
        }
    }
    pub fn get_occupied_mut(&mut self, i: usize) -> Option<&mut T> {
        // SAFE: bitvec bounds ensured by invariant E
        if i < self.data.len() && unsafe { self.occupied.contains(i) } {
            unsafe {
                // 1. index is within bounds
                // 2. Invariant A => reading valid ata
                Some(&mut *self.data.get_unchecked_mut(i).as_mut_ptr())
            }
        } else {
            None
        }
    }
    pub fn new_reserved(&mut self) -> usize {
        if let Some(i) = self.vacant.pop_first() {
            i
        } else {
            let bitsets_need_another_chunk = self.data.len() % usize_bits() == 0;
            // every (usize_bits())th time self.data grows by 1, bitsets grow by usize_bits().
            if bitsets_need_another_chunk {
                self.vacant.0.push(0usize);
                self.occupied.0.push(0usize);
            }
            self.data.push(MaybeUninit::uninit());
            self.data.len() - 1
        }
    }
    pub fn occupy_reserved(&mut self, i: usize, t: T) {
        // SAFE: bitvec bounds ensured by invariant E
        assert!(i < self.data.len());
        // element is within bounds
        assert!(unsafe { !self.occupied.contains(i) && !self.vacant.contains(i) });
        // element is surely reserved
        unsafe {
            // 1. invariant C => write is within bounds
            // 2. i WAS reserved => no initialized data is being overwritten
            self.data.get_unchecked_mut(i).as_mut_ptr().write(t);
            self.occupied.insert(i);
        };
        self.occupied_bit_count += 1;
    }
    pub fn new_occupied(&mut self, t: T) -> usize {
        let i = self.new_reserved();
        unsafe {
            // 1. invariant C => write is within bounds
            // 2. i WAS reserved => no initialized data is being overwritten
            self.data.get_unchecked_mut(i).as_mut_ptr().write(t);
            self.occupied.insert(i);
        };
        self.occupied_bit_count += 1;
        i
    }
    pub fn vacate(&mut self, i: usize) -> Option<T> {
        // SAFE: bitvec bounds ensured by invariant E
        if i >= self.data.len() || unsafe { self.vacant.contains(i) } {
            // already vacant. nothing to do here
            return None;
        }
        // i is certainly within bounds of self.data
        // SAFE: bitvec bounds ensured by invariant E
        let value = if unsafe { self.occupied.remove(i) } {
            unsafe {
                // 1. index is within bounds
                // 2. i is occupied => initialized data is being read
                self.occupied_bit_count -= 1;
                Some(self.data.get_unchecked_mut(i).as_ptr().read())
            }
        } else {
            // reservations have no data to drop
            None
        };
        // Mark as vacant...
        if i + 1 == self.data.len() {
            // ... by truncating self.data.
            // must truncate to avoid violating invariant D.
            // pops at least once:
            while let Some(_) = self.data.pop() {
                let pop_next = self
                    .data
                    .len()
                    .checked_sub(1)
                    .map(|index| unsafe {
                        // SAFE: bitvec bounds ensured by invariant E
                        self.vacant.remove(index)
                    })
                    .unwrap_or(false);
                if !pop_next {
                    break;
                }
            }
        } else {
            // ... by populating self.vacant.
            // SAFE: bitvec bounds ensured by invariant E
            unsafe { self.vacant.insert(i) };
        }
        value
    }
    pub fn iter_reserved(&self) -> impl Iterator<Item = usize> + '_ {
        BitChunkIter::new(self.occupied.0.iter().zip(self.vacant.0.iter()).map(|(&a, &b)| !(a | b)))
            .take_while(move |&x| x < self.data.len())
    }
}

#[test]
fn vec_storage() {
    #[derive(Debug)]
    struct Foo;
    impl Drop for Foo {
        fn drop(&mut self) {
            println!("DROPPING FOO!");
        }
    }
    let mut v = VecStorage::with_reserved_range(4);
    let i0 = v.new_occupied(Foo);
    println!("{:?}", &v);

    let i1 = v.new_reserved();
    println!("{:?}", &v);

    println!("reserved {:?}", v.iter_reserved().collect::<Vec<_>>());

    println!("q {:?}", v.vacate(i0));
    println!("{:?}", &v);

    println!("q {:?}", v.vacate(2));
    println!("{:?}", &v);

    println!("q {:?}", v.vacate(1));
    println!("{:?}", &v);

    v.occupy_reserved(i1, Foo);
    println!("{:?}", &v);

    *v.get_occupied_mut(i1).unwrap() = Foo;
    println!("{:?}", &v);

    println!("q {:?}", v.vacate(i1));
    println!("{:?}", &v);
    println!("q {:?}", v.vacate(3));
    println!("{:?}", &v);
}
