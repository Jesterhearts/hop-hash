#[cfg_attr(not(feature = "std"), no_std)]
use alloc::alloc::handle_alloc_error;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::fmt::Debug;
use core::mem::MaybeUninit;
use std::ptr::NonNull;

extern crate alloc;

const EMPTY: u8 = 0x80;

const HOP_RANGE: usize = 16;

#[inline(always)]
fn hashtag(tag: u64) -> u8 {
    (tag >> 57) as u8
}

/// Search for a movable index in the bubble range
///
/// # Safety
/// Hashes must be a valid pointer to a slice of `MaybeUninit<u64>` with at
/// least `empty_idx` elements.
/// The caller must ensure that `bubble_base < empty_idx` and that both indices
/// are within bounds of the `hashes` slice.
#[inline(always)]
unsafe fn find_next_movable_index(
    hashes: NonNull<[MaybeUninit<u64>]>,
    bubble_base: usize,
    empty_idx: usize,
    max_root_mask: usize,
) -> Option<usize> {
    for idx in bubble_base..empty_idx {
        // SAFETY: `idx` is within bounds since it ranges from `bubble_base` to
        // `empty_idx`, and caller guarantees both are within the hashes slice
        // bounds
        unsafe {
            let hash = hashes.as_ref().get_unchecked(idx).assume_init_read();
            let hopmap_index = (hash as usize & max_root_mask) * 16;

            let distance = empty_idx.wrapping_sub(hopmap_index);
            if distance < HOP_RANGE * 16 {
                return Some(idx);
            }
        }
    }

    None
}

#[derive(Clone, Copy)]
struct Capacity {
    base: usize,
}

impl From<usize> for Capacity {
    #[inline(always)]
    fn from(value: usize) -> Self {
        let base = if value == 0 {
            0
        } else {
            value.next_power_of_two().checked_add(HOP_RANGE).unwrap()
        };
        Capacity { base }
    }
}

impl Capacity {
    #[inline(always)]
    fn max_root_mask(self) -> usize {
        self.base.saturating_sub(HOP_RANGE).wrapping_sub(1)
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
struct HopInfo {
    neighbors: [u8; HOP_RANGE],
}

impl HopInfo {
    #[inline(always)]
    fn candidates(&self) -> u16 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        {
            return self.candidates_sse2();
        }

        #[allow(unused_variables, unreachable_code)]
        {
            let mut bits: u16 = 0;
            for i in 0..16 {
                if self.neighbors[i] != 0 {
                    bits |= 1 << i;
                }
            }
            bits
        }
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    #[inline(always)]
    fn candidates_sse2(&self) -> u16 {
        use core::arch::x86_64::*;
        // SAFETY: `self.neighbors` is a 16-byte aligned array, safe to load as __m128i
        unsafe {
            let data = _mm_load_si128(self.neighbors.as_ptr() as *const __m128i);
            let cmp = _mm_cmpgt_epi8(data, _mm_setzero_si128());
            _mm_movemask_epi8(cmp) as u16 & ((1 << HOP_RANGE) - 1) as u16
        }
    }

    #[inline(always)]
    fn is_full(&self) -> bool {
        self.candidates() == 0xFFFF
    }

    /// Clear neighbor count at the given index
    ///
    /// # Safety
    /// Caller must ensure `n_index` is within bounds of the neighbors array (<
    /// HOP_RANGE)
    #[inline(always)]
    unsafe fn clear(&mut self, n_index: usize) {
        // SAFETY: Caller ensures `n_index` is within bounds of the neighbors array
        unsafe {
            *self.neighbors.get_unchecked_mut(n_index) -= 1;
        }
    }

    /// Set neighbor count at the given index
    ///
    /// # Safety
    /// Caller must ensure `n_index` is within bounds of the neighbors array (<
    /// HOP_RANGE)
    #[inline(always)]
    unsafe fn set(&mut self, n_index: usize) {
        // SAFETY: Caller ensures `n_index` is within bounds of the neighbors array
        unsafe {
            *self.neighbors.get_unchecked_mut(n_index) += 1;
        }
    }
}

#[derive(Debug)]
struct DataLayout {
    layout: Layout,
    hopmap: usize,
    tags_offset: usize,

    buckets_offset: usize,
    hashes_offset: usize,
}

impl DataLayout {
    fn new<V>(capacity: Capacity) -> Self {
        let hopmap_layout = Layout::array::<HopInfo>(capacity.max_root_mask().wrapping_add(1))
            .expect("allocation size overflow");
        let tags_layout =
            Layout::array::<u8>(capacity.base * 16).expect("allocation size overflow");
        let buckets_layout =
            Layout::array::<MaybeUninit<V>>(capacity.base * 16).expect("allocation size overflow");
        let hashes_layout = Layout::array::<MaybeUninit<u64>>(capacity.base * 16)
            .expect("allocation size overflow");

        let (layout, hopmap) = Layout::new::<()>().extend(hopmap_layout).unwrap();
        let (layout, tags_offset) = layout.extend(tags_layout).unwrap();
        let (layout, buckets_offset) = layout.extend(buckets_layout).unwrap();
        let (layout, hashes_offset) = layout.extend(hashes_layout).unwrap();

        DataLayout {
            layout,
            hopmap,
            tags_offset,
            buckets_offset,
            hashes_offset,
        }
    }
}

pub struct HashTable<V> {
    layout: DataLayout,
    alloc: NonNull<u8>,

    overflow: Vec<(u64, V)>,

    populated: usize,
    max_pop: usize,
    max_root_mask: usize,

    _phantom: core::marker::PhantomData<V>,
}

impl<V> Debug for HashTable<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use alloc::format;
        if self.is_empty() {
            return f
                .debug_struct("HashTable")
                .field("metadata", &"empty")
                .field("populated", &self.populated)
                .field("capacity", &self.max_pop)
                .field("has_overflow", &self.overflow.len())
                .finish();
        }

        // SAFETY: Valid pointers created during initialization and capacity > 0 when
        // accessed
        unsafe {
            f.debug_struct("HashTable")
                .field(
                    "metadata",
                    &self
                        .hopmap_ptr()
                        .as_ref()
                        .iter()
                        .map(|b| {
                            let mut items = Vec::new();
                            for i in 0..HOP_RANGE {
                                if b.neighbors[i] != 0 {
                                    items.push(format!("{i:02}x{:02}", b.neighbors[i]));
                                } else {
                                    items.push(".....".to_string());
                                }
                            }
                            items.join(", ")
                        })
                        .collect::<Vec<_>>(),
                )
                .field(
                    "popmap",
                    &self
                        .tags_ptr()
                        .as_ref()
                        .chunks(16)
                        .map(|w| {
                            let mut items = Vec::new();
                            for b in w {
                                if *b == 0 {
                                    items.push("..".to_string());
                                } else {
                                    items.push(format!("{:02x}", b));
                                }
                            }
                            items.join(", ")
                        })
                        .collect::<Vec<_>>(),
                )
                .field("populated", &self.populated)
                .field("capacity", &self.max_pop)
                .field("has_overflow", &self.overflow.len())
                .finish()
        }
    }
}

