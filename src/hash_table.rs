use alloc::alloc::handle_alloc_error;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::fmt::Debug;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

#[inline(always)]
fn target_load_factor(capacity: usize) -> usize {
    ((capacity as u128 * 99) / 100) as usize
}

#[inline(always)]
fn target_load_factor_inverse(capacity: usize) -> usize {
    ((capacity as u128 * 100) / 99) as usize
}

#[inline(always)]
fn prefetch<T>(ptr: *const T) {
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    unsafe {
        use core::arch::x86_64::*;
        _mm_prefetch(ptr as *const i8, _MM_HINT_T0);
    }
}

/// Special tag value marking an empty slot.
///
/// Chosen as 0x80 (sign bit set) so SSE2 `movemask`-based scans can leverage
/// the sign bit to detect empties efficiently.
///
/// The alternative 0x00 would also work, with hashtags using 0x80 to mark
/// occupied slots, but this is slightly faster during searches for empty slots
/// during a collision/bubbling which is hot during profiling and seems to
/// improve microbenchmarks.
const EMPTY: u8 = 0x80;

/// Number of neighbors tracked per bucket. Could be larger for wider SIMD
/// operations, but we only support SSE2. If you change this, you'll need to
/// update a lot of code that currently assumes 16-way hopscotch.
const HOP_RANGE: usize = 16;

#[inline(always)]
fn hashtag(tag: u64) -> u8 {
    (tag >> 57) as u8
}

/// Search for a movable index in the bubble range
///
/// # Safety
/// - `hashes` must point to a slice of `MaybeUninit<u64>` with length strictly
///   greater than `empty_idx`.
/// - The range `[bubble_base, empty_idx)` must be initialized.
/// - Caller must ensure `0 <= bubble_base < empty_idx <= hashes.len()`.
/// - `max_root_mask` must match the table’s current mask; roots are
///   `0..=max_root_mask` and map to absolute indices as `root*16`.
#[inline(always)]
unsafe fn find_next_movable_index(
    hashes: &[MaybeUninit<u64>],
    bubble_base: usize,
    empty_idx: usize,
    max_root_mask: usize,
) -> Option<usize> {
    for idx in bubble_base..empty_idx {
        // SAFETY: We have validated that `idx` is within the `bubble_base..empty_idx`
        // range. The caller guarantees `empty_idx <= hashes.len()`, ensuring
        // `get_unchecked` is safe. Additionally, the caller ensures elements in
        // `[bubble_base, empty_idx)` are initialized, making `assume_init_read`
        // safe.
        unsafe {
            let hash = hashes.get_unchecked(idx).assume_init_read();
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
            // Note - sizes _must_ be power-of-two plus HOP_RANGE to ensure nothing ends up
            // reading out OOB since we don't do wrapping, and computing the root buckets
            // relies on this being power-of-two for masking to work. Yes using & instead of
            // modulo makes a difference for performance.
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
        // SAFETY: We have ensured that `HopInfo` is `#[repr(C, align(16))]`,
        // with `neighbors` at offset 0. This guarantees 16-byte alignment,
        // making it safe to load via `_mm_load_si128`.
        unsafe {
            let data = _mm_load_si128(self.neighbors.as_ptr() as *const __m128i);
            let cmp = _mm_cmpgt_epi8(data, _mm_setzero_si128());
            _mm_movemask_epi8(cmp) as u16
        }
    }

    #[inline(always)]
    fn is_full(&self) -> bool {
        self.candidates() == 0xFFFF
    }

    /// Clear neighbor count at the given index
    ///
    /// # Safety
    ///
    /// The caller must ensure `n_index` is within the bounds of the neighbors
    /// array (less than `HOP_RANGE`).
    #[inline(always)]
    unsafe fn clear(&mut self, n_index: usize) {
        // SAFETY: Caller ensures `n_index` is within bounds of the neighbors array
        unsafe {
            debug_assert!(self.neighbors.get_unchecked(n_index) > &0);
            *self.neighbors.get_unchecked_mut(n_index) -= 1;
        }
    }

    /// Set neighbor count at the given index
    ///
    /// # Safety
    ///
    /// The caller must ensure `n_index` is within the bounds of the neighbors
    /// array (less than `HOP_RANGE`).
    #[inline(always)]
    unsafe fn set(&mut self, n_index: usize) {
        // SAFETY: Caller ensures `n_index` is within bounds of the neighbors array
        unsafe {
            debug_assert!(self.neighbors.get_unchecked(n_index) < &16);
            *self.neighbors.get_unchecked_mut(n_index) += 1;
        }
    }
}

#[derive(Debug)]
struct DataLayout {
    layout: Layout,
    hopmap_offset: usize,
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

        let (layout, hopmap_offset) = Layout::new::<()>().extend(hopmap_layout).unwrap();
        let (layout, tags_offset) = layout.extend(tags_layout).unwrap();
        let (layout, buckets_offset) = layout.extend(buckets_layout).unwrap();
        let (layout, hashes_offset) = layout.extend(hashes_layout).unwrap();

        DataLayout {
            layout,
            hopmap_offset,
            tags_offset,
            buckets_offset,
            hashes_offset,
        }
    }
}

/// Debug statistics for hash table analysis.
///
/// Test-only: compiled only with `cfg(test)`.
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct DebugStats {
    /// Number of elements currently in the table
    pub populated: usize,
    /// Maximum load capacity before resize
    pub capacity: usize,
    /// Total number of slots allocated
    pub total_slots: usize,
    /// Number of slots currently occupied
    pub occupied_slots: usize,
    /// Number of entries in overflow storage
    pub overflow_entries: usize,
    /// Load factor (populated / capacity)
    pub load_factor: f64,
    /// Slot utilization (occupied_slots / total_slots)
    pub slot_utilization: f64,
    /// Total memory in bytes used by the table
    pub total_bytes: usize,
    /// Estimated wasted memory in bytes
    pub wasted_bytes: usize,
}

#[cfg(test)]
impl DebugStats {
    /// Pretty-print the debug statistics.
    #[cfg(feature = "std")]
    pub fn print(&self) {
        println!("=== Hash Table Debug Statistics ===");
        println!(
            "Population: {}/{} ({:.2}% load factor)",
            self.populated,
            self.capacity,
            self.load_factor * 100.0
        );
        println!(
            "Slot Usage: {}/{} ({:.2}% utilization)",
            self.occupied_slots,
            self.total_slots,
            self.slot_utilization * 100.0
        );
        println!("Overflow: {} entries", self.overflow_entries);
        println!("Total Allocated: {} bytes", self.total_bytes);
        println!(
            "Memory: {} bytes wasted ({:.02}%)",
            self.wasted_bytes,
            if self.total_bytes == 0 {
                0.0
            } else {
                (self.wasted_bytes as f64 / self.total_bytes as f64) * 100.0
            }
        );
    }
}

/// A high-performance hash table using 16-way hopscotch hashing.
///
/// `HashTable<V>` stores values of type `V` and provides fast insertion,
/// lookup, and removal operations. Unlike standard hash maps, this
/// implementation requires you to provide both the hash value and an equality
/// predicate for each operation.
///
/// ## Performance Characteristics
///
/// - **Memory**: 2 bytes per entry overhead, plus the size of `V` plus a u64
///   for the hash.
///
/// ## Example
///
/// ```rust
/// # use core::hash::Hash;
/// # use core::hash::Hasher;
/// #
/// # use hop_hash::hash_table::HashTable;
/// # use siphasher::sip::SipHasher;
/// #
/// # #[derive(Debug, PartialEq)]
/// # struct Person {
/// #     id: u64,
/// #     name: String,
/// # }
/// #
/// # fn hash_id(id: u64) -> u64 {
/// #     let mut hasher = SipHasher::new();
/// #     id.hash(&mut hasher);
/// #     hasher.finish()
/// # }
///
/// let mut table = HashTable::with_capacity(100);
/// let hash = hash_id(123);
///
/// // Insert a person
/// match table.entry(hash, |p: &Person| p.id == 123) {
///     hop_hash::hash_table::Entry::Vacant(entry) => {
///         entry.insert(Person {
///             id: 123,
///             name: "Alice".to_string(),
///         });
///     }
///     hop_hash::hash_table::Entry::Occupied(_) => {
///         println!("Person already exists");
///     }
/// }
/// ```
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
        use alloc::string::ToString;

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
                                if *b == EMPTY {
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

impl<V> Clone for HashTable<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        let mut new_table = Self::with_capacity(self.max_pop);

        // SAFETY: We have ensured that both tables have valid allocations and
        // the same capacity.
        unsafe {
            let src_buckets = self.buckets_ptr().as_ref();
            let dst_buckets = new_table.buckets_ptr().as_mut();
            let src_hashes = self.hashes_ptr().as_ref();
            let dst_hashes = new_table.hashes_ptr().as_mut();
            let src_tags = self.tags_ptr().as_ref();
            let dst_tags = new_table.tags_ptr().as_mut();

            for i in 0..src_tags.len() {
                let tag = *src_tags.get_unchecked(i);
                *dst_tags.get_unchecked_mut(i) = tag;

                if tag != EMPTY {
                    let value = src_buckets.get_unchecked(i).assume_init_ref().clone();
                    let hash = src_hashes.get_unchecked(i).assume_init_read();
                    *dst_buckets.get_unchecked_mut(i) = MaybeUninit::new(value);
                    *dst_hashes.get_unchecked_mut(i) = MaybeUninit::new(hash);
                    new_table.populated += 1;

                    let hop_bucket = (hash as usize & new_table.max_root_mask) * 16;
                    let offset = i - hop_bucket;
                    let n_index = offset / 16;
                    // SAFETY: We have validated that `i` is a valid slot index from
                    // `hop_bucket` is also valid, and `n_index` is derived from
                    // `offset` which is guaranteed to be less than `HOP_RANGE * 16`
                    new_table
                        .hopmap_ptr()
                        .as_mut()
                        .get_unchecked_mut(hop_bucket / 16)
                        .set(n_index);

                    debug_assert!(new_table.populated <= new_table.max_pop);
                }
            }
            new_table.overflow = self.overflow.clone();

            debug_assert!(new_table.populated == self.populated);

            new_table
        }
    }
}

