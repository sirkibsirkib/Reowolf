use crate::common::*;
use core::mem::MaybeUninit;
use std::collections::BTreeSet;

// A T-type arena which:
// 1. does not check for the ABA problem
// 2. imposes the object keys on the user
// 3. allows the reservation of a space (getting the key) to precede the value being provided.
//
// data contains values in one of three states:
// 1. occupied: ininitialized. will be dropped.
// 2. vacant: uninitialized. may be reused implicitly. won't be dropped.
// 2. reserved: uninitialized. may be occupied implicitly. won't be dropped.
// invariant A: elements at indices (0..data.len()) / vacant / reserved are occupied
// invariant B: reserved & vacant = {}
// invariant C: (vacant U reserved) subset of (0..data.len)
// invariant D: last element of data is not in VACANT state
pub struct VecStorage<T> {
    data: Vec<MaybeUninit<T>>,
    vacant: BTreeSet<usize>,
    reserved: BTreeSet<usize>,
}
impl<T> Default for VecStorage<T> {
    fn default() -> Self {
        Self { data: Default::default(), vacant: Default::default(), reserved: Default::default() }
    }
}
impl<T: Debug> Debug for VecStorage<T> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        enum FmtT<'a, T> {
            Vacant,
            Reserved,
            Occupied(&'a T),
        };
        impl<T: Debug> Debug for FmtT<'_, T> {
            fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
                match self {
                    FmtT::Vacant => write!(f, "Vacant"),
                    FmtT::Reserved => write!(f, "Reserved"),
                    FmtT::Occupied(t) => write!(f, "Occupied({:?})", t),
                }
            }
        }
        let iter = (0..self.data.len()).map(|i| {
            if self.vacant.contains(&i) {
                FmtT::Vacant
            } else if self.reserved.contains(&i) {
                FmtT::Reserved
            } else {
                // 2. Invariant A => reading valid ata
                unsafe {
                    // 1. index is within bounds
                    // 2. i is occupied => initialized data is being dropped
                    FmtT::Occupied(&*self.data.get_unchecked(i).as_ptr())
                }
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
        if self.vacant.contains(&i) || self.reserved.contains(&i) {
            None
        } else {
            // 2. Invariant A => reading valid ata
            Some(&*self.data.get_unchecked(i).as_ptr())
        }
    }
    // breaks invariant A: returned index is in NO state
    fn pop_vacant(&mut self) -> usize {
        if let Some(i) = pop_set_arb(&mut self.vacant) {
            i
        } else {
            self.data.push(MaybeUninit::uninit());
            self.data.len() - 1
        }
    }
    //////////////
    pub fn clear(&mut self) {
        for i in 0..self.data.len() {
            if !self.vacant.contains(&i) && !self.reserved.contains(&i) {
                // invariant A: this element is OCCUPIED
                unsafe {
                    // 1. by construction, i is in bounds
                    // 2. i is occupied => initialized data is being dropped
                    drop(self.data.get_unchecked_mut(i).as_ptr().read());
                }
            }
        }
        self.vacant.clear();
        self.reserved.clear();
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.data.len()).filter_map(move |i| unsafe { self.get_occupied_unchecked(i) })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        (0..self.data.len()).filter_map(move |i| unsafe {
            if self.vacant.contains(&i) || self.reserved.contains(&i) {
                None
            } else {
                // 2. Invariant A => reading valid ata
                Some(&mut *self.data.get_unchecked_mut(i).as_mut_ptr())
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
    pub fn get_mut_occupied(&mut self, i: usize) -> Option<&mut T> {
        if i >= self.data.len() || self.vacant.contains(&i) || self.reserved.contains(&i) {
            None
        } else {
            unsafe {
                // 1. index is within bounds
                // 2. Invariant A => reading valid ata
                Some(&mut *self.data.get_unchecked_mut(i).as_mut_ptr())
            }
        }
    }
    pub fn new_reserved(&mut self) -> usize {
        let i = self.pop_vacant(); // breaks invariant A: i is in NO state
        self.reserved.insert(i); // restores invariant A
        i
    }
    pub fn occupy_reserved(&mut self, i: usize, t: T) {
        assert!(self.reserved.remove(&i)); // breaks invariant A
        unsafe {
            // 1. invariant C => write is within bounds
            // 2. i WAS reserved => no initialized data is being overwritten
            self.data.get_unchecked_mut(i).as_mut_ptr().write(t)
            // restores invariant A
        };
    }
    pub fn new_occupied(&mut self, t: T) -> usize {
        let i = self.pop_vacant(); // breaks invariant A: i is in NO state
        unsafe {
            // 1. invariant C => write is within bounds
            // 2. i WAS reserved => no initialized data is being overwritten
            self.data.get_unchecked_mut(i).as_mut_ptr().write(t)
            // restores invariant A
        };
        i
    }
    pub fn vacate(&mut self, i: usize) -> Option<T> {
        if i >= self.data.len() || self.vacant.contains(&i) {
            // already vacant. nothing to do here
            return None;
        }
        // i is certainly within bounds of self.data
        let value = if self.reserved.remove(&i) {
            // no data to drop
            None
        } else {
            // invariant A => this element is OCCUPIED!
            unsafe {
                // 1. index is within bounds
                // 2. i is occupied => initialized data is being dropped
                Some(self.data.get_unchecked_mut(i).as_ptr().read())
            }
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
                    .map(|index| self.vacant.remove(&index))
                    .unwrap_or(false);
                if !pop_next {
                    break;
                }
            }
        } else {
            // ... by populating self.vacant.
            self.vacant.insert(i);
        }
        value
    }
    pub fn iter_reserved(&self) -> impl Iterator<Item = usize> + '_ {
        self.reserved.iter().copied()
    }
}

fn pop_set_arb(s: &mut BTreeSet<usize>) -> Option<usize> {
    if let Some(&x) = s.iter().next() {
        s.remove(&x);
        Some(x)
    } else {
        None
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

    let mut v = VecStorage::default();
    let i0 = v.new_occupied(Foo);
    println!("{:?}", &v);
    let i1 = v.new_reserved();
    println!("{:?}", &v);
    let q = v.vacate(i0);
    println!("q {:?}", q);
    println!("{:?}", &v);
    v.occupy_reserved(i1, Foo);
    println!("{:?}", &v);
}