impl<V> Drop for HashTable<V> {
    fn drop(&mut self) {
        // SAFETY: All pointers are valid, allocated by this struct. Values are properly
        // initialized before drop.
        unsafe {
            if core::mem::needs_drop::<V>() && self.populated > 0 {
                for (word_idx, word) in self.tags_ptr().as_ref().iter().enumerate() {
                    if *word != EMPTY {
                        self.buckets_ptr()
                            .as_mut()
                            .get_unchecked_mut(word_idx)
                            .assume_init_drop();
                    }
                }
            }

            if self.layout.layout.size() == 0 {
                return;
            }
            alloc::alloc::dealloc(self.alloc.as_ptr(), self.layout.layout);
        }
    }
}

impl<V> HashTable<V> {
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity: Capacity = ((capacity.div_ceil(16) as u128 * 16 / 15) as usize).into();

        let layout = DataLayout::new::<V>(capacity);
        let alloc = if layout.layout.size() == 0 {
            NonNull::dangling()
        } else {
            // SAFETY: Layout size is non-zero, alloc returns valid pointer and we handle
            // alloc errors when it fails and returns null.
            unsafe {
                let raw_alloc = alloc::alloc::alloc(layout.layout);
                if raw_alloc.is_null() {
                    handle_alloc_error(layout.layout);
                }

                core::ptr::write_bytes(raw_alloc, 0x0, layout.tags_offset);
                core::ptr::write_bytes(
                    raw_alloc.add(layout.tags_offset),
                    EMPTY,
                    layout.buckets_offset - layout.tags_offset,
                );

                NonNull::new_unchecked(raw_alloc)
            }
        };

