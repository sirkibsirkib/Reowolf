use crate::common::*;
use std::alloc::Layout;

/// Given an iterator over BitChunk Items, iterates over the indices (each represented as a u32) for which the bit is SET,
/// treating the bits in the BitChunk as a contiguous array.
/// e.g. input [0b111000, 0b11] gives output [3, 4, 5, 32, 33].
/// observe that the bits per chunk are ordered from least to most significant bits, yielding smaller to larger usizes.
/// assumes chunk_iter will yield no more than std::u32::MAX / 32 chunks

pub const fn usize_bytes() -> usize {
    std::mem::size_of::<usize>()
}
pub const fn usize_bits() -> usize {
    usize_bytes() * 8
}
pub const fn usizes_for_bits(bits: usize) -> usize {
    (bits + (usize_bits() - 1)) / usize_bits()
}

type Chunk = usize;
type BitIndex = usize;

pub(crate) struct BitChunkIter<I: Iterator<Item = Chunk>> {
    cached: usize,
    chunk_iter: I,
    next_bit_index: BitIndex,
}
impl<I: Iterator<Item = Chunk>> BitChunkIter<I> {
    pub fn new(chunk_iter: I) -> Self {
        // first chunk is always a dummy zero, as if chunk_iter yielded Some(FALSE_CHUNK).
        // Consequences:
        // 1. our next_bit_index is always off by usize_bits() (we correct for it in Self::next) (no additional overhead)
        // 2. we cache Chunk and not Option<Chunk>, because chunk_iter.next() is only called in Self::next.
        Self { chunk_iter, next_bit_index: 0, cached: 0 }
    }
}
impl<I: Iterator<Item = Chunk>> Iterator for BitChunkIter<I> {
    type Item = BitIndex;
    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = self.cached;

        // loop until either:
        // 1. there are no more Items to return, or
        // 2. chunk encodes 1+ Items, one of which we will return.
        while chunk == 0 {
            // chunk has no bits set! get the next one...
            chunk = self.chunk_iter.next()?;

            // ... and jump self.next_bit_index to the next multiple of usize_bits().
            self.next_bit_index = (self.next_bit_index + usize_bits()) & !(usize_bits() - 1);
        }
        // there exists 1+ set bits in chunk
        // assert(chunk > 0);

        // Until the least significant bit of chunk is 1:
        // 1. shift chunk to the right,
        // 2. and increment self.next_bit_index accordingly
        // effectively performs a little binary search, shifting 32, then 16, ...
        // TODO perhaps there is a more efficient SIMD op for this?
        const N_INIT: BitIndex = usize_bits() / 2;
        let mut n = N_INIT;
        while n >= 1 {
            // n is [32,16,8,4,2,1] on 64-bit machine
            // this loop is unrolled with release optimizations
            let n_least_significant_mask = (1 << n) - 1;
            if chunk & n_least_significant_mask == 0 {
                // no 1 set within 0..n least significant bits.
                self.next_bit_index += n;
                chunk >>= n;
            }
            n /= 2;
        }
        // least significant bit of chunk is 1. Item to return is known.
        // assert(chunk & 1 == 1)

        // prepare our state for the next time Self::next is called.
        // Overwrite self.cached such that its shifted state is retained,
        // and jump over the bit whose index we are about to return.
        self.next_bit_index += 1;
        self.cached = chunk >> 1;

        // returned index is usize_bits() smaller than self.next_bit_index because we use an
        // off-by-usize_bits() encoding to avoid having to cache an Option<usize>.
        Some(self.next_bit_index - 1 - usize_bits())
    }
}

pub(crate) struct BitChunkIterRev<I: ExactSizeIterator<Item = Chunk>> {
    cached: usize,
    chunk_iter: I,
    next_bit_index: BitIndex,
}
impl<I: ExactSizeIterator<Item = Chunk>> BitChunkIterRev<I> {
    pub fn new(chunk_iter: I) -> Self {
        let next_bit_index = chunk_iter.len() * usize_bits();
        Self { chunk_iter, next_bit_index, cached: 0 }
    }
}
impl<I: ExactSizeIterator<Item = Chunk>> Iterator for BitChunkIterRev<I> {
    type Item = BitIndex;
    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = self.cached;
        if chunk == 0 {
            self.next_bit_index += usize_bits();
            loop {
                self.next_bit_index -= usize_bits();
                chunk = self.chunk_iter.next()?;
                if chunk != 0 {
                    break;
                }
            }
        }
        const N_INIT: BitIndex = usize_bits() / 2;
        let mut n = N_INIT;
        while n >= 1 {
            let n_most_significant_mask = !0 << (usize_bits() - n);
            if chunk & n_most_significant_mask == 0 {
                self.next_bit_index -= n;
                chunk <<= n;
            }
            n /= 2;
        }
        self.cached = chunk << 1;
        self.next_bit_index -= 1;
        Some(self.next_bit_index)
    }
}

