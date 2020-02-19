use crate::common::*;

/// Given an iterator over BitChunk Items, iterates over the indices (each represented as a u32) for which the bit is SET,
/// treating the bits in the BitChunk as a contiguous array.
/// e.g. input [0b111000, 0b11] gives output [3, 4, 5, 32, 33].
/// observe that the bits per chunk are ordered from least to most significant bits, yielding smaller to larger usizes.
/// assumes chunk_iter will yield no more than std::u32::MAX / 32 chunks

const fn usize_bytes() -> usize {
    std::mem::size_of::<usize>()
}
const fn usize_bits() -> usize {
    usize_bytes() * 8
}

pub(crate) struct BitChunkIter<I: Iterator<Item = usize>> {
    cached: usize,
    chunk_iter: I,
    next_bit_index: u32,
}
impl<I: Iterator<Item = usize>> BitChunkIter<I> {
    pub fn new(chunk_iter: I) -> Self {
        // first chunk is always a dummy zero, as if chunk_iter yielded Some(FALSE).
        // Consequences:
        // 1. our next_bit_index is always off by usize_bits() (we correct for it in Self::next) (no additional overhead)
        // 2. we cache usize and not Option<usize>, because chunk_iter.next() is only called in Self::next.
        Self { chunk_iter, next_bit_index: 0, cached: 0 }
    }
}
impl<I: Iterator<Item = usize>> Iterator for BitChunkIter<I> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = self.cached;

        // loop until either:
        // 1. there are no more Items to return, or
        // 2. chunk encodes 1+ Items, one of which we will return.
        while chunk == 0 {
            // chunk has no bits set! get the next one...
            chunk = self.chunk_iter.next()?;

            // ... and jump self.next_bit_index to the next multiple of usize_bits().
            self.next_bit_index =
                (self.next_bit_index + usize_bits() as u32) & !(usize_bits() as u32 - 1);
        }
        // there exists 1+ set bits in chunk
        // assert(chunk > 0);

        // Until the least significant bit of chunk is 1:
        // 1. shift chunk to the right,
        // 2. and increment self.next_bit_index accordingly
        // effectively performs a little binary search, shifting 32, then 16, ...
        // TODO perhaps there is a more efficient SIMD op for this?
        const N_INIT: u32 = usize_bits() as u32 / 2;
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
        Some(self.next_bit_index - 1 - usize_bits() as u32)
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
struct Pair {
    entity: u32,
    property: u32,
}
impl From<[u32; 2]> for Pair {
    fn from([entity, property]: [u32; 2]) -> Self {
        Pair { entity, property }
    }
}
struct BitMatrix {
    bounds: Pair,
    buffer: *mut usize,
}
impl Drop for BitMatrix {
    fn drop(&mut self) {
        let total_chunks = Self::row_chunks(self.bounds.property as usize)
            * Self::column_chunks(self.bounds.entity as usize);
        let layout = Self::layout_for(total_chunks);
        unsafe {
            // ?
            std::alloc::dealloc(self.buffer as *mut u8, layout);
        }
    }
}
impl Debug for BitMatrix {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let row_chunks = Self::row_chunks(self.bounds.property as usize);
        let column_chunks = Self::column_chunks(self.bounds.entity as usize);
        for property in 0..row_chunks {
            for entity_chunk in 0..column_chunks {
                write!(f, "|")?;
                let mut chunk = unsafe { *self.buffer.add(row_chunks * entity_chunk + property) };
                let end = if entity_chunk + 1 == column_chunks {
                    self.bounds.entity % usize_bits() as u32
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
            }
            write!(f, "|\n")?;
        }
        Ok(())
    }
}
impl BitMatrix {
    #[inline]
    const fn chunk_len_ceil(value: usize) -> usize {
        (value + usize_bits() - 1) & !(usize_bits() - 1)
    }
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
        Self::chunk_len_ceil(entity_bound) / usize_bits()
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

    fn layout_for(mut total_chunks: usize) -> std::alloc::Layout {
        unsafe {
            // this layout is ALWAYS valid:
            // 1. size is always nonzero
            // 2. size is always a multiple of 4 and 4-aligned
            if total_chunks == 0 {
                total_chunks = 1;
            }
            std::alloc::Layout::from_size_align_unchecked(
                usize_bytes() * total_chunks,
                usize_bytes(),
            )
        }
    }
    /////////

    fn reshape(&mut self, bounds: Pair) {
        todo!()
    }

    fn new(bounds: Pair) -> Self {
        let total_chunks = Self::row_chunks(bounds.property as usize)
            * Self::column_chunks(bounds.entity as usize);
        let layout = Self::layout_for(total_chunks);
        let buffer;
        unsafe {
            buffer = std::alloc::alloc(layout) as *mut usize;
            buffer.write_bytes(0u8, total_chunks);
        };
        Self { buffer, bounds }
    }
    fn set(&mut self, at: Pair) {
        self.assert_within_bounds(at);
        let [o_of, o_in] = self.offsets_unchecked(at);
        unsafe { *self.buffer.add(o_of) |= 1 << o_in };
    }
    fn unset(&mut self, at: Pair) {
        self.assert_within_bounds(at);
        let [o_of, o_in] = self.offsets_unchecked(at);
        unsafe { *self.buffer.add(o_of) &= !(1 << o_in) };
    }
    fn test(&self, at: Pair) -> bool {
        self.assert_within_bounds(at);
        let [o_of, o_in] = self.offsets_unchecked(at);
        unsafe { *self.buffer.add(o_of) & 1 << o_in != 0 }
    }

    fn batch_mut<'a, 'b>(&mut self, mut chunk_mut_fn: impl FnMut(&'b mut [BitChunk])) {
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
            let mut ptr =
                unsafe { self.buffer.add((column_chunks - 1) as usize * row_chunks as usize) };
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
    fn iter_entities_where<'a, 'b>(
        &'a self,
        buf: &'b mut Vec<usize>,
        mut fold_fn: impl FnMut(&'b [BitChunk]) -> BitChunk,
    ) -> BitChunkIter<std::vec::Drain<'b, usize>> {
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
        BitChunkIter::new(buf.drain(buf_start..))
    }
}

use derive_more::*;
#[derive(
    Debug, Copy, Clone, BitAnd, Not, BitOr, BitXor, BitAndAssign, BitOrAssign, BitXorAssign,
)]
#[repr(transparent)]
pub struct BitChunk(usize);
impl BitChunk {
    const fn bits() -> usize {
        Self::bytes() * 8
    }
    const fn bytes() -> usize {
        std::mem::size_of::<Self>()
    }
    const fn any(self) -> bool {
        self.0 != FALSE.0
    }
    const fn all(self) -> bool {
        self.0 == TRUE.0
    }
}
const TRUE: BitChunk = BitChunk(!0);
const FALSE: BitChunk = BitChunk(0);

#[test]
fn matrix_test() {
    let mut m = BitMatrix::new(Pair { entity: 70, property: 3 });
    m.set([2, 0].into());
    m.set([40, 1].into());
    m.set([40, 2].into());
    m.set([40, 0].into());
    println!("{:?}", &m);

    m.batch_mut(|p| p[0] = TRUE);
    println!("{:?}", &m);

    for i in (0..40).step_by(7) {
        m.unset([i, 0].into());
    }
    m.unset([62, 0].into());
    println!("{:?}", &m);

    m.batch_mut(move |p| p[1] = p[0] ^ TRUE);
    println!("{:?}", &m);

    let mut buf = vec![];
    for index in m.iter_entities_where(&mut buf, move |p| p[1]) {
        println!("index {}", index);
    }
}