        Self {
            layout,
            alloc,
            overflow: Vec::new(),
            populated: 0,
            max_pop: ((capacity.max_root_mask().wrapping_add(1) * 16) as u128 * 15 / 16) as usize,
            max_root_mask: capacity.max_root_mask(),
            _phantom: core::marker::PhantomData,
        }
    }

    fn hopmap_ptr(&self) -> NonNull<[HopInfo]> {
        // SAFETY: Allocation is valid and properly sized for the hopmap slice
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.hopmap).cast(),
                self.max_root_mask.wrapping_add(1),
            )
        }
    }

    fn buckets_ptr(&self) -> NonNull<[MaybeUninit<V>]> {
        // SAFETY: Allocation is valid and properly sized for the buckets slice
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.buckets_offset).cast(),
                (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * 16,
            )
        }
    }

    fn hashes_ptr(&self) -> NonNull<[MaybeUninit<u64>]> {
        // SAFETY: Allocation is valid and properly sized for the hashes slice
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.hashes_offset).cast(),
                (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * 16,
            )
        }
    }

    fn tags_ptr(&self) -> NonNull<[u8]> {
        // SAFETY: Allocation is valid and properly sized for the tags slice
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.tags_offset).cast(),
                (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * 16,
            )
        }
    }

    pub fn iter(&self) -> Iter<'_, V> {
        Iter {
            table: self,
            bucket_index: 0,
            overflow_index: 0,
        }
    }

    pub fn drain(&mut self) -> Drain<'_, V> {
        Drain {
            table: self,
            bucket_index: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.populated == 0
    }

    pub fn len(&self) -> usize {
        self.populated
    }

    pub fn remove(&mut self, hash: u64, eq: impl Fn(&V) -> bool) -> Option<V> {
        if self.populated == 0 {
            return None;
        }

        let hop_bucket = self.hopmap_index(hash);
        let index = self.search_neighborhood(hash, hop_bucket, &eq);
        if let Some(index) = index {
            self.populated -= 1;

            // SAFETY: `index` is validated to be within bounds by search_neighborhood
            let bucket_mut = unsafe { self.buckets_ptr().as_ref().get_unchecked(index) };
            // SAFETY: Value at this index is initialized (confirmed by occupied tag)
            let value = unsafe { bucket_mut.assume_init_read() };

            let offset = index - hop_bucket * 16;
            let n_index = offset / 16;
            // SAFETY: hop_bucket and n_index are within bounds, index was validated
            unsafe {
                // SAFETY: n_index is calculated from valid offset division
                self.hopmap_ptr()
                    .as_mut()
                    .get_unchecked_mut(hop_bucket)
                    .clear(n_index);
                self.clear_occupied(index);
            }

            return Some(value);
        }

        if self.overflow.is_empty() {
            return None;
        }

        for (idx, (_, overflow)) in self.overflow.iter().enumerate() {
            if eq(overflow) {
                self.populated -= 1;
                let value = self.overflow.swap_remove(idx);
                return Some(value.1);
            }
        }

        None
    }

    #[inline(always)]
    pub fn entry(&mut self, hash: u64, eq: impl Fn(&V) -> bool) -> Entry<'_, V> {
        self.maybe_resize_rehash();
        self.entry_impl(hash, eq)
    }

    #[inline(always)]
    fn search_neighborhood(
        &self,
        hash: u64,
        bucket: usize,
        eq: impl Fn(&V) -> bool,
    ) -> Option<usize> {
        // SAFETY: `bucket` is derived from hash and max_root_mask, ensuring it's within
        // bounds
        let mut neighborhood_mask = unsafe {
            self.hopmap_ptr()
                .as_ref()
                .get_unchecked(bucket)
                .candidates()
        };
        if neighborhood_mask == 0 {
            return None;
        }

        let tag = hashtag(hash);
        while neighborhood_mask != 0 {
            let index = neighborhood_mask.trailing_zeros() as usize;
            neighborhood_mask &= !(1 << index);

            let base = bucket * 16 + index * 16;
            // SAFETY: base is calculated from validated bucket and index within
            // neighborhood
            let tags = unsafe { self.scan_tags(base, tag) };
            if tags == 0 {
                continue;
            }

            for idx in 0..16 {
                if tags & (1 << idx) == 0 {
                    continue;
                }
                let slot = base + idx;

                // SAFETY: `slot` is calculated from validated base and idx, ensuring it's
                // within bounds
                if unsafe {
                    eq(self
                        .buckets_ptr()
                        .as_ref()
                        .get_unchecked(slot)
                        .assume_init_ref())
                } {
                    return Some(slot);
                }
            }
        }

        None
    }

    /// Scan 16 bytes starting at bucket for matching tags
    ///
    /// # Safety
    /// Caller must ensure `bucket` is within valid range such that `bucket +
    /// 15` is within the bounds of the tags array
    #[inline(always)]
    unsafe fn scan_tags(&self, bucket: usize, tag: u8) -> u16 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        {
            return unsafe { self.scan_tags_sse2(bucket, tag) };
        }

        #[allow(unused_variables, unreachable_code)]
        {
            let meta_ptr = self.tags_ptr();
            let mut tags: u16 = 0;
            for i in 0..16 {
                // SAFETY: `bucket + i` is within bounds as bucket comes from scan_tags with
                // valid base
                let t = unsafe { *meta_ptr.as_ref().get_unchecked(bucket + i) };
                if t == tag {
                    tags |= 1 << i;
                }
            }
            tags
        }
    }

    /// SSE2 optimized version of scan_tags
    ///
    /// # Safety
    /// Caller must ensure `bucket` is within valid range such that `bucket +
    /// 15` is within the bounds of the tags array
    #[inline(always)]
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    unsafe fn scan_tags_sse2(&self, bucket: usize, tag: u8) -> u16 {
        use core::arch::x86_64::*;
        // SAFETY: bucket is validated to be within bounds, and we load exactly 16 bytes
        unsafe {
            let meta_ptr = self.tags_ptr();
            let tags_ptr = meta_ptr.as_ref().as_ptr().add(bucket);
            let tag_vec = _mm_set1_epi8(tag as i8);

            let data = _mm_loadu_si128(tags_ptr as *const __m128i);
            let cmp = _mm_cmpeq_epi8(data, tag_vec);

            _mm_movemask_epi8(cmp) as u16
        }
    }

    #[inline(always)]
    fn hopmap_index(&self, hash: u64) -> usize {
        (hash as usize) & self.max_root_mask
    }

    #[inline(always)]
    fn absolute_index(&self, hop_bucket: usize, n_index: usize) -> usize {
        hop_bucket * 16 + n_index
    }

    #[inline]
    fn entry_impl(&mut self, hash: u64, eq: impl Fn(&V) -> bool) -> Entry<'_, V> {
        let hop_bucket = self.hopmap_index(hash);

        let index = self.search_neighborhood(hash, hop_bucket, &eq);
        if let Some(index) = index {
            return Entry::Occupied(OccupiedEntry {
                n_index: index - hop_bucket * 16,
                table: self,
                root_index: hop_bucket,
                overflow_index: None,
            });
        }

        if !self.overflow.is_empty() {
            for (idx, (_, overflow)) in self.overflow.iter().enumerate() {
                if eq(overflow) {
                    return Entry::Occupied(OccupiedEntry {
                        table: self,
                        root_index: hop_bucket,
                        n_index: 0,
                        overflow_index: Some(idx),
                    });
                }
            }
        }

        Entry::Vacant(self.do_vacant_lookup(hash, hop_bucket))
    }

    fn do_vacant_lookup(&mut self, hash: u64, hop_bucket: usize) -> VacantEntry<'_, V> {
        let empty_idx = self.find_next_unoccupied(self.absolute_index(hop_bucket, 0));

        if empty_idx.is_none()
            || empty_idx.unwrap() >= self.absolute_index(self.max_root_mask + 1 + HOP_RANGE, 0)
        {
            self.do_resize_rehash();
            return self.do_vacant_lookup(hash, self.hopmap_index(hash));
        }

        let mut absolute_empty_idx = empty_idx.unwrap();
        debug_assert!(unsafe { !self.is_occupied(absolute_empty_idx) });

        if absolute_empty_idx < self.absolute_index(hop_bucket + HOP_RANGE, 0) {
            return VacantEntry {
                table: self,
                hopmap_root: hop_bucket,
                hash,
                n_index: absolute_empty_idx - hop_bucket * 16,
                is_overflow: false,
            };
        }

        while absolute_empty_idx >= self.absolute_index(hop_bucket + HOP_RANGE, 0) {
            let bubble_base = absolute_empty_idx - (HOP_RANGE - 1) * 16;

            if let Some(absolute_idx) = unsafe {
                find_next_movable_index(
                    self.hashes_ptr(),
                    bubble_base,
                    absolute_empty_idx,
                    self.max_root_mask,
                )
            } {
                unsafe {
                    let moved_hash = self
                        .hashes_ptr()
                        .as_ref()
                        .get_unchecked(absolute_idx)
                        .assume_init_read();

                    core::ptr::copy_nonoverlapping(
                        self.buckets_ptr()
                            .as_ref()
                            .get_unchecked(absolute_idx)
                            .as_ptr(),
                        self.buckets_ptr()
                            .as_mut()
                            .get_unchecked_mut(absolute_empty_idx)
                            .as_mut_ptr(),
                        1,
                    );
                    self.hashes_ptr()
                        .as_mut()
                        .get_unchecked_mut(absolute_empty_idx)
                        .write(moved_hash);

                    let hopmap_root = self.hopmap_index(moved_hash);
                    let hopmap_abs_idx = self.absolute_index(hopmap_root, 0);

                    let old_off_abs = absolute_idx - hopmap_abs_idx;
                    let old_n_index = old_off_abs / 16;
                    let new_off_abs = absolute_empty_idx - hopmap_abs_idx;
                    let new_n_index = new_off_abs / 16;

                    // SAFETY: old_n_index and new_n_index are calculated from valid offset division
                    self.hopmap_ptr()
                        .as_mut()
                        .get_unchecked_mut(hopmap_root)
                        .clear(old_n_index);
                    self.hopmap_ptr()
                        .as_mut()
                        .get_unchecked_mut(hopmap_root)
                        .set(new_n_index);

                    self.clear_occupied(absolute_idx);
                    self.set_occupied(absolute_empty_idx, hashtag(moved_hash));
                    absolute_empty_idx = absolute_idx;
                }
            } else {
                let is_full = unsafe {
                    self.hopmap_ptr()
                        .as_ref()
                        .get_unchecked(hop_bucket)
                        .is_full()
                };
                if is_full {
                    return VacantEntry {
                        table: self,
                        hopmap_root: hop_bucket,
                        hash,
                        n_index: 0,
                        is_overflow: true,
                    };
                }

                self.do_resize_rehash();
                return self.do_vacant_lookup(hash, self.hopmap_index(hash));
            }
        }

        debug_assert!(unsafe { !self.is_occupied(absolute_empty_idx) });
        VacantEntry {
            n_index: absolute_empty_idx - hop_bucket * 16,
            table: self,
            hopmap_root: hop_bucket,
            hash,
            is_overflow: false,
        }
    }

    #[inline(always)]
    unsafe fn is_occupied(&self, index: usize) -> bool {
        // SAFETY: Caller ensures `index` is within bounds of the tags array
        unsafe { *self.tags_ptr().as_ref().get_unchecked(index) != EMPTY }
    }

    #[inline(always)]
    unsafe fn clear_occupied(&mut self, index: usize) {
        // SAFETY: Caller ensures `index` is within bounds of the tags array
        unsafe {
            *self.tags_ptr().as_mut().get_unchecked_mut(index) = EMPTY;
        }
    }

    #[inline(always)]
    unsafe fn set_occupied(&mut self, index: usize, tag: u8) {
        // SAFETY: Caller ensures `index` is within bounds of the tags array
        unsafe {
            *self.tags_ptr().as_mut().get_unchecked_mut(index) = tag;
        }
    }

    #[inline(always)]
    fn find_next_unoccupied(&self, start: usize) -> Option<usize> {
        // SAFETY: start is validated to be within table bounds by caller
        unsafe {
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            {
                return self.find_next_unoccupied_sse2(start);
            }

            #[allow(unused_variables, unreachable_code)]
            {
                self.tags_ptr().as_ref()[start..]
                    .iter()
                    .position(|&b| b == EMPTY)
                    .map(|idx| idx + start)
            }
        }
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    #[inline(always)]
    fn find_next_unoccupied_sse2(&self, start: usize) -> Option<usize> {
        use core::arch::x86_64::*;
        unsafe {
            let meta_ptr = self.tags_ptr();
            let tags_ptr = meta_ptr.as_ref().as_ptr().add(start);
            let len = (meta_ptr.as_ref().len()).saturating_sub(start);

            let mut offset = 0;
            while offset + 16 <= len {
                let data = _mm_loadu_si128(tags_ptr.add(offset) as *const __m128i);
                let mask = _mm_movemask_epi8(data) as u16;

                if mask != 0 {
                    let tz = mask.trailing_zeros() as usize;
                    return Some(start + offset + tz);
                }

                offset += 16;
            }

            while offset < len {
                let byte = *tags_ptr.add(offset);
                if byte == EMPTY {
                    return Some(start + offset);
                }
                offset += 1;
            }

            None
        }
    }

    #[inline]
    pub fn find(&self, hash: u64, eq: impl Fn(&V) -> bool) -> Option<&V> {
        if self.populated == 0 {
            return None;
        }

        let bucket = self.hopmap_index(hash);
        let index = self.search_neighborhood(hash, bucket, &eq);
        if let Some(index) = index {
            return Some(unsafe {
                self.buckets_ptr()
                    .as_ref()
                    .get_unchecked(index)
                    .assume_init_ref()
            });
        }

        if self.overflow.is_empty() {
            return None;
        }

        self.overflow
            .iter()
            .map(|(_, overflow)| overflow)
            .find(|&overflow| eq(overflow))
    }

    #[inline]
    pub fn find_mut(&mut self, hash: u64, eq: impl Fn(&V) -> bool) -> Option<&mut V> {
        if self.populated == 0 {
            return None;
        }

        let bucket = self.hopmap_index(hash);

        if let Some(index) = self.search_neighborhood(hash, bucket, &eq) {
            return Some(unsafe {
                self.buckets_ptr()
                    .as_mut()
                    .get_unchecked_mut(index)
                    .assume_init_mut()
            });
        }

        if self.overflow.is_empty() {
            return None;
        }

        self.overflow
            .iter_mut()
            .map(|(_, overflow)| overflow)
            .find(|overflow| eq(overflow))
    }

    #[inline]
    fn maybe_resize_rehash(&mut self) {
        if self.populated >= self.max_pop {
            self.do_resize_rehash();
        }
    }

    #[inline(always)]
    #[cold]
    fn do_resize_rehash(&mut self) {
        let capacity = self.max_root_mask.wrapping_add(1).max(HOP_RANGE) + 1;
        let capacity: Capacity = capacity.into();

        let new_layout = DataLayout::new::<V>(capacity);
        let new_alloc = unsafe {
            let raw_alloc = alloc::alloc::alloc(new_layout.layout);
            if raw_alloc.is_null() {
                handle_alloc_error(new_layout.layout);
            }
            core::ptr::write_bytes(raw_alloc, 0x0, new_layout.tags_offset);
            core::ptr::write_bytes(
                raw_alloc.add(new_layout.tags_offset),
                EMPTY,
                new_layout.buckets_offset - new_layout.tags_offset,
            );

            NonNull::new_unchecked(raw_alloc)
        };

        let old_layout = core::mem::replace(&mut self.layout, new_layout);
        let old_alloc = core::mem::replace(&mut self.alloc, new_alloc);

        let old_max_root = self.max_root_mask.wrapping_add(1);
        let old_base = old_max_root + HOP_RANGE;
        let old_empty_words = old_base * 16;

        self.max_pop = ((capacity.max_root_mask().wrapping_add(1) * 16) as u128 * 15 / 16) as usize;
        self.max_root_mask = capacity.max_root_mask();

        if self.populated == 0 {
            unsafe {
                if old_layout.layout.size() != 0 {
                    alloc::alloc::dealloc(old_alloc.as_ptr(), old_layout.layout);
                }
            }

            return;
        }

        let overflows = self.overflow.drain(..).collect::<Vec<_>>();

        let old_emptymap: NonNull<[u8]> = unsafe {
            NonNull::slice_from_raw_parts(
                old_alloc.add(old_layout.tags_offset).cast(),
                old_empty_words,
            )
        };

        let old_hashes: NonNull<[MaybeUninit<u64>]> = unsafe {
            NonNull::slice_from_raw_parts(
                old_alloc.add(old_layout.hashes_offset).cast(),
                old_base * 16,
            )
        };

        let old_buckets: NonNull<[MaybeUninit<V>]> = unsafe {
            NonNull::slice_from_raw_parts(
                old_alloc.add(old_layout.buckets_offset).cast(),
                old_base * 16,
            )
        };

        unsafe {
            self.populated = 0;

            let old_populated = find_occupied_slots(old_emptymap.as_ref());
            let mut pending_indexes = Vec::with_capacity(old_max_root * 16);
            for bucket_index in old_populated {
                let hash = old_hashes
                    .as_ref()
                    .get_unchecked(bucket_index)
                    .assume_init_read();
                let bucket = self.hopmap_index(hash);

                if self.is_occupied(self.absolute_index(bucket, 0)) {
                    pending_indexes.push(bucket_index);
                    continue;
                }

                self.populated += 1;
                core::ptr::copy_nonoverlapping(
                    old_buckets.as_ref().get_unchecked(bucket_index).as_ptr(),
                    self.buckets_ptr()
                        .as_mut()
                        .get_unchecked_mut(self.absolute_index(bucket, 0))
                        .as_mut_ptr(),
                    1,
                );
                core::ptr::copy_nonoverlapping(
                    old_hashes.as_ref().get_unchecked(bucket_index).as_ptr(),
                    self.hashes_ptr()
                        .as_mut()
                        .get_unchecked_mut(self.absolute_index(bucket, 0))
                        .as_mut_ptr(),
                    1,
                );
                self.set_occupied(self.absolute_index(bucket, 0), hashtag(hash));
                // SAFETY: index 0 is always within bounds of HOP_RANGE
                self.hopmap_ptr().as_mut().get_unchecked_mut(bucket).set(0);
            }

            let mut needs_shuffle = Vec::with_capacity(pending_indexes.len());
            for old_index in pending_indexes {
                let hash = old_hashes
                    .as_ref()
                    .get_unchecked(old_index)
                    .assume_init_read();
                let bucket = self.hopmap_index(hash);
                let empty = self.find_next_unoccupied(self.absolute_index(bucket, 0));
                if empty.is_none() || empty.unwrap() > self.absolute_index(bucket + HOP_RANGE, 0) {
                    needs_shuffle.push(old_index);
                    continue;
                }
                self.populated += 1;

                let absolute_empty_idx = empty.unwrap();
                core::ptr::copy_nonoverlapping(
                    old_buckets.as_ref().get_unchecked(old_index).as_ptr(),
                    self.buckets_ptr()
                        .as_mut()
                        .get_unchecked_mut(absolute_empty_idx)
                        .as_mut_ptr(),
                    1,
                );
                core::ptr::copy_nonoverlapping(
                    old_hashes.as_ref().get_unchecked(old_index).as_ptr(),
                    self.hashes_ptr()
                        .as_mut()
                        .get_unchecked_mut(absolute_empty_idx)
                        .as_mut_ptr(),
                    1,
                );
                let n_index = (absolute_empty_idx - bucket * 16) / 16;
                self.set_occupied(absolute_empty_idx, hashtag(hash));
                // SAFETY: n_index is calculated from valid offset division within HOP_RANGE
                self.hopmap_ptr()
                    .as_mut()
                    .get_unchecked_mut(bucket)
                    .set(n_index);
            }

            for old_index in needs_shuffle {
                let hash = old_hashes
                    .as_ref()
                    .get_unchecked(old_index)
                    .assume_init_read();
                let bucket = self.hopmap_index(hash);
                self.do_vacant_lookup(hash, bucket).insert(
                    old_buckets
                        .as_ref()
                        .get_unchecked(old_index)
                        .assume_init_read(),
                );
            }

            for (hash, overflow) in overflows {
                let bucket = self.hopmap_index(hash);
                self.do_vacant_lookup(hash, bucket).insert(overflow);
            }

            if old_layout.layout.size() != 0 {
                alloc::alloc::dealloc(old_alloc.as_ptr(), old_layout.layout);
            }
        }
    }

    pub fn capacity(&self) -> usize {
        self.max_pop
    }
}