/*  --properties-->
     ___ ___ ___ ___
    |___|___|___|___|
  | |___|___|___|___|
  | |___|___|___|___|
  | |___|___|___|___|
  |
  V
 entity chunks (groups of size usize_bits())
*/

// TODO newtypes Entity and Property

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Pair {
    pub entity: u32,
    pub property: u32,
}
impl From<[u32; 2]> for Pair {
    fn from([entity, property]: [u32; 2]) -> Self {
        Pair { entity, property }
    }
}
impl Default for BitMatrix {
    fn default() -> Self {
        Self::new(Pair { entity: 0, property: 0 })
    }
}
pub struct BitMatrix {
    buffer: *mut usize,
    bounds: Pair,
    layout: Layout, // layout of the currently-allocated buffer
}
impl Drop for BitMatrix {
    fn drop(&mut self) {
        unsafe {
            // ?
            std::alloc::dealloc(self.buffer as *mut u8, self.layout);
        }
    }
}
impl Debug for BitMatrix {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        struct FmtRow<'a> {
            me: &'a BitMatrix,
            property: usize,
        };
        impl Debug for FmtRow<'_> {
            fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
                let row_chunks = BitMatrix::row_chunks(self.me.bounds.property as usize);
                let column_chunks = BitMatrix::column_chunks(self.me.bounds.entity as usize);
                write!(f, "|")?;
                for entity_chunk in 0..column_chunks {
                    let mut chunk =
                        unsafe { *self.me.buffer.add(row_chunks * entity_chunk + self.property) };
                    let end = if entity_chunk + 1 == column_chunks {
                        self.me.bounds.entity % usize_bits() as u32
                    } else {
                        usize_bits() as u32
                    };
                    for _ in 0..end {
                        let c = match chunk & 1 {
                            0 => '0',
                            _ => '1',
                        };
                        write!(f, "{}", c)?;
                        chunk >>= 1;
                    }
                    write!(f, "_")?;
                }
                Ok(())
            }
        }
        let row_chunks = BitMatrix::row_chunks(self.bounds.property as usize);
        let iter = (0..row_chunks).map(move |property| FmtRow { me: self, property });
        f.debug_list().entries(iter).finish()
    }
}
impl BitMatrix {
    #[inline]
    const fn row_of(entity: usize) -> usize {
        entity / usize_bits()
    }
    #[inline]
    const fn row_chunks(property_bound: usize) -> usize {
        property_bound
    }
    #[inline]
    const fn column_chunks(entity_bound: usize) -> usize {
        usizes_for_bits(entity_bound)
    }
    #[inline]
    fn offsets_unchecked(&self, at: Pair) -> [usize; 2] {
        let o_in = at.entity as usize % usize_bits();
        let row = Self::row_of(at.entity as usize);
        let row_chunks = self.bounds.property as usize;
        let o_of = row * row_chunks + at.property as usize;
        [o_of, o_in]
    }
    // returns a u32 which has bits 000...000111...111
    // for the last JAGGED chunk given the column size
    // if the last chunk is not jagged (when entity_bound % 32 == 0)
    // None is returned,
    // otherwise Some(x) is returned such that x & chunk would mask out
    // the bits NOT in 0..entity_bound
    fn last_row_chunk_mask(entity_bound: u32) -> Option<usize> {
        let zero_prefix_len = entity_bound as usize % usize_bits();
        if zero_prefix_len == 0 {
            None
        } else {
            Some(!0 >> (usize_bits() - zero_prefix_len))
        }
    }
    fn assert_within_bounds(&self, at: Pair) {
        assert!(at.entity < self.bounds.entity);
        assert!(at.property < self.bounds.property);
    }

