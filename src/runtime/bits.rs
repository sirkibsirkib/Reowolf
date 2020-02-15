use crate::common::*;

/// Converts an iterator over contiguous u32 chunks into an iterator over usize
/// e.g. input [0b111000, 0b11] gives output [3, 4, 5, 32, 33]
/// observe that the bits per chunk are ordered from least to most significant bits, yielding smaller to larger usizes.
/// works by draining the inner u32 chunk iterator one u32 at a time, then draining that chunk until its 0.
struct BitChunkIter<I: Iterator<Item = u32>> {
    chunk_iter: I,
    next_bit_index: u32,
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
    type Item = u32;
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
        fn skip_n_zeroes(chunk: &mut u32, n: u32, next_bit_index: &mut u32) {
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

/*  --properties-->
     ___ ___ ___ ___
    |___|___|___|___|
  | |___|___|___|___|
  | |___|___|___|___|
  | |___|___|___|___|
  |
  V
 entity chunks (groups of 32)
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
    buffer: *mut u32,
}
impl Drop for BitMatrix {
    fn drop(&mut self) {
        let total_chunks = Self::row_chunks(self.bounds.property) as usize
            * Self::column_chunks(self.bounds.entity) as usize;
        let layout = Self::layout_for(total_chunks);
        unsafe {
            // ?
            std::alloc::dealloc(self.buffer as *mut u8, layout);
        }
    }
}
impl Debug for BitMatrix {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let row_chunks = Self::row_chunks(self.bounds.property) as usize;
        let column_chunks = Self::column_chunks(self.bounds.entity) as usize;
        for property in 0..row_chunks {
            for entity_chunk in 0..column_chunks {
                write!(f, "|")?;
                let mut chunk = unsafe { *self.buffer.add(row_chunks * entity_chunk + property) };
                let end =
                    if entity_chunk + 1 == column_chunks { self.bounds.entity % 32 } else { 32 };
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
    fn ceiling_to_mul_32(value: u32) -> u32 {
        (value + 31) & !31
    }
    #[inline]
    fn row_of(entity: u32) -> u32 {
        entity / 32
    }
    #[inline]
    fn row_chunks(property_bound: u32) -> u32 {
        property_bound
    }
    #[inline]
    fn column_chunks(entity_bound: u32) -> u32 {
        Self::ceiling_to_mul_32(entity_bound) / 32
    }
    #[inline]
    fn offsets_unchecked(&self, at: Pair) -> [usize; 2] {
        let o_in = at.entity as usize % 32;
        let row = Self::row_of(at.entity);
        let row_chunks = self.bounds.property;
        let o_of = row as usize * row_chunks as usize + at.property as usize;
        [o_of, o_in]
    }
    // returns a u32 which has bits 000...000111...111
    // for the last JAGGED chunk given the column size
    // if the last chunk is not jagged (when entity_bound % 32 == 0)
    // None is returned,
    // otherwise Some(x) is returned such that x & chunk would mask out
    // the bits NOT in 0..entity_bound
    fn last_row_chunk_mask(entity_bound: u32) -> Option<u32> {
        let zero_prefix_len = entity_bound % 32;
        if zero_prefix_len == 0 {
            None
        } else {
            Some(!0u32 >> (32 - zero_prefix_len))
        }
    }
    fn assert_within_bounds(&self, at: Pair) {
        assert!(at.entity < self.bounds.entity);
        assert!(at.property < self.bounds.property);
    }
    /////////

    fn reshape(&mut self, dims: [usize; 2]) {
        todo!()
    }

    fn new(bounds: Pair) -> Self {
        let total_chunks = Self::row_chunks(bounds.property) as usize
            * Self::column_chunks(bounds.entity) as usize;
        let layout = Self::layout_for(total_chunks);
        let buffer;
        unsafe {
            buffer = std::alloc::alloc(layout) as *mut u32;
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
        unsafe { (*self.buffer.add(o_of) & 1 << o_in) != 0 }
    }

    fn batch_mut<'a, 'b>(&mut self, mut chunk_mut_fn: impl FnMut(&'b mut [u32])) {
        let row_chunks = Self::row_chunks(self.bounds.property) as usize;
        let column_chunks = Self::column_chunks(self.bounds.entity);
        let mut ptr = self.buffer;
        for _row in 0..column_chunks {
            let slice;
            unsafe {
                slice = std::slice::from_raw_parts_mut(ptr, row_chunks);
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

    fn iter_entities_where<'a, 'b>(
        &'a self,
        buf: &'b mut Vec<u32>,
        mut fold_fn: impl FnMut(&'b [u32]) -> u32,
    ) -> impl Iterator<Item = u32> + 'b {
        let buf_start = buf.len();
        let row_chunks = Self::row_chunks(self.bounds.property) as usize;
        let column_chunks = Self::column_chunks(self.bounds.entity);
        let mut ptr = self.buffer;
        for _row in 0..column_chunks {
            let slice;
            unsafe {
                slice = std::slice::from_raw_parts(ptr, row_chunks);
                ptr = ptr.add(row_chunks);
            }
            buf.push(fold_fn(slice));
        }
        if let Some(mask) = Self::last_row_chunk_mask(self.bounds.entity) {
            *buf.iter_mut().last().unwrap() &= mask;
        }
        BitChunkIter::new(buf.drain(buf_start..))
    }

    fn layout_for(total_chunks: usize) -> std::alloc::Layout {
        unsafe {
            // this layout is ALWAYS valid:
            // 1. size is always nonzero
            // 2. size is always a multiple of 4 and 4-aligned
            std::alloc::Layout::from_size_align_unchecked(4 * total_chunks.max(1), 4)
        }
    }
}

#[test]
fn matrix_test() {
    let mut m = BitMatrix::new(Pair { entity: 50, property: 3 });
    m.set([2, 0].into());
    m.set([40, 1].into());
    m.set([40, 2].into());
    m.set([40, 0].into());
    println!("{:?}", &m);

    m.batch_mut(|p| p[0] = !0);
    println!("{:?}", &m);

    let mut buf = vec![];
    for index in m.iter_entities_where(&mut buf, move |p| p[0] ^ p[1] ^ p[2]) {
        println!("index {}", index);
    }
    for index in m.iter_entities_where(&mut buf, move |p| (p[0] | p[1]) & p[2]) {
        println!("index {}", index);
    }
}

// TODO something still a bit screwy with 1s where theere should be zeroes