#[inline(always)]
fn find_occupied_slots(tags: &[u8]) -> impl Iterator<Item = usize> {
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    {
        Sse2FindOccupiedIter {
            tags: tags.as_ptr(),
            total_bytes: tags.len(),
            byte_index: 0,
            bit_mask: 0,
        }
    }

    #[cfg(not(all(target_arch = "x86_64", target_feature = "sse2")))]
    {
        tags.iter()
            .enumerate()
            .filter(|&(_, b)| *b != EMPTY)
            .map(|(i, _)| i)
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
struct Sse2FindOccupiedIter {
    tags: *const u8,
    total_bytes: usize,
    byte_index: usize,
    bit_mask: u16,
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
impl Iterator for Sse2FindOccupiedIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        use core::arch::x86_64::*;
        unsafe {
            loop {
                if self.bit_mask != 0 {
                    let tz = self.bit_mask.trailing_zeros() as usize;
                    self.bit_mask &= !(1 << tz);
                    return Some(self.byte_index - 16 + tz);
                }

                if self.byte_index + 16 <= self.total_bytes {
                    let ptr = self.tags.add(self.byte_index);
                    let data = _mm_loadu_si128(ptr as *const __m128i);
                    self.bit_mask = !(_mm_movemask_epi8(data) as u16);
                    self.byte_index += 16;
                } else if self.byte_index < self.total_bytes {
                    let byte = *self.tags.add(self.byte_index);
                    if byte != EMPTY {
                        let idx = self.byte_index;
                        self.byte_index += 1;
                        return Some(idx);
                    }
                    self.byte_index += 1;
                } else {
                    return None;
                }
            }
        }
    }
}

pub enum Entry<'a, V> {
    Vacant(VacantEntry<'a, V>),
    Occupied(OccupiedEntry<'a, V>),
}

pub struct VacantEntry<'a, V> {
    table: &'a mut HashTable<V>,
    hopmap_root: usize,
    hash: u64,
    n_index: usize,
    is_overflow: bool,
}

impl<'a, V> VacantEntry<'a, V> {
    pub fn insert(self, value: V) -> &'a mut V {
        self.table.populated += 1;
        if self.is_overflow {
            return self.insert_overflow(value);
        }

        unsafe {
            let neighbor = self.n_index / 16;
            // SAFETY: neighbor is calculated from n_index division, within bounds
            self.table
                .hopmap_ptr()
                .as_mut()
                .get_unchecked_mut(self.hopmap_root)
                .set(neighbor);

            let target_index = self.hopmap_root * 16 + self.n_index;
            self.table.set_occupied(target_index, hashtag(self.hash));
            self.table
                .hashes_ptr()
                .as_mut()
                .get_unchecked_mut(target_index)
                .write(self.hash);

            self.table
                .buckets_ptr()
                .as_mut()
                .get_unchecked_mut(target_index)
                .write(value)
        }
    }

    #[cold]
    #[inline(never)]
    fn insert_overflow(self, value: V) -> &'a mut V {
        self.table.overflow.push((self.hash, value));
        &mut self.table.overflow.last_mut().unwrap().1
    }
}