    fn layout_for(total_chunks: usize) -> std::alloc::Layout {
        unsafe {
            // this layout is ALWAYS valid:
            // 1. size is always nonzero
            // 2. size is always a multiple of 4 and 4-aligned
            Layout::from_size_align_unchecked(usize_bytes() * total_chunks.max(1), usize_bytes())
        }
    }
    /////////
    pub fn get_bounds(&self) -> &Pair {
        &self.bounds
    }
    pub fn grow_to(&mut self, bounds: Pair) {
        assert!(bounds.entity >= self.bounds.entity);
        assert!(bounds.property >= self.bounds.property);

        let old_row_chunks = Self::row_chunks(self.bounds.property as usize);
        let old_col_chunks = Self::column_chunks(self.bounds.entity as usize);
        let new_row_chunks = Self::row_chunks(bounds.property as usize);
        let new_col_chunks = Self::column_chunks(bounds.entity as usize);

        let new_layout = Self::layout_for(new_row_chunks * new_col_chunks);
        let new_buffer = unsafe {
            let new_buffer = std::alloc::alloc(new_layout) as *mut usize;
            let mut src: *mut usize = self.buffer;
            let mut dest: *mut usize = new_buffer;
            let row_chunk_diff = new_row_chunks - old_row_chunks;
            for _col_idx in 0..old_col_chunks {
                src.copy_to_nonoverlapping(dest, old_row_chunks);
                src = src.add(old_row_chunks);
                dest = dest.add(old_row_chunks);
                if row_chunk_diff > 0 {
                    dest.write_bytes(0u8, row_chunk_diff);
                    dest = dest.add(row_chunk_diff);
                }
            }
            let last_zero_chunks = (new_col_chunks - old_col_chunks) * new_row_chunks;
            dest.write_bytes(0u8, last_zero_chunks);
            new_buffer
        };
        self.layout = new_layout;
        self.buffer = new_buffer;
        self.bounds = bounds;
    }
    pub fn clear(&mut self) {
        let total_chunks = Self::row_chunks(self.bounds.property as usize)
            * Self::column_chunks(self.bounds.entity as usize);
        unsafe {
            self.buffer.write_bytes(0u8, total_chunks);
        }
    }
    pub fn new(bounds: Pair) -> Self {
        let total_chunks = Self::row_chunks(bounds.property as usize)
            * Self::column_chunks(bounds.entity as usize);
        let layout = Self::layout_for(total_chunks);
        let buffer;
        unsafe {
            buffer = std::alloc::alloc(layout) as *mut usize;
            buffer.write_bytes(0u8, total_chunks);
        };
        Self { buffer, bounds, layout }
    }
    pub fn set(&mut self, at: Pair) {
        self.assert_within_bounds(at);
        let [o_of, o_in] = self.offsets_unchecked(at);
        unsafe { *self.buffer.add(o_of) |= 1 << o_in };
    }
    pub fn unset(&mut self, at: Pair) {
        self.assert_within_bounds(at);
        let [o_of, o_in] = self.offsets_unchecked(at);
        unsafe { *self.buffer.add(o_of) &= !(1 << o_in) };
    }
    pub fn test(&self, at: Pair) -> bool {
        self.assert_within_bounds(at);
        let [o_of, o_in] = self.offsets_unchecked(at);
        unsafe { *self.buffer.add(o_of) & 1 << o_in != 0 }
    }