impl<V> Drop for HashTable<V> {
    fn drop(&mut self) {
        // SAFETY: We validate that values are properly initialized before being
        // dropped. We also validate that we have a valid allocation before
        // deallocating.
        unsafe {
            if core::mem::needs_drop::<V>() && self.populated > 0 {
                for (index, tag) in self.tags_ptr().as_ref().iter().enumerate() {
                    if *tag != EMPTY {
                        self.buckets_ptr()
                            .as_mut()
                            .get_unchecked_mut(index)
                            .assume_init_drop();
                    }
                }
            }

            if self.layout.layout.size() != 0 {
                alloc::alloc::dealloc(self.alloc.as_ptr(), self.layout.layout);
            }
        }
    }
}

impl<V> HashTable<V> {
    /// Creates a new hash table with the specified capacity.
    ///
    /// The actual capacity may be larger than requested due to the bucket-based
    /// organization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// // Create a table that can hold at least 100 items without resizing
    /// let table: HashTable<String> = HashTable::with_capacity(100);
    /// assert!(table.capacity() >= 100);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity: Capacity = target_load_factor_inverse(capacity.div_ceil(16)).into();

        let layout = DataLayout::new::<V>(capacity);
        let alloc = if layout.layout.size() == 0 {
            NonNull::dangling()
        } else {
            // SAFETY: We have validated that the layout size is non-zero. The `alloc`
            // function returns a valid pointer, and we handle allocation errors
            // if it returns null.
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
            max_pop: target_load_factor(capacity.max_root_mask().wrapping_add(1) * 16),
            max_root_mask: capacity.max_root_mask(),
            _phantom: core::marker::PhantomData,
        }
    }

    fn hopmap_ptr(&self) -> NonNull<[HopInfo]> {
        // SAFETY: Allocation is valid and properly sized for the hopmap slice
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.hopmap_offset).cast(),
                self.max_root_mask.wrapping_add(1),
            )
        }
    }

    fn buckets_ptr(&self) -> NonNull<[MaybeUninit<V>]> {
        // SAFETY: Allocation is valid and properly sized for the buckets slice
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.buckets_offset).cast(),
                if self.layout.layout.size() == 0 {
                    0
                } else {
                    (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * 16
                },
            )
        }
    }

    fn hashes_ptr(&self) -> NonNull<[MaybeUninit<u64>]> {
        // SAFETY: Allocation is valid and properly sized for the hashes slice
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.hashes_offset).cast(),
                if self.layout.layout.size() == 0 {
                    0
                } else {
                    (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * 16
                },
            )
        }
    }

    fn tags_ptr(&self) -> NonNull<[u8]> {
        // SAFETY: Allocation is valid and properly sized for the tags slice
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.tags_offset).cast(),
                if self.layout.layout.size() == 0 {
                    0
                } else {
                    (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * 16
                },
            )
        }
    }

    /// Returns an iterator over all values in the table.
    ///
    /// The iterator yields `&V` references in an arbitrary order.
    /// The iteration order is not specified and may change between versions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// table
    ///     .entry(hash_str("key1"), |s: &String| s == "key1")
    ///     .or_insert("key1".to_string());
    /// table
    ///     .entry(hash_str("key2"), |s: &String| s == "key2")
    ///     .or_insert("key1".to_string());
    ///
    /// for value in table.iter() {
    ///     println!("Value: {}", value);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<'_, V> {
        Iter {
            table: self,
            bucket_index: 0,
            overflow_index: 0,
        }
    }

    /// Returns an iterator that removes and yields all values from the table.
    ///
    /// After calling `drain()`, the table will be empty. The iterator yields
    /// owned values in an arbitrary order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// table
    ///     .entry(hash_str("key1"), |s: &String| s == "key1")
    ///     .or_insert("key1".to_string());
    ///
    /// let values: Vec<String> = table.drain().collect();
    /// assert!(table.is_empty());
    /// assert_eq!(values.len(), 1);
    /// ```
    pub fn drain(&mut self) -> Drain<'_, V> {
        Drain {
            table: self,
            bucket_index: 0,
        }
    }

    /// Returns `true` if the table contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// let table: HashTable<i32> = HashTable::with_capacity(10);
    /// assert!(table.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.populated == 0
    }

    /// Returns the number of elements in the table.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # fn hash_u64(n: u64) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     n.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// assert_eq!(table.len(), 0);
    ///
    /// table.entry(hash_u64(1), |&n: &u64| n == 1).or_insert(1);
    /// assert_eq!(table.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.populated
    }

    /// Removes all elements from the table.
    ///
    /// This operation preserves the table's allocated capacity. All values are
    /// properly dropped if they implement `Drop`. After calling `clear()`, the
    /// table will be empty but maintain its current capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # fn hash_u64(n: u64) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     n.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// table.entry(hash_u64(1), |&n: &u64| n == 1).or_insert(1);
    /// table.entry(hash_u64(2), |&n: &u64| n == 2).or_insert(2);
    /// assert_eq!(table.len(), 2);
    ///
    /// table.clear();
    /// assert_eq!(table.len(), 0);
    /// assert!(table.is_empty());
    /// ```
    pub fn clear(&mut self) {
        // SAFETY: We have ensured that values are properly initialized before being
        // dropped.
        unsafe {
            if core::mem::needs_drop::<V>() && self.populated > 0 {
                for (index, tag) in self.tags_ptr().as_ref().iter().enumerate() {
                    if *tag != EMPTY {
                        self.buckets_ptr()
                            .as_mut()
                            .get_unchecked_mut(index)
                            .assume_init_drop();
                    }
                }
            }

            if self.layout.layout.size() != 0 {
                core::ptr::write_bytes(self.alloc.as_ptr(), 0x0, self.layout.tags_offset);
                core::ptr::write_bytes(
                    self.alloc.as_ptr().add(self.layout.tags_offset),
                    EMPTY,
                    self.layout.buckets_offset - self.layout.tags_offset,
                );
            }
        }

        self.populated = 0;
        self.overflow.clear();
    }

    /// Shrinks the capacity of the hash table as much as possible.
    ///
    /// This method will shrink the table's capacity to just fit the current
    /// number of elements, potentially freeing up significant amounts of
    /// memory.
    ///
    /// If the table is empty, it will be completely deallocated and reset to
    /// a zero-capacity state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use hop_hash::HashTable;
    ///
    /// let mut table: HashTable<i32> = HashTable::with_capacity(1000);
    /// assert!(table.capacity() >= 1000);
    ///
    /// // Add a few elements
    /// table.entry(42, |&v| v == 5).or_insert(5);
    /// table.entry(123, |&v| v == 10).or_insert(10);
    ///
    /// // Shrink to fit
    /// table.shrink_to_fit();
    /// assert!(table.capacity() < 1000);
    /// assert!(table.capacity() >= 2);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        if self.populated == 0 && self.overflow.is_empty() {
            if self.layout.layout.size() != 0 {
                // SAFETY: We have ensured that the allocation is valid before
                // deallocating.
                unsafe {
                    alloc::alloc::dealloc(self.alloc.as_ptr(), self.layout.layout);
                }
                self.alloc = NonNull::dangling();
                let new_capacity: Capacity = 0.into();
                self.layout = DataLayout::new::<V>(new_capacity);
                self.max_root_mask = new_capacity.max_root_mask();
                self.max_pop = 0;
            }
            return;
        }

        let required = self.populated + self.overflow.len();
        let new_capacity: Capacity = target_load_factor_inverse(required.div_ceil(16)).into();
        if new_capacity.max_root_mask() < self.max_root_mask {
            self.do_resize_rehash(new_capacity);
        }
    }

    /// Reserves capacity for at least `additional` more elements.
    ///
    /// The collection may reserve more space to speculatively avoid frequent
    /// reallocations. After calling `reserve`, capacity will be greater than or
    /// equal to `self.len() + additional`. Does nothing if capacity is already
    /// sufficient.
    ///
    /// # Arguments
    ///
    /// * `additional` - The number of additional elements the table should be
    ///   able to hold
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// let mut table: HashTable<i32> = HashTable::with_capacity(15);
    /// for i in 0..15 {
    ///     table.entry(i as u64, |&n: &i32| n == i).or_insert(i);
    /// }
    /// let original_capacity = table.capacity();
    ///
    /// // Reserve space for 50 more elements
    /// table.reserve(50);
    /// assert!(table.capacity() >= original_capacity + 50);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        let required = self.populated.saturating_add(additional);
        if required > self.max_pop {
            let new_capacity: Capacity = target_load_factor_inverse(required.div_ceil(16)).into();
            self.do_resize_rehash(new_capacity);
        }
    }

    /// Removes and returns a value from the table.
    ///
    /// The value is identified by its hash and an equality predicate. If the
    /// value is found, it is removed from the table and returned.
    /// Otherwise, `None` is returned.
    ///
    /// # Arguments
    ///
    /// * `hash` - The hash value of the entry to remove
    /// * `eq` - A predicate function that returns `true` for the value to
    ///   remove
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # fn hash_u64(n: u64) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     n.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// table.entry(hash_u64(42), |&n: &u64| n == 42).or_insert(42);
    ///
    /// let removed = table.remove(hash_u64(42), |&n| n == 42);
    /// assert_eq!(removed, Some(42));
    /// assert!(table.is_empty());
    ///
    /// // Removing non-existent value returns None
    /// let not_found = table.remove(hash_u64(99), |&n| n == 99);
    /// assert_eq!(not_found, None);
    /// ```
    pub fn remove(&mut self, hash: u64, eq: impl Fn(&V) -> bool) -> Option<V> {
        if self.populated == 0 {
            return None;
        }

        let hop_bucket = self.hopmap_index(hash);
        // SAFETY: We have validated that `hop_bucket` is within bounds through
        // `hopmap_index`, which derives it from the hash and `max_root_mask`.
        let index = unsafe { self.search_neighborhood(hash, hop_bucket, &eq) };
        if let Some(index) = index {
            self.populated -= 1;

            // SAFETY: We have validated that `index` is within bounds through
            // `search_neighborhood`.
            let bucket_mut = unsafe { self.buckets_ptr().as_ref().get_unchecked(index) };
            // SAFETY: We have confirmed that the value at this index is initialized due to
            // an occupied tag.
            let value = unsafe { bucket_mut.assume_init_read() };

            let offset = index - hop_bucket * 16;
            let n_index = offset / 16;
            // SAFETY: We have validated that `index` is a valid slot index from
            // `search_neighborhood`, `hop_bucket` is also valid, and `n_index`
            // is derived from these, ensuring it is a valid neighbor index.
            unsafe {
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

    /// Gets an entry for the given hash and equality predicate.
    ///
    /// This method returns an `Entry` enum that allows for efficient insertion
    /// or modification of values. The entry API provides zero-cost abstractions
    /// for common patterns like "insert if not exists" or "update if exists".
    ///
    /// # Arguments
    ///
    /// * `hash` - The hash value for the entry
    /// * `eq` - A predicate function that returns `true` for matching values
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// let hash = hash_str("hello");
    ///
    /// // Insert or update pattern
    /// match table.entry(hash, |s: &String| s == "hello") {
    ///     hop_hash::hash_table::Entry::Vacant(entry) => {
    ///         entry.insert("world".to_string());
    ///     }
    ///     hop_hash::hash_table::Entry::Occupied(mut entry) => {
    ///         *entry.get_mut() = "updated".to_string();
    ///     }
    /// }
    ///
    /// // Or use the convenience method
    /// table
    ///     .entry(hash, |s: &String| s == "hello")
    ///     .or_insert("hello".to_string());
    /// ```
    #[inline(always)]
    pub fn entry(&mut self, hash: u64, eq: impl Fn(&V) -> bool) -> Entry<'_, V> {
        self.maybe_resize_rehash();
        self.entry_impl(hash, eq)
    }

    /// Search the neighborhood of a given bucket for a matching value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `bucket` is within the valid range of
    /// buckets, derived from the hash and `max_root_mask`.
    #[inline]
    unsafe fn search_neighborhood(
        &self,
        hash: u64,
        bucket: usize,
        eq: impl Fn(&V) -> bool,
    ) -> Option<usize> {
        // SAFETY: Caller ensures that `bucket` is within bounds, as it is derived from
        // the hash and `max_root_mask`.
        unsafe {
            prefetch(self.tags_ptr().as_ref().as_ptr().add(bucket * 16));
            prefetch(self.buckets_ptr().as_ref().as_ptr().add(bucket * 16));
        };

        // SAFETY: Caller ensures that `bucket` is within bounds, as it is derived from
        // the hash and `max_root_mask`.
        let mut neighborhood_mask = unsafe {
            self.hopmap_ptr()
                .as_ref()
                .get_unchecked(bucket)
                .candidates()
        };

        let tag = hashtag(hash);
        while neighborhood_mask != 0 {
            let index = neighborhood_mask.trailing_zeros() as usize;
            neighborhood_mask &= !(1 << index);

            let base = bucket * 16 + index * 16;
            // SAFETY: We have ensured `base` is valid, calculated from a validated bucket
            // and an index within the neighborhood.
            let tags = unsafe { self.scan_tags(base, tag) };
            if tags == 0 {
                continue;
            }

            for idx in 0..16 {
                if tags & (1 << idx) == 0 {
                    continue;
                }
                let slot = base + idx;

                // SAFETY: We have ensured `slot` is within bounds, as it is calculated from a
                // validated base and index.
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
    ///
    /// The caller must ensure `bucket` is within a valid range, such that
    /// `bucket + 16` does not exceed the bounds of the tags array.
    #[inline(always)]
    unsafe fn scan_tags(&self, bucket: usize, tag: u8) -> u16 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        {
            // SAFETY: We have validated the bucket bounds, as per the requirements of
            // `scan_tags`.
            return unsafe { self.scan_tags_sse2(bucket, tag) };
        }

        #[allow(unused_variables, unreachable_code)]
        {
            let meta_ptr = self.tags_ptr();
            let mut tags: u16 = 0;
            for i in 0..16 {
                // SAFETY: We have ensured `bucket + i` is within bounds, as `bucket` is a valid
                // base for `scan_tags`.
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
    ///
    /// The caller must ensure `bucket` is within a valid range, such that
    /// `bucket + 16` does not exceed the bounds of the tags array. This
    /// relies on `EMPTY` (0x80) using the sign bit for complementary SIMD
    /// scans.
    #[inline(always)]
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    unsafe fn scan_tags_sse2(&self, bucket: usize, tag: u8) -> u16 {
        use core::arch::x86_64::*;
        // SAFETY: We have validated that `bucket` is within bounds, allowing for a safe
        // load of 16 consecutive bytes.
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

        // SAFETY: We have ensured that `hop_bucket` is within bounds, as it is derived
        // from the hash and mask.
        let index = unsafe { self.search_neighborhood(hash, hop_bucket, &eq) };
        if let Some(index) = index {
            return Entry::Occupied(OccupiedEntry {
                n_index: index - hop_bucket * 16,
                table: self,
                root_index: hop_bucket,
                overflow_index: None,
            });
        }

        if !self.overflow.is_empty() {
            #[cold]
            #[inline(never)]
            fn search_overflow<V>(
                overflow: &[(u64, V)],
                eq: &impl Fn(&V) -> bool,
            ) -> Option<usize> {
                for (idx, (_, overflow)) in overflow.iter().enumerate() {
                    if eq(overflow) {
                        return Some(idx);
                    }
                }
                None
            }
            if let Some(overflow_index) = search_overflow(&self.overflow, &eq) {
                return Entry::Occupied(OccupiedEntry {
                    n_index: 0,
                    table: self,
                    root_index: hop_bucket,
                    overflow_index: Some(overflow_index),
                });
            }
        }

        // SAFETY: We have ensured `hop_bucket` is within bounds, as it is derived from
        // the hash and mask.
        Entry::Vacant(unsafe { self.do_vacant_lookup(hash, hop_bucket) })
    }

    /// Perform a vacant lookup, finding or creating a suitable slot for
    /// insertion
    ///
    /// # Safety
    ///
    /// The caller must ensure that `hop_bucket` is within the bounds of the
    /// hopmap array.
    unsafe fn do_vacant_lookup(&mut self, hash: u64, hop_bucket: usize) -> VacantEntry<'_, V> {
        debug_assert!(hop_bucket <= self.max_root_mask);
        let empty_idx = unsafe { self.find_next_unoccupied(self.absolute_index(hop_bucket, 0)) };

        if empty_idx.is_none()
            || empty_idx.unwrap() >= self.absolute_index(self.max_root_mask + 1 + HOP_RANGE, 0)
        {
            self.resize_rehash();
            // SAFETY: We have ensured `hop_bucket` is within the hopmap bounds.
            return unsafe { self.do_vacant_lookup(hash, self.hopmap_index(hash)) };
        }

        let mut absolute_empty_idx = empty_idx.unwrap();
        // SAFETY: We have validated `absolute_empty_idx` through
        // `find_next_unoccupied`.
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

            // SAFETY: We have ensured that `bubble_base` and `absolute_empty_idx` are
            // within the table bounds.
            if let Some(absolute_idx) = unsafe {
                find_next_movable_index(
                    self.hashes_ptr().as_ref(),
                    bubble_base,
                    absolute_empty_idx,
                    self.max_root_mask,
                )
            } {
                // SAFETY: We have validated `absolute_idx` through `find_next_movable_index`,
                // ensuring it is within bounds.
                unsafe {
                    let moved_hash = self
                        .hashes_ptr()
                        .as_ref()
                        .get_unchecked(absolute_idx)
                        .assume_init_read();

                    let buckets_ptr = self.buckets_ptr().as_mut().as_mut_ptr();
                    debug_assert_ne!(absolute_idx, absolute_empty_idx);

                    core::ptr::copy_nonoverlapping(
                        buckets_ptr.add(absolute_idx),
                        buckets_ptr.add(absolute_empty_idx),
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

                    // SAFETY: We have ensured through `find_next_movable_index` that the moved
                    // element is within the hop-neighborhood of its
                    // `hopmap_root`. `absolute_empty_idx` is also within this
                    // neighborhood, making `old_n_index` and `new_n_index` valid neighbor indices.
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
                // SAFETY: We have ensured `hop_bucket` is within hopmap bounds.
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

                self.resize_rehash();
                // SAFETY: We have ensured `hop_bucket` is within the hopmap bounds.
                return unsafe { self.do_vacant_lookup(hash, self.hopmap_index(hash)) };
            }
        }

        // SAFETY: We have validated `absolute_empty_idx` through
        // `find_next_unoccupied`.
        debug_assert!(unsafe { !self.is_occupied(absolute_empty_idx) });
        VacantEntry {
            n_index: absolute_empty_idx - hop_bucket * 16,
            table: self,
            hopmap_root: hop_bucket,
            hash,
            is_overflow: false,
        }
    }

    /// Check if the slot at index is occupied
    ///
    /// # Safety
    ///
    /// The caller must ensure `index` is within the bounds of the tags array.
    #[inline(always)]
    unsafe fn is_occupied(&self, index: usize) -> bool {
        // SAFETY: Caller ensures `index` is within bounds of the tags array
        unsafe { *self.tags_ptr().as_ref().get_unchecked(index) != EMPTY }
    }

    /// Clear the occupied tag at index
    ///
    /// # Safety
    ///
    /// The caller must ensure `index` is within the bounds of the tags array.
    #[inline(always)]
    unsafe fn clear_occupied(&mut self, index: usize) {
        // SAFETY: Caller ensures `index` is within bounds of the tags array
        unsafe {
            *self.tags_ptr().as_mut().get_unchecked_mut(index) = EMPTY;
        }
    }

    /// Set the occupied tag at index
    ///
    /// # Safety
    ///
    /// The caller must ensure `index` is within the bounds of the tags array
    /// and that `tag` is a valid tag (not `EMPTY`).
    #[inline(always)]
    unsafe fn set_occupied(&mut self, index: usize, tag: u8) {
        // SAFETY: Caller ensures `index` is within bounds of the tags array
        unsafe {
            debug_assert!(tag != EMPTY);
            *self.tags_ptr().as_mut().get_unchecked_mut(index) = tag;
        }
    }

    /// Find the next unoccupied index starting from `start`
    ///
    /// # Safety
    ///
    /// The caller must ensure `start` is within the bounds of the tags array.
    #[inline(always)]
    unsafe fn find_next_unoccupied(&self, start: usize) -> Option<usize> {
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

    /// SSE2 optimized version of find_next_unoccupied
    ///
    /// # Safety
    ///
    /// The caller must ensure `start` is within the bounds of the tags array.
    /// This relies on `EMPTY` (0x80) having the sign bit set for `movemask`
    /// to find empty slots. Unaligned loads are performed but guarded by
    /// bounds checks.
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    #[inline(always)]
    unsafe fn find_next_unoccupied_sse2(&self, start: usize) -> Option<usize> {
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

    /// Finds a value in the table by hash and equality predicate.
    ///
    /// Returns a reference to the value if found, or `None` if no matching
    /// value exists. This method does not modify the table and can be
    /// called on shared references.
    ///
    /// # Arguments
    ///
    /// * `hash` - The hash value to search for
    /// * `eq` - A predicate function that returns `true` for the desired value
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # fn hash_u64(n: u64) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     n.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// table.entry(hash_u64(42), |&n: &u64| n == 42).or_insert(42);
    ///
    /// // Find existing value
    /// let found = table.find(hash_u64(42), |&n| n == 42);
    /// assert_eq!(found, Some(&42));
    ///
    /// // Search for non-existent value
    /// let not_found = table.find(hash_u64(99), |&n| n == 99);
    /// assert_eq!(not_found, None);
    /// ```
    #[inline]
    pub fn find(&self, hash: u64, eq: impl Fn(&V) -> bool) -> Option<&V> {
        if self.populated == 0 {
            return None;
        }

        let bucket = self.hopmap_index(hash);
        // SAFETY: We have ensured that `bucket` is within bounds through
        // `hopmap_index`, which derives it from the hash and `max_root_mask`.
        let index = unsafe { self.search_neighborhood(hash, bucket, &eq) };
        if let Some(index) = index {
            // SAFETY: We have validated `index` through `search_neighborhood`, and the
            // bucket is confirmed to be initialized by an occupied tag.
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

        self.find_overflow(eq)
    }

    #[cold]
    #[inline(never)]
    fn find_overflow(&self, eq: impl Fn(&V) -> bool) -> Option<&V> {
        self.overflow
            .iter()
            .map(|(_, overflow)| overflow)
            .find(|&overflow| eq(overflow))
    }

    /// Finds a value in the table by hash and equality predicate, returning a
    /// mutable reference.
    ///
    /// Returns a mutable reference to the value if found, or `None` if no
    /// matching value exists. This method allows modification of values
    /// in-place without removing and re-inserting them.
    ///
    /// # Arguments
    ///
    /// * `hash` - The hash value to search for
    /// * `eq` - A predicate function that returns `true` for the desired value
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # fn hash_u64(n: u64) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     n.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// table.entry(hash_u64(42), |&n: &u64| n == 42).or_insert(42);
    ///
    /// // Find and modify existing value
    /// if let Some(value) = table.find_mut(hash_u64(42), |&n| n == 42) {
    ///     *value = 100;
    /// }
    ///
    /// let found = table.find(hash_u64(42), |&n| n == 100);
    /// assert_eq!(found, Some(&100));
    ///
    /// // Search for non-existent value
    /// let not_found = table.find_mut(hash_u64(99), |&n| n == 99);
    /// assert_eq!(not_found, None);
    /// ```
    #[inline]
    pub fn find_mut(&mut self, hash: u64, eq: impl Fn(&V) -> bool) -> Option<&mut V> {
        if self.populated == 0 {
            return None;
        }

        let bucket = self.hopmap_index(hash);

        // SAFETY: We have ensured that `bucket` is within bounds through
        // `hopmap_index`, which derives it from the hash and `max_root_mask`.
        if let Some(index) = unsafe { self.search_neighborhood(hash, bucket, &eq) } {
            // SAFETY: We have validated `index` through `search_neighborhood`, and the
            // bucket is confirmed to be initialized by an occupied tag.
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

        self.find_overflow_mut(eq)
    }

    #[cold]
    #[inline(never)]
    fn find_overflow_mut(&mut self, eq: impl Fn(&V) -> bool) -> Option<&mut V> {
        self.overflow
            .iter_mut()
            .map(|(_, overflow)| overflow)
            .find(|overflow| eq(overflow))
    }

    #[inline]
    fn maybe_resize_rehash(&mut self) {
        if self.populated >= self.max_pop {
            self.resize_rehash();
        }
    }

    #[inline]
    #[cold]
    fn resize_rehash(&mut self) {
        let capacity = self.max_root_mask.wrapping_add(1).max(HOP_RANGE) + 1;
        let capacity: Capacity = capacity.into();

        self.do_resize_rehash(capacity);
    }

    #[inline]
    fn do_resize_rehash(&mut self, capacity: Capacity) {
        debug_assert!(
            capacity.max_root_mask() != self.max_root_mask || self.max_root_mask == usize::MAX
        );

        let new_layout = DataLayout::new::<V>(capacity);
        // SAFETY: layout.layout validated non-zero size, alloc failure handled
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
        self.max_pop = target_load_factor(capacity.max_root_mask().wrapping_add(1) * 16);
        self.max_root_mask = capacity.max_root_mask();
        if self.populated == 0 {
            // SAFETY: old_layout.layout.size() checked non-zero, old_alloc from valid
            // allocation
            unsafe {
                if old_layout.layout.size() != 0 {
                    alloc::alloc::dealloc(old_alloc.as_ptr(), old_layout.layout);
                }
            }

            return;
        }
        let overflows = core::mem::take(&mut self.overflow);
        // SAFETY: old_alloc valid, old_empty_words calculated from valid old capacity
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

        // SAFETY: Moving initialized values from old table, pointers valid from
        // allocation
        unsafe {
            // Ownership note: We move values (V) and hashes (u64) out of the old
            // allocation into the new one. The old allocation is then deallocated without
            // running destructors for moved-out contents; only the new table will drop
            // values.
            self.populated = 0;

            let mut pending_indexes = Vec::with_capacity(old_max_root * 16);
            for (bucket_index, &tag) in old_emptymap.as_ref().iter().enumerate() {
                if tag == EMPTY {
                    continue;
                }

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
                // SAFETY: `absolute_empty_idx` is guaranteed to be within the hop-neighborhood
                // of `bucket` by the check before this block. Therefore, `n_index` is a
                // valid neighbor index (< HOP_RANGE).
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

    /// Returns the current capacity of the table.
    ///
    /// The capacity represents the maximum number of elements the table can
    /// hold before it needs to resize. Due to the hopscotch hashing
    /// algorithm and bucket-based organization, the actual capacity may be
    /// larger than what was initially requested.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// let table: HashTable<i32> = HashTable::with_capacity(100);
    /// println!("Table can hold {} elements", table.capacity());
    /// assert!(table.capacity() >= 100);
    /// ```
    ///
    /// # Load Factor
    ///
    /// The table maintains a load factor of approximately 99% before
    /// triggering a resize operation.
    pub fn capacity(&self) -> usize {
        self.max_pop
    }

    /// Computes a histogram of probe lengths for the current table state.
    ///
    /// Test-only: compiled only with `cfg(test)`.
    ///
    /// Definition of probe length used here:
    /// - For each occupied slot in the table, we compute the distance from its
    ///   root bucket as `n_index = (absolute_index - root*16) / 16`, which is
    ///   always in `0..HOP_RANGE` for in-table entries. This corresponds to the
    ///   hop-neighborhood index tracked by the hopmap.
    /// - All overflowed entries (stored in the overflow vector) are counted in
    ///   an extra bin at index `HOP_RANGE`.
    ///
    /// Returns a vector of length `HOP_RANGE + 1` where indices
    /// `0..HOP_RANGE-1` represent in-table probe lengths, and index
    /// `HOP_RANGE` represents overflow entries.
    #[cfg(test)]
    pub fn probe_histogram(&self) -> alloc::vec::Vec<usize> {
        let mut hist = alloc::vec![0usize; HOP_RANGE + 1];

        if self.populated == 0 {
            return hist;
        }

        // SAFETY: All pointer/slice accesses obey table bounds; occupied slots
        // are identified by tags, and corresponding hashes are initialized.
        unsafe {
            for bucket in self.hopmap_ptr().as_ref().iter() {
                let mask = bucket.candidates();
                if mask != 0 {
                    hist[mask.count_ones() as usize - 1] += bucket
                        .neighbors
                        .iter()
                        .copied()
                        .map(|u| u as usize)
                        .sum::<usize>();
                }
            }

            hist[HOP_RANGE] += self.overflow.len();
        }

        hist
    }

    /// Returns detailed performance and utilization statistics for debugging.
    ///
    /// Test-only: compiled only with `cfg(test)`.
    #[cfg(test)]
    pub fn debug_stats(&self) -> DebugStats {
        let total_slots = if self.max_root_mask == usize::MAX {
            0
        } else {
            (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * 16
        };

        let mut occupied_slots = 0;

        if total_slots > 0 {
            // SAFETY: Valid allocation and bounds checked
            unsafe {
                for i in 0..total_slots {
                    if self.is_occupied(i) {
                        occupied_slots += 1;
                    }
                }
            }
        }

        DebugStats {
            populated: self.populated,
            capacity: self.max_pop,
            total_slots,
            occupied_slots,
            overflow_entries: self.overflow.len(),
            load_factor: if self.max_pop == 0 {
                0.0
            } else {
                self.populated as f64 / self.max_pop as f64
            },
            slot_utilization: if total_slots == 0 {
                0.0
            } else {
                occupied_slots as f64 / total_slots as f64
            },
            total_bytes: self.layout.layout.size(),
            wasted_bytes: (total_slots - occupied_slots)
                * (core::mem::size_of::<V>() + core::mem::size_of::<u64>()),
        }
    }

    /// Pretty-prints the probe-length histogram horizontally using stdout.
    ///
    /// Test-only, requires the `std` feature. Produces a horizontal bar chart.
    /// Each row corresponds to a probe-length bin (0..HOP_RANGE-1), plus an
    /// "OF" row for overflows.
    #[cfg(all(test, feature = "std"))]
    pub fn print_probe_histogram(&self) {
        let hist = self.probe_histogram();
        let max = *hist.iter().max().unwrap_or(&0);
        if max == 0 {
            println!("probe histogram: empty");
            return;
        }

        let max_bar = 60usize;
        let total_units = max_bar * 8;
        println!("probe histogram ({} entries):", self.populated);

        let make_bar = |count: usize| -> alloc::string::String {
            if count == 0 || max == 0 {
                return alloc::string::String::new();
            }
            let units = ((count as u128 * total_units as u128).div_ceil(max as u128)) as usize;
            let full = units / 8;
            let rem = units % 8;
            let mut bar = "█".repeat(full);
            if rem > 0 {
                let ch = match rem {
                    1 => '▏',
                    2 => '▎',
                    3 => '▍',
                    4 => '▌',
                    5 => '▋',
                    6 => '▊',
                    7 => '▉',
                    _ => unreachable!(),
                };
                bar.push(ch);
            }
            bar
        };

        for (i, &count) in hist.iter().take(HOP_RANGE).enumerate() {
            let label = alloc::format!("{:>2}", i);
            let bar = make_bar(count);
            println!("{} | {} ({})", label, bar, count);
        }

        let of_count = hist[HOP_RANGE];
        let of_bar = make_bar(of_count);
        println!("OF | {} ({})", of_bar, of_count);
    }
}

/// A view into a single entry in the hash table, which may be vacant or
/// occupied.
///
/// This enum is constructed from the [`entry`] method on [`HashTable`].
/// It provides efficient APIs for insertion and modification operations.
///
/// [`entry`]: HashTable::entry
///
/// # Examples
///
/// ```rust
/// # use core::hash::Hash;
/// # use core::hash::Hasher;
/// #
/// # use hop_hash::hash_table::Entry;
/// # use hop_hash::hash_table::HashTable;
/// # use siphasher::sip::SipHasher;
/// #
/// # fn hash_str(s: &str) -> u64 {
/// #     let mut hasher = SipHasher::new();
/// #     s.hash(&mut hasher);
/// #     hasher.finish()
/// # }
///
/// let mut table = HashTable::with_capacity(10);
/// let hash = hash_str("key");
///
/// match table.entry(hash, |s: &String| s == "key") {
///     Entry::Vacant(entry) => {
///         entry.insert("value".to_string());
///     }
///     Entry::Occupied(entry) => {
///         println!("Key already exists with value: {}", entry.get());
///     }
/// }
/// ```
pub enum Entry<'a, V> {
    /// A vacant entry - the key is not present in the table
    Vacant(VacantEntry<'a, V>),
    /// An occupied entry - the key is present in the table
    Occupied(OccupiedEntry<'a, V>),
}

impl<'a, V> Entry<'a, V> {
    /// Inserts a default value if the entry is vacant and returns a mutable
    /// reference.
    ///
    /// If the entry is occupied, returns a mutable reference to the existing
    /// value. This method provides a convenient way to implement "insert or
    /// get" semantics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// let hash = hash_str("key");
    ///
    /// // Insert if not present
    /// let value = table
    ///     .entry(hash, |s: &String| s == "key")
    ///     .or_insert("key".to_string());
    /// assert_eq!(value, "key");
    ///
    /// // Get existing value
    /// let existing = table
    ///     .entry(hash, |s: &String| s == "key")
    ///     .or_insert("other".to_string());
    /// assert_eq!(existing, "key");
    /// ```
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Inserts a value computed from a closure if the entry is vacant and
    /// returns a mutable reference.
    ///
    /// If the entry is occupied, returns a mutable reference to the existing
    /// value. If the entry is vacant, calls the provided closure to compute
    /// the value and inserts it.
    ///
    /// # Arguments
    ///
    /// * `default` - A closure that returns the value to insert if the entry is
    ///   vacant
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// let hash = hash_str("key");
    ///
    /// // Insert with computed value
    /// let value = table
    ///     .entry(hash, |s: &String| s == "key")
    ///     .or_insert_with(|| "key".to_string());
    /// assert_eq!(value, "key");
    ///
    /// // Get existing value (closure is not called)
    /// let existing = table
    ///     .entry(hash, |s: &String| s == "key")
    ///     .or_insert_with(|| panic!("Should not be called"));
    /// assert_eq!(existing, "key");
    /// ```
    pub fn or_insert_with(self, default: impl FnOnce() -> V) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the table.
    ///
    /// If the entry is occupied, applies the provided closure to the existing
    /// value and returns a mutable reference to it. If the entry is vacant,
    /// returns `None` without inserting anything.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that modifies the existing value
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// # fn hash_u64(n: u64) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     n.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// let hash = hash_u64(42);
    ///
    /// // Entry doesn't exist, so and_modify returns None
    /// let result = table
    ///     .entry(hash, |&n: &u64| n == 42)
    ///     .and_modify(|v| *v += 1);
    /// assert_eq!(result, None);
    ///
    /// // Insert a value
    /// table.entry(hash, |&n: &u64| n == 42).or_insert(42);
    ///
    /// // Now modify the existing value
    /// let result = table
    ///     .entry(hash, |&n: &u64| n == 42)
    ///     .and_modify(|v| *v += 1);
    /// assert_eq!(result, Some(&mut 43));
    /// ```
    ///
    /// This method is useful for implementing "update if exists" semantics
    /// without inserting a default value when the key is not present.
    pub fn and_modify(self, f: impl FnOnce(&mut V)) -> Option<&'a mut V> {
        match self {
            Entry::Occupied(entry) => {
                let value = entry.into_mut();
                f(value);
                Some(value)
            }
            Entry::Vacant(_) => None,
        }
    }

    /// Inserts the default value if the entry is vacant and returns a mutable
    /// reference.
    ///
    /// If the entry is occupied, returns a mutable reference to the existing
    /// value. If the entry is vacant, inserts the default value for type `V`
    /// and returns a mutable reference to it.
    ///
    /// This method requires that `V` implements the `Default` trait.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table: HashTable<Vec<i32>> = HashTable::with_capacity(10);
    /// let hash = hash_str("key");
    ///
    /// // Insert default value (empty Vec)
    /// let vec_ref = table.entry(hash, |v: &Vec<i32>| v.is_empty()).or_default();
    /// vec_ref.push(1);
    /// vec_ref.push(2);
    ///
    /// assert_eq!(
    ///     table.find(hash, |v: &Vec<i32>| !v.is_empty()),
    ///     Some(&vec![1, 2])
    /// );
    /// ```
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(Default::default)
    }
}

/// A view into a vacant entry in the hash table.
///
/// This struct is created by the [`entry`] method on [`HashTable`] when the
/// requested key is not present in the table. It provides methods to insert
/// a value into the vacant slot.
///
/// [`entry`]: HashTable::entry
pub struct VacantEntry<'a, V> {
    table: &'a mut HashTable<V>,
    hopmap_root: usize,
    hash: u64,
    n_index: usize,
    is_overflow: bool,
}

impl<'a, V> VacantEntry<'a, V> {
    /// Inserts a value into the vacant entry and returns a mutable reference to
    /// it.
    ///
    /// The value is inserted at the position determined by the hash and
    /// hopscotch algorithm.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::hash_table::Entry;
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// let hash = hash_str("key");
    ///
    /// match table.entry(hash, |s: &String| s == "key") {
    ///     Entry::Vacant(entry) => {
    ///         let value_ref = entry.insert("value".to_string());
    ///         assert_eq!(value_ref, "value");
    ///     }
    ///     Entry::Occupied(_) => unreachable!("Entry should be vacant"),
    /// }
    /// ```
    pub fn insert(self, value: V) -> &'a mut V {
        self.table.populated += 1;
        if self.is_overflow {
            return self.insert_overflow(value);
        }

        // SAFETY: absolute_index validated during vacant lookup, n_index < HOP_RANGE
        unsafe {
            // SAFETY: `self.n_index` is an offset from the root bucket, guaranteed
            // to be in the hop-neighborhood by `do_vacant_lookup`. Thus, `neighbor`
            // is a valid index (< HOP_RANGE), and `target_index` is a valid,
            // unoccupied slot within bounds.
            let neighbor = self.n_index / 16;
            debug_assert!(neighbor < HOP_RANGE);
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

/// A view into an occupied entry in the hash table.
///
/// This struct is created by the [`entry`] method on [`HashTable`] when the
/// requested key is present in the table. It provides methods to access,
/// modify, or remove the existing value.
///
/// [`entry`]: HashTable::entry
pub struct OccupiedEntry<'a, V> {
    table: &'a mut HashTable<V>,
    root_index: usize,
    n_index: usize,
    overflow_index: Option<usize>,
}

impl<'a, V> OccupiedEntry<'a, V> {
    /// Gets a reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::hash_table::Entry;
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// let hash = hash_str("key");
    /// table
    ///     .entry(hash, |s: &String| s == "key")
    ///     .or_insert("key".to_string());
    ///
    /// match table.entry(hash, |s: &String| s == "key") {
    ///     Entry::Occupied(entry) => {
    ///         assert_eq!(entry.get(), "key");
    ///     }
    ///     Entry::Vacant(_) => unreachable!(),
    /// }
    /// ```
    pub fn get(&self) -> &V {
        if let Some(overflow_index) = self.overflow_index {
            // SAFETY: overflow_index validated when Some, within overflow vec bounds
            return unsafe { &self.table.overflow.get_unchecked(overflow_index).1 };
        }

        // SAFETY: absolute_index validated during lookup, bucket confirmed initialized
        unsafe {
            self.table
                .buckets_ptr()
                .as_ref()
                .get_unchecked(self.root_index * 16 + self.n_index)
                .assume_init_ref()
        }
    }

    /// Gets a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::hash_table::Entry;
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// let hash = hash_str("key");
    /// table
    ///     .entry(hash, |s: &String| s == "key")
    ///     .or_insert("key".to_string());
    ///
    /// match table.entry(hash, |s: &String| s == "key") {
    ///     Entry::Occupied(mut entry) => {
    ///         *entry.get_mut() = "modified".to_string();
    ///     }
    ///     Entry::Vacant(_) => unreachable!(),
    /// }
    /// ```
    pub fn get_mut(&mut self) -> &mut V {
        if let Some(overflow_index) = self.overflow_index {
            // SAFETY: overflow_index validated when Some, within overflow vec bounds
            return unsafe { &mut self.table.overflow.get_unchecked_mut(overflow_index).1 };
        }

        // SAFETY: absolute_index validated during lookup, bucket confirmed initialized
        unsafe {
            self.table
                .buckets_ptr()
                .as_mut()
                .get_unchecked_mut(self.root_index * 16 + self.n_index)
                .assume_init_mut()
        }
    }

    /// Converts the entry into a mutable reference to the value with the
    /// lifetime of the entry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::hash_table::Entry;
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// let hash = hash_str("key");
    /// table
    ///     .entry(hash, |s: &String| s == "key")
    ///     .or_insert("key".to_string());
    ///
    /// let value_ref = match table.entry(hash, |s: &String| s == "key") {
    ///     Entry::Occupied(entry) => entry.into_mut(),
    ///     Entry::Vacant(_) => unreachable!(),
    /// };
    /// *value_ref = "new_value".to_string();
    /// ```
    pub fn into_mut(self) -> &'a mut V {
        if let Some(overflow_index) = self.overflow_index {
            // SAFETY: overflow_index validated when Some, within overflow vec bounds
            return unsafe { &mut self.table.overflow.get_unchecked_mut(overflow_index).1 };
        }

        // SAFETY: absolute_index validated during lookup, bucket confirmed initialized
        unsafe {
            self.table
                .buckets_ptr()
                .as_mut()
                .get_unchecked_mut(self.root_index * 16 + self.n_index)
                .assume_init_mut()
        }
    }

    /// Removes the entry from the table and returns the value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::Hash;
    /// # use core::hash::Hasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::hash_table::Entry;
    /// # use hop_hash::hash_table::HashTable;
    /// #
    /// # fn hash_str(s: &str) -> u64 {
    /// #     let mut hasher = SipHasher::new();
    /// #     s.hash(&mut hasher);
    /// #     hasher.finish()
    /// # }
    /// #
    /// let mut table = HashTable::with_capacity(10);
    /// let hash = hash_str("key");
    /// table
    ///     .entry(hash, |s: &String| s == "key")
    ///     .or_insert("key".to_string());
    ///
    /// let removed_value = match table.entry(hash, |s: &String| s == "key") {
    ///     Entry::Occupied(entry) => entry.remove(),
    ///     Entry::Vacant(_) => unreachable!(),
    /// };
    /// assert_eq!(removed_value, "key");
    /// assert!(table.is_empty());
    /// ```
    pub fn remove(self) -> V {
        self.table.populated -= 1;

        if let Some(overflow_index) = self.overflow_index {
            let (_, value) = self.table.overflow.swap_remove(overflow_index);
            return value;
        }

        // SAFETY: absolute_index validated during lookup, value confirmed initialized
        unsafe {
            let bucket_mut = self
                .table
                .buckets_ptr()
                .as_ref()
                .get_unchecked(self.root_index * 16 + self.n_index);
            let value = bucket_mut.assume_init_read();
            let neighbor = self.n_index / 16;
            // SAFETY: `self.n_index` is the offset from the root bucket, and is
            // guaranteed to be within the hop-neighborhood by `search_neighborhood`.
            // Therefore, `neighbor` will be a valid neighbor index (< HOP_RANGE).
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

/// An iterator over the values in a [`HashTable`].
///
/// This struct is created by the [`iter`] method on [`HashTable`].
/// It yields `&V` references in an arbitrary order.
///
/// [`iter`]: HashTable::iter
///
/// # Examples
///
/// ```rust
/// # use core::hash::Hash;
/// # use core::hash::Hasher;
/// #
/// # use hop_hash::hash_table::HashTable;
/// # use siphasher::sip::SipHasher;
/// #
/// # fn hash_str(s: &str) -> u64 {
/// #     let mut hasher = SipHasher::new();
/// #     s.hash(&mut hasher);
/// #     hasher.finish()
/// # }
/// #
/// let mut table = HashTable::with_capacity(10);
/// table
///     .entry(hash_str("a"), |s: &String| s == "a")
///     .or_insert("1".to_string());
/// table
///     .entry(hash_str("b"), |s: &String| s == "b")
///     .or_insert("2".to_string());
///
/// for value in table.iter() {
///     println!("Value: {}", value);
/// }
/// ```
pub struct Iter<'a, V> {
    table: &'a HashTable<V>,
    bucket_index: usize,
    overflow_index: usize,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        if self.table.populated == 0 {
            return None;
        }

        // SAFETY: slot_index bounds checked, values confirmed initialized by occupied
        // tags
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

/// A draining iterator over the values in a [`HashTable`].
///
/// This struct is created by the [`drain`] method on [`HashTable`].
/// It yields owned `V` values and empties the table as it iterates.
///
/// [`drain`]: HashTable::drain
///
/// # Examples
///
/// ```rust
/// # use core::hash::Hash;
/// # use core::hash::Hasher;
/// #
/// # use hop_hash::hash_table::HashTable;
/// # use siphasher::sip::SipHasher;
/// #
/// # fn hash_str(s: &str) -> u64 {
/// #     let mut hasher = SipHasher::new();
/// #     s.hash(&mut hasher);
/// #     hasher.finish()
/// # }
/// #
/// let mut table = HashTable::with_capacity(10);
/// table
///     .entry(hash_str("a"), |s: &String| s == "a")
///     .or_insert("1".to_string());
/// table
///     .entry(hash_str("b"), |s: &String| s == "b")
///     .or_insert("2".to_string());
///
/// let values: Vec<String> = table.drain().collect();
/// assert!(table.is_empty());
/// assert_eq!(values.len(), 2);
/// ```
pub struct Drain<'a, V> {
    table: &'a mut HashTable<V>,
    bucket_index: usize,
}

impl<V> Drop for Drain<'_, V> {
    fn drop(&mut self) {
        for _ in &mut *self {}

        unsafe {
            core::ptr::write_bytes(
                self.table
                    .alloc
                    .add(self.table.layout.hopmap_offset)
                    .as_ptr(),
                0,
                self.table.layout.tags_offset,
            );
        }
    }
}

impl<'a, V> Iterator for Drain<'a, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        if self.table.populated == 0 {
            return None;
        }

        // SAFETY: slot_index bounds checked, values confirmed initialized by occupied
        // tags
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

                    // SAFETY: An element is always stored within the hop-neighborhood of its
                    // root bucket. This invariant is enforced on insertion. Therefore, `off`
                    // is guaranteed to be less than `HOP_RANGE * 16`, making `off / 16` a
                    // valid neighbor index (less than HOP_RANGE).
                    debug_assert!(off < HOP_RANGE * 16);
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
    use alloc::string::String;
    use alloc::string::ToString;
    use alloc::vec;
    use core::hash::Hasher;

    use rand::TryRngCore;
    use rand::rngs::OsRng;
    use siphasher::sip::SipHasher;

    use super::*;

    struct HashState {
        k0: u64,
        k1: u64,
    }

    impl HashState {
        fn default() -> Self {
            let mut rng = OsRng;
            Self {
                k0: rng.try_next_u64().unwrap(),
                k1: rng.try_next_u64().unwrap(),
            }
        }

        fn build_hasher(&self) -> SipHasher {
            SipHasher::new_with_keys(self.k0, self.k1)
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    struct Item {
        key: u64,
        value: i32,
    }

    fn hash_key(state: &HashState, key: u64) -> u64 {
        let mut h = state.build_hasher();
        h.write_u64(key);
        h.finish()
    }

    #[test]
    fn insert_and_find() {
        let state = HashState::default();
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
        let state = HashState::default();
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
        let state = HashState::default();
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
        let state = HashState::default();
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
        let state = HashState::default();
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
    fn iter_and_drain() {
        let state = HashState::default();
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

    #[derive(Debug, PartialEq, Eq, Clone)]
    struct StringItem {
        key: String,
        value: i32,
    }

    fn hash_string_key(state: &HashState, key: &str) -> u64 {
        let mut h = state.build_hasher();
        h.write(key.as_bytes());
        h.finish()
    }

    #[test]
    fn insert_and_find_string_keys() {
        let state = HashState::default();
        let mut table: HashTable<StringItem> = HashTable::with_capacity(0);
        let keys = ["hello", "world", "foo", "bar", "baz"];

        for (i, k) in keys.iter().enumerate() {
            let hash = hash_string_key(&state, k);
            match table.entry(hash, |v: &StringItem| v.key == *k) {
                Entry::Vacant(v) => {
                    v.insert(StringItem {
                        key: k.to_string(),
                        value: i as i32,
                    });
                }
                Entry::Occupied(_) => panic!("unexpected occupied on first insert"),
            }
        }

        assert_eq!(table.len(), keys.len());

        for (i, k) in keys.iter().enumerate() {
            let hash = hash_string_key(&state, k);
            assert_eq!(
                table.find(hash, |v| v.key == *k),
                Some(&StringItem {
                    key: k.to_string(),
                    value: i as i32
                })
            );
        }

        let miss_hash = hash_string_key(&state, "not found");
        assert!(table.find(miss_hash, |v| v.key == "not found").is_none());
    }

    #[test]
    fn remove_string_keys() {
        let state = HashState::default();
        let mut table: HashTable<StringItem> = HashTable::with_capacity(0);
        let keys = ["a", "b", "c", "d", "e"];
        for (i, k) in keys.iter().enumerate() {
            let hash = hash_string_key(&state, k);
            match table.entry(hash, |v| v.key == *k) {
                Entry::Vacant(v) => {
                    v.insert(StringItem {
                        key: k.to_string(),
                        value: i as i32,
                    });
                }
                Entry::Occupied(_) => unreachable!(),
            }
        }

        assert_eq!(table.len(), 5);
        let hash_c = hash_string_key(&state, "c");
        let removed = table.remove(hash_c, |v| v.key == "c").unwrap();
        assert_eq!(removed.key, "c");
        assert_eq!(removed.value, 2);
        assert_eq!(table.len(), 4);

        let hash_a = hash_string_key(&state, "a");
        assert!(table.find(hash_a, |v| v.key == "a").is_some());
        let hash_c_2 = hash_string_key(&state, "c");
        assert!(table.find(hash_c_2, |v| v.key == "c").is_none());
    }

    #[test]
    fn iter_string_keys() {
        let state = HashState::default();
        let mut table: HashTable<StringItem> = HashTable::with_capacity(0);
        let keys = ["a", "b", "c"];
        for (i, k) in keys.iter().enumerate() {
            let hash = hash_string_key(&state, k);
            table.entry(hash, |v| v.key == *k).or_insert(StringItem {
                key: k.to_string(),
                value: i as i32,
            });
        }

        let mut found_keys = table
            .iter()
            .map(|item| item.key.clone())
            .collect::<Vec<_>>();
        found_keys.sort();
        assert_eq!(found_keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn drain_string_keys() {
        let state = HashState::default();
        let mut table: HashTable<StringItem> = HashTable::with_capacity(0);
        let keys = ["a", "b", "c"];
        for (i, k) in keys.iter().enumerate() {
            let hash = hash_string_key(&state, k);
            table.entry(hash, |v| v.key == *k).or_insert(StringItem {
                key: k.to_string(),
                value: i as i32,
            });
        }

        let drained_items: Vec<String> = table.drain().map(|item| item.key).collect();
        assert_eq!(table.len(), 0);
        assert_eq!(drained_items.len(), 3);
        assert!(drained_items.contains(&"a".to_string()));
        assert!(drained_items.contains(&"b".to_string()));
        assert!(drained_items.contains(&"c".to_string()));
    }

    #[test]
    fn entry_or_insert_with() {
        let state = HashState::default();
        let mut table: HashTable<StringItem> = HashTable::with_capacity(0);
        let key = "unique_key";
        let hash = hash_string_key(&state, key);

        let value_ref = table
            .entry(hash, |v| v.key == key)
            .or_insert_with(|| StringItem {
                key: key.to_string(),
                value: 42,
            });
        assert_eq!(value_ref.value, 42);

        let existing_ref = table
            .entry(hash, |v| v.key == key)
            .or_insert_with(|| StringItem {
                key: key.to_string(),
                value: 100,
            });
        assert_eq!(existing_ref.value, 42);

        assert_eq!(table.len(), 1);
    }

    #[test]
    fn entry_into_mut() {
        let state = HashState::default();
        let mut table = HashTable::with_capacity(10);
        let hash = hash_string_key(&state, "key");
        table
            .entry(hash, |s: &String| s == "key")
            .or_insert("key".to_string());

        let value_ref = match table.entry(hash, |s: &String| s == "key") {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(_) => unreachable!("Entry should be occupied: {:#?}", table),
        };
        *value_ref = "new_value".to_string();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[cfg(feature = "std")]
    fn histogram_output() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(10000);
        for k in 0..table.capacity() as u64 {
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

        table.print_probe_histogram();
        table.debug_stats().print();
    }

    #[test]
    fn test_clone() {
        let state = HashState::default();
        let mut original: HashTable<StringItem> = HashTable::with_capacity(10);

        let test_data = [
            ("hello", 1),
            ("world", 2),
            ("rust", 3),
            ("clone", 4),
            ("test", 5),
        ];

        for (key, value) in test_data.iter() {
            let hash = hash_string_key(&state, key);
            original
                .entry(hash, |v| v.key == *key)
                .or_insert(StringItem {
                    key: key.to_string(),
                    value: *value,
                });
        }

        let cloned = original.clone();

        assert_eq!(original.len(), cloned.len());
        assert_eq!(cloned.len(), test_data.len());

        for (key, expected_value) in test_data.iter() {
            let hash = hash_string_key(&state, key);

            let original_item = original.find(hash, |v| v.key == *key).unwrap();
            assert_eq!(original_item.value, *expected_value);

            let cloned_item = cloned.find(hash, |v| v.key == *key).unwrap();
            assert_eq!(cloned_item.value, *expected_value);
            assert_eq!(cloned_item.key, *key);
        }

        let hash = hash_string_key(&state, "hello");
        if let Some(item) = original.find_mut(hash, |v| v.key == "hello") {
            item.value = 999;
        }

        let original_hello = original.find(hash, |v| v.key == "hello").unwrap();
        assert_eq!(original_hello.value, 999);

        let cloned_hello = cloned.find(hash, |v| v.key == "hello").unwrap();
        assert_eq!(cloned_hello.value, 1);
    }

    #[test]
    fn test_clone_empty_table() {
        let original: HashTable<Item> = HashTable::with_capacity(10);
        let cloned = original.clone();

        assert_eq!(original.len(), 0);
        assert_eq!(cloned.len(), 0);
        assert!(original.is_empty());
        assert!(cloned.is_empty());
    }

    #[test]
    fn test_clone_with_overflow() {
        let mut table: HashTable<Item> = HashTable::with_capacity(16);

        let hash = 0u64;

        let num_items = 200u64;
        for k in 0..num_items {
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

        let cloned = table.clone();

        assert_eq!(table.len(), cloned.len());
        assert_eq!(table.len(), num_items as usize);

        for k in 0..num_items {
            let original_item = table.find(hash, |v| v.key == k).unwrap();
            let cloned_item = cloned.find(hash, |v| v.key == k).unwrap();

            assert_eq!(original_item.key, k);
            assert_eq!(cloned_item.key, k);
            assert_eq!(original_item.value, k as i32);
            assert_eq!(cloned_item.value, k as i32);
        }

        if let Some(item) = table.find_mut(hash, |v| v.key == 0) {
            item.value = -999;
        }

        let original_item_0 = table.find(hash, |v| v.key == 0).unwrap();
        let cloned_item_0 = cloned.find(hash, |v| v.key == 0).unwrap();

        assert_eq!(original_item_0.value, -999);
        assert_eq!(cloned_item_0.value, 0);
    }

    #[test]
    fn test_shrink_to_fit_empty_table() {
        let mut table: HashTable<Item> = HashTable::with_capacity(100);
        let initial_capacity = table.capacity();

        assert!(initial_capacity > 0);
        assert_eq!(table.len(), 0);

        table.shrink_to_fit();

        assert_eq!(table.len(), 0);
        assert_eq!(table.capacity(), 0);
    }

    #[test]
    fn test_shrink_to_fit_with_items() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(1000);

        for i in 0..50 {
            let hash = hash_key(&state, i);
            table.entry(hash, |v| v.key == i).or_insert(Item {
                key: i,
                value: i as i32,
            });
        }

        let initial_capacity = table.capacity();
        assert_eq!(table.len(), 50);
        assert!(initial_capacity >= 1000);

        table.shrink_to_fit();

        assert_eq!(table.len(), 50);
        assert!(table.capacity() < initial_capacity);
        assert!(table.capacity() >= 50);

        for i in 0..50 {
            let hash = hash_key(&state, i);
            let found = table.find(hash, |v| v.key == i);
            assert!(found.is_some());
            assert_eq!(found.unwrap().value, i as i32);
        }
    }

    #[test]
    fn test_shrink_to_fit_after_clear() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(1000);

        for i in 0..100 {
            let hash = hash_key(&state, i);
            table.entry(hash, |v| v.key == i).or_insert(Item {
                key: i,
                value: i as i32,
            });
        }

        assert_eq!(table.len(), 100);
        let capacity_with_items = table.capacity();

        table.clear();
        assert_eq!(table.len(), 0);
        assert_eq!(table.capacity(), capacity_with_items);

        table.shrink_to_fit();

        assert_eq!(table.len(), 0);
        assert_eq!(table.capacity(), 0);
    }

    #[test]
    fn test_shrink_to_fit_after_removals() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(1000);

        for i in 0..200 {
            let hash = hash_key(&state, i);
            table.entry(hash, |v| v.key == i).or_insert(Item {
                key: i,
                value: i as i32,
            });
        }

        assert_eq!(table.len(), 200);
        let initial_capacity = table.capacity();

        for i in 0..190 {
            let hash = hash_key(&state, i);
            table.remove(hash, |v| v.key == i);
        }

        assert_eq!(table.len(), 10);
        assert_eq!(table.capacity(), initial_capacity);

        table.shrink_to_fit();

        assert_eq!(table.len(), 10);
        assert!(table.capacity() < initial_capacity);
        assert!(table.capacity() >= 10);

        for i in 190..200 {
            let hash = hash_key(&state, i);
            let found = table.find(hash, |v| v.key == i);
            assert!(found.is_some());
            assert_eq!(found.unwrap().value, i as i32);
        }
    }

    #[test]
    fn test_shrink_to_fit_with_overflow() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(100);

        let mut items_with_same_hash = Vec::new();
        let base_hash = hash_key(&state, 42);

        for i in 0..50 {
            items_with_same_hash.push(Item {
                key: 1000 + i,
                value: i as i32,
            });
        }

        for item in &items_with_same_hash {
            table
                .entry(base_hash, |v| v.key == item.key)
                .or_insert(item.clone());
        }

        assert_eq!(table.len(), 50);
        let initial_capacity = table.capacity();

        for i in 0..40 {
            table.remove(base_hash, |v| v.key == 1000 + i);
        }

        assert_eq!(table.len(), 10);

        table.shrink_to_fit();

        assert_eq!(table.len(), 10);
        assert!(table.capacity() <= initial_capacity);

        for i in 40..50 {
            let found = table.find(base_hash, |v| v.key == 1000 + i);
            assert!(found.is_some());
            assert_eq!(found.unwrap().value, i as i32);
        }
    }

    #[test]
    fn test_shrink_to_fit_no_change_when_optimal() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        // Add items gradually and call shrink_to_fit to get to optimal size
        for i in 0..50 {
            let hash = hash_key(&state, i);
            table.entry(hash, |v| v.key == i).or_insert(Item {
                key: i,
                value: i as i32,
            });
        }

        // First shrink to get to optimal size
        table.shrink_to_fit();
        let optimal_capacity = table.capacity();

        // Now shrink_to_fit should not change the capacity
        table.shrink_to_fit();
        let capacity_after_second_shrink = table.capacity();

        assert_eq!(table.len(), 50);
        assert_eq!(optimal_capacity, capacity_after_second_shrink);

        // Verify all items are still there
        for i in 0..50 {
            let hash = hash_key(&state, i);
            let found = table.find(hash, |v| v.key == i);
            assert!(found.is_some());
            assert_eq!(found.unwrap().value, i as i32);
        }
    }

    #[test]
    fn test_shrink_to_fit_preserves_functionality() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(500);

        for i in 0..100 {
            let hash = hash_key(&state, i);
            table.entry(hash, |v| v.key == i).or_insert(Item {
                key: i,
                value: i as i32,
            });
        }

        table.shrink_to_fit();

        let new_hash = hash_key(&state, 999);
        table.entry(new_hash, |v| v.key == 999).or_insert(Item {
            key: 999,
            value: 999,
        });

        assert_eq!(table.len(), 101);

        let found = table.find(new_hash, |v| v.key == 999);
        assert!(found.is_some());
        assert_eq!(found.unwrap().value, 999);

        let removed = table.remove(new_hash, |v| v.key == 999);
        assert!(removed.is_some());
        assert_eq!(table.len(), 100);
    }
}