pub struct OccupiedEntry<'a, V> {
    table: &'a mut HashTable<V>,
    root_index: usize,
    n_index: usize,
    overflow_index: Option<usize>,
}

impl<'a, V> OccupiedEntry<'a, V> {
    pub fn get(&self) -> &V {
        if let Some(overflow_index) = self.overflow_index {
            return unsafe { &self.table.overflow.get_unchecked(overflow_index).1 };
        }

        unsafe {
            self.table
                .buckets_ptr()
                .as_ref()
                .get_unchecked(self.root_index * 16 + self.n_index)
                .assume_init_ref()
        }
    }

    pub fn get_mut(&mut self) -> &mut V {
        if let Some(overflow_index) = self.overflow_index {
            return unsafe { &mut self.table.overflow.get_unchecked_mut(overflow_index).1 };
        }

        unsafe {
            self.table
                .buckets_ptr()
                .as_mut()
                .get_unchecked_mut(self.root_index * 16 + self.n_index)
                .assume_init_mut()
        }
    }

    pub fn into_mut(self) -> &'a mut V {
        if let Some(overflow_index) = self.overflow_index {
            return unsafe { &mut self.table.overflow.get_unchecked_mut(overflow_index).1 };
        }

        unsafe {
            self.table
                .buckets_ptr()
                .as_mut()
                .get_unchecked_mut(self.root_index * 16 + self.n_index)
                .assume_init_mut()
        }
    }

    pub fn remove(self) -> V {
        self.table.populated -= 1;

        if let Some(overflow_index) = self.overflow_index {
            let (_, value) = self.table.overflow.swap_remove(overflow_index);
            return value;
        }

        unsafe {
            let bucket_mut = self
                .table
                .buckets_ptr()
                .as_ref()
                .get_unchecked(self.root_index * 16 + self.n_index);
            let value = bucket_mut.assume_init_read();
            let neighbor = self.n_index / 16;
            // SAFETY: neighbor is calculated from n_index division, within bounds
            self.table
                .hopmap_ptr()
                .as_mut()
                .get_unchecked_mut(self.root_index)
                .clear(neighbor);

            self.table
                .clear_occupied(self.root_index * 16 + self.n_index);

            value
        }
    }
}