    pub fn batch_mut<'a, 'b>(&mut self, mut chunk_mut_fn: impl FnMut(&'b mut [BitChunk])) {
        let row_chunks = Self::row_chunks(self.bounds.property as usize);
        let column_chunks = Self::column_chunks(self.bounds.entity as usize);
        let mut ptr = self.buffer;
        for _row in 0..column_chunks {
            let slice;
            unsafe {
                let slicey = std::slice::from_raw_parts_mut(ptr, row_chunks);
                slice = std::mem::transmute(slicey);
                ptr = ptr.add(row_chunks);
            }
            chunk_mut_fn(slice);
        }
        if let Some(mask) = Self::last_row_chunk_mask(self.bounds.entity) {
            // TODO TEST
            let mut ptr = unsafe { self.buffer.add((column_chunks - 1) * row_chunks) };
            for _ in 0..row_chunks {
                unsafe {
                    *ptr &= mask;
                    ptr = ptr.add(1);
                }
            }
        }
    }

    /// given:
    /// 1. a buffer to work with
    /// 2. a _fold function_ for combining the properties of a given entity
    ///    and returning a new derived property (working )
    pub fn iter_entities_where<'a, 'b>(
        &'a self,
        buf: &'b mut Vec<usize>,
        mut fold_fn: impl FnMut(&'b [BitChunk]) -> BitChunk,
    ) -> impl Iterator<Item = u32> + 'b {
        let buf_start = buf.len();
        let row_chunks = Self::row_chunks(self.bounds.property as usize);
        let column_chunks = Self::column_chunks(self.bounds.entity as usize);
        let mut ptr = self.buffer;
        for _row in 0..column_chunks {
            let slice;
            unsafe {
                let slicey = std::slice::from_raw_parts(ptr, row_chunks);
                slice = std::mem::transmute(slicey);
                ptr = ptr.add(row_chunks);
            }
            let chunk = fold_fn(slice);
            buf.push(chunk.0);
        }
        if let Some(mask) = Self::last_row_chunk_mask(self.bounds.entity) {
            *buf.iter_mut().last().unwrap() &= mask;
        }
        BitChunkIter::new(buf.drain(buf_start..)).map(|x| x as u32)
    }
    pub fn iter_entities_where_rev<'a, 'b>(
        &'a self,
        buf: &'b mut Vec<usize>,
        mut fold_fn: impl FnMut(&'b [BitChunk]) -> BitChunk,
    ) -> impl Iterator<Item = u32> + 'b {
        let buf_start = buf.len();
        let row_chunks = Self::row_chunks(self.bounds.property as usize);
        let column_chunks = Self::column_chunks(self.bounds.entity as usize);
        let mut ptr = self.buffer;
        for _row in 0..column_chunks {
            let slice;
            unsafe {
                let slicey = std::slice::from_raw_parts(ptr, row_chunks);
                slice = std::mem::transmute(slicey);
                ptr = ptr.add(row_chunks);
            }
            let chunk = fold_fn(slice);
            buf.push(chunk.0);
        }
        if let Some(mask) = Self::last_row_chunk_mask(self.bounds.entity) {
            *buf.iter_mut().last().unwrap() &= mask;
        }
        BitChunkIterRev::new(buf.drain(buf_start..).rev()).map(|x| x as u32)
    }
}

use derive_more::*;
#[derive(
    Debug, Copy, Clone, BitAnd, Not, BitOr, BitXor, BitAndAssign, BitOrAssign, BitXorAssign,
)]
#[repr(transparent)]
pub struct BitChunk(usize);
impl BitChunk {
    const fn any(self) -> bool {
        self.0 != FALSE_CHUNK.0
    }
    const fn all(self) -> bool {
        self.0 == TRUE_CHUNK.0
    }
}
pub const TRUE_CHUNK: BitChunk = BitChunk(!0);
pub const FALSE_CHUNK: BitChunk = BitChunk(0);

#[test]
fn matrix_test() {
    let mut m = BitMatrix::new(Pair { entity: 70, property: 3 });
    m.set([2, 0].into());
    m.set([40, 1].into());
    m.set([40, 2].into());
    m.set([40, 0].into());
    println!("{:#?}", &m);

    m.batch_mut(|p| p[0] = TRUE_CHUNK);
    println!("{:#?}", &m);

    for i in (0..40).step_by(7) {
        m.unset([i, 0].into());
    }
    m.unset([62, 0].into());
    println!("{:#?}", &m);

    m.batch_mut(move |p| p[1] = p[0] ^ TRUE_CHUNK);
    println!("{:#?}", &m);

    let mut buf = vec![];
    for index in m.iter_entities_where(&mut buf, move |p| p[1]) {
        println!("index {}", index);
    }
    for index in m.iter_entities_where_rev(&mut buf, move |p| p[1]) {
        println!("index {}", index);
    }
}

#[test]
fn bit_chunk_iter_rev() {
    let x = &[0b1, 0b1000011, 0, 0, 0b101];
    for i in BitChunkIterRev::new(x.iter().copied()) {
        println!("i = {:?}", i);
    }
}