pub struct Iter<'a, V> {
    table: &'a HashTable<V>,
    bucket_index: usize,
    overflow_index: usize,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let total_slots = (self.table.max_root_mask.wrapping_add(1) + HOP_RANGE) * 16;
            while self.bucket_index < total_slots {
                if self.table.is_occupied(self.bucket_index) {
                    let bucket = &self
                        .table
                        .buckets_ptr()
                        .as_ref()
                        .get_unchecked(self.bucket_index);
                    self.bucket_index += 1;
                    return Some(bucket.assume_init_ref());
                }

                self.bucket_index += 1;
            }

            if self.overflow_index < self.table.overflow.len() {
                let item = &self.table.overflow[self.overflow_index].1;
                self.overflow_index += 1;
                return Some(item);
            }

            None
        }
    }
}

pub struct Drain<'a, V> {
    table: &'a mut HashTable<V>,
    bucket_index: usize,
}

impl<V> Drop for Drain<'_, V> {
    fn drop(&mut self) {
        for _ in self {}
    }
}

impl<'a, V> Iterator for Drain<'a, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let total_slots = (self.table.max_root_mask.wrapping_add(1) + HOP_RANGE) * 16;
            while self.bucket_index < total_slots {
                if self.table.is_occupied(self.bucket_index) {
                    self.table.populated -= 1;
                    let hash = self
                        .table
                        .hashes_ptr()
                        .as_ref()
                        .get_unchecked(self.bucket_index)
                        .assume_init_read();

                    let root = hash as usize & self.table.max_root_mask;
                    let off = self.bucket_index - root * 16;

                    // SAFETY: off / 16 is within bounds as calculated from valid bucket offset
                    self.table
                        .hopmap_ptr()
                        .as_mut()
                        .get_unchecked_mut(root)
                        .clear(off / 16);
                    self.table.clear_occupied(self.bucket_index);

                    let bucket = self
                        .table
                        .buckets_ptr()
                        .as_ref()
                        .get_unchecked(self.bucket_index);

                    self.bucket_index += 1;
                    return Some(bucket.assume_init_read());
                }

                self.bucket_index += 1;
            }

            if !self.table.overflow.is_empty() {
                self.table.populated -= 1;
                let (_, value) = self.table.overflow.pop().unwrap();
                return Some(value);
            }

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use core::hash::BuildHasher;
    use core::hash::Hasher;

    use hashbrown::DefaultHashBuilder as RandomState;

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Item {
        key: u64,
        value: i32,
    }

    fn hash_key(state: &RandomState, key: u64) -> u64 {
        let mut h = state.build_hasher();
        h.write_u64(key);
        h.finish()
    }

    #[test]
    fn insert_and_find() {
        let state = RandomState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        for k in 0..32u64 {
            let hash = hash_key(&state, k);
            match table.entry(hash, |v: &Item| v.key == k) {
                Entry::Vacant(v) => {
                    v.insert(Item {
                        key: k,
                        value: (k as i32) * 2,
                    });
                    assert_eq!(
                        table.find(hash, |v| v.key == k),
                        Some(&Item {
                            key: k,
                            value: (k as i32) * 2
                        }),
                        "{:#?}",
                        table
                    );
                }
                Entry::Occupied(_) => panic!("unexpected occupied on first insert: {:#?}", table),
            }
        }
        assert_eq!(table.len(), 32);
        for k in 0..32u64 {
            let hash = hash_key(&state, k);
            assert_eq!(
                table.find(hash, |v| v.key == k),
                Some(&Item {
                    key: k,
                    value: (k as i32) * 2
                }),
                "{:#?}",
                table
            );
        }

        let miss_hash = hash_key(&state, 999);
        assert!(table.find(miss_hash, |v| v.key == 999).is_none());
    }

    #[test]
    fn duplicate_entry_is_occupied() {
        let state = RandomState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        let k = 42u64;
        let hash = hash_key(&state, k);

        match table.entry(hash, |v| v.key == k) {
            Entry::Vacant(v) => {
                v.insert(Item { key: k, value: 7 });
            }
            Entry::Occupied(_) => panic!("should be vacant first time"),
        }

        match table.entry(hash, |v| v.key == k) {
            Entry::Occupied(mut occ) => {
                let prev_value = occ.get().value;
                *occ.get_mut() = Item { key: k, value: 11 };
                assert_eq!(prev_value, 7, "{:#?}", table);
            }
            Entry::Vacant(_) => panic!("should be occupied: {}#{:02X} in {:#?}", k, hash, table),
        }
        let found = table.find(hash, |v| v.key == k).unwrap();
        assert_eq!(found.value, 11);
    }

    #[test]
    fn find_mut_and_modify() {
        let state = RandomState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        for k in 0..5u64 {
            let hash = hash_key(&state, k);
            match table.entry(hash, |v| v.key == k) {
                Entry::Vacant(v) => {
                    v.insert(Item { key: k, value: 1 });
                }
                _ => unreachable!(),
            }
        }

        for k in 0..5u64 {
            let hash = hash_key(&state, k);
            if let Some(v) = table.find_mut(hash, |v| v.key == k) {
                v.value += 9;
            }
        }
        for k in 0..5u64 {
            let hash = hash_key(&state, k);
            let v = table.find(hash, |v| v.key == k).unwrap();
            assert_eq!(v.value, 10);
        }
    }

    #[test]
    fn remove_items() {
        let state = RandomState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        for k in 0..8u64 {
            let hash = hash_key(&state, k);
            match table.entry(hash, |v| v.key == k) {
                Entry::Vacant(v) => {
                    v.insert(Item {
                        key: k,
                        value: k as i32,
                    });
                }
                _ => unreachable!(),
            }
        }
        assert_eq!(table.len(), 8);
        for k in [0u64, 3, 7] {
            let hash = hash_key(&state, k);
            let removed = table.remove(hash, |v| v.key == k).expect("should remove");
            assert_eq!(removed.key, k);
        }
        assert_eq!(table.len(), 5);

        let hash = hash_key(&state, 1000);
        assert!(table.remove(hash, |v| v.key == 1000,).is_none());
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn insert_many() {
        let state = RandomState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        for k in 0..100000u64 {
            let hash = hash_key(&state, k);
            match table.entry(hash, |v| v.key == k) {
                Entry::Vacant(v) => {
                    v.insert(Item {
                        key: k,
                        value: k as i32,
                    });

                    assert_eq!(
                        table.find(hash, |v| v.key == k),
                        Some(&Item {
                            key: k,
                            value: k as i32
                        })
                    );
                }
                _ => unreachable!(),
            }
        }

        dbg!(&table);

        assert_eq!(table.len(), 100000, "{:#?}", table);
        for k in 0..100000u64 {
            let hash = hash_key(&state, k);

            assert_eq!(
                table.find(hash, |v| v.key == k),
                Some(&Item {
                    key: k,
                    value: k as i32
                }),
                "{:#?}",
                table
            );
        }
    }

    #[test]
    fn explicit_collision() {
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        let hash = 0;
        for k in 0..65u64 {
            match table.entry(hash, |v| v.key == k) {
                Entry::Vacant(v) => {
                    v.insert(Item {
                        key: k,
                        value: k as i32,
                    });
                }
                _ => unreachable!(),
            }
        }

        assert_eq!(table.len(), 65);
        for k in 0..65u64 {
            assert_eq!(
                table.find(hash, |v| v.key == k),
                Some(&Item {
                    key: k,
                    value: k as i32
                }),
                "{:#?}",
                table
            );
        }
    }

    #[test]
    fn capacity_debug() {
        println!("=== Capacity Debug ===");
        for requested in [16, 32, 64, 128, 256, 512, 1024] {
            let table: HashTable<i32> = HashTable::with_capacity(requested);
            println!(
                "Requested: {}, Actual capacity: {}, Ratio: {:.2}%",
                requested,
                table.capacity(),
                table.capacity() as f64 / requested as f64 * 100.0
            );
        }
    }

    #[test]
    fn iter_and_drain() {
        let state = RandomState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        for k in 10..20u64 {
            let hash = hash_key(&state, k);
            match table.entry(hash, |v| v.key == k) {
                Entry::Vacant(v) => {
                    v.insert(Item {
                        key: k,
                        value: (k as i32) + 1,
                    });
                }
                _ => unreachable!(),
            }
        }
        let collected: Vec<u64> = table.iter().map(|v| v.key).collect();
        assert_eq!(collected.len(), 10, "{:#?}", table);
        for k in 10..20u64 {
            assert!(collected.contains(&k));
        }

        let drained: Vec<Item> = table.drain().collect();
        assert_eq!(drained.len(), 10);
        assert_eq!(table.len(), 0);

        for k in 10..20u64 {
            let hash = hash_key(&state, k);
            assert!(table.find(hash, |v| v.key == k).is_none());
        }
    }
}
