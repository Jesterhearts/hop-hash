//! A high-performance hash table using 16-way hopscotch hashing.
//!
//! Hopscotch hashing is a technique which attempts to place an item within a
//! fixed distance of its home bucket during insertion. If this fails, an empty
//! spot is located in the table and bubbled backwards by repeatedly identifying
//! an element that can move into the empty slot without leaving its
//! neighborhood and swapping it with the empty slot until the empty slot is in
//! the fixed range. If bubbling fails (typically due to very high load), the
//! table is resized and insertion re-attempted.  This has the nice effect that
//! lookups and removals have constant-time worst case behavior (and insertion
//! has amortized constant time behavior), rather than O(N). With pathological
//! or adversarial inputs, resizing can end up in an infinite loop that results
//! in OOM, so you still need a good hash function.
//!
//! [`HashTable<V>`] stores values of type `V` and provides fast insertion,
//! lookup, and removal operations. This is a fairly low-level structure that
//! requires you to provide both the hash value and an equality predicate for
//! each operation. Prefer using the [`HashMap<K, V>`] or [`HashSet<V>`]
//! wrappers for a more convenient key-value or set interface unless you are
//! implementing your own Map or Set structure.
//!
//! ## Design
//!
//! This table is designed around 16-byte sse2 operations to facilitate
//! performance. The table is a contiguous sequence of 16-entry buckets. An
//! item's hash maps it to a "root" bucket. For each root, a corresponding
//! `HopInfo` struct is allocated which tracks the occupancy of the 8 neighbor
//! buckets starting at the root. Each bucket also has a corresponding 16-byte
//! tag array which tracks a 7-bit fingerprint of the hash for each entry in the
//! bucket.
//!
//! Each neighborhood byte tracks how many entries in that neighbor slot are
//! occupied, allowing for up to 16 entries per neighbor slot. This allows for
//! fast scans to see which neighbors need to be probed during lookups, as the
//! algorithm knows to check all buckets with at least one neighbor. It might be
//! possible to get more precise bucket scans by tracking a 16-bit mask of which
//! bucket slots are occupied per neighbor, but this would increase overhead to
//! 3 bytes per entry for 16-way and prevent identifying scan targets with a
//! cmp/mask operation pair. Ultimately, it seems unlikely to provide
//! performance gains as intra-bucket tag collisions are not common enough
//! (eliminating these false positives is the major benefit you'd see from this
//! scheme) and identifying which neighbors to scan is fairly hot in profiles,
//! so slowing this down at all is likely to hurt rather than help performance.
//!
//! Tags are derived from the top 7-bits of the hash value, with the sign bit
//! reserved to mark empty slots. This allows the use of just a single load/mask
//! operation to identify empty slots when scanning for an empty slot during
//! insertion, which is a hot path in profiles. It is important that tags are
//! not derived from the lower bits of the hash, as that causes them to be
//! correlated with their location in the table, leading to significantly more
//! tag collisions and greatly increased scan times.
//!
//! For bad hash functions (e.g. one that only provides a 16-bit hash), this
//! can cause every tag to evaluate to zero, but using bit-mixing over a simple
//! shift hurts benchmarks for the far more common case of a 64-bit hash value.
//!
//! All data is stored in one contiguous type-erased allocation.
//! `[ HopInfo | Tags | Values ]`
//!
//! It's possible to combine all of the items into one single array of a struct
//! type which combines a `HopInfo`, 16 tags, and 16 `MaybeUninit<V>` entries,
//! but in testing this seems to signficantly hurt iteration performance without
//! an offsetting increase in other benchmarks. In addition, storing the items
//! in separate allocations seems to harm performance considerably, although I
//! don't have a good explanation for why yet. It's possible that it's simply
//! overhead from how the items were being initialized, and further testing
//! might indicate that it's safe to split out the allocations. I'm not sure if
//! this would actually simplify the code much, though.
//!
//! Sizes are always rounded up to the next power-of-two for the extent of the
//! root buckets to allow for simple & masking operations to compute root
//! buckets based on hashes. Using this over modulo has a significant
//! performance impact.
//!
//! An additional pad of `HOP_RANGE` buckets is added to the end of the table to
//! allow the final neighborhood to span a full 16 buckets without wrapping.
//! Adding wrapping would save the memory allocated to this pad (`256 *
//! size_of(V)`), but would complicate the code significantly, particularly
//! during bubbling.
//!
//! ### Other Quirks & Oddities
//!
//! The table makes use of `ptr::write_bytes(0)` to initialize the hopinfo
//! arrays rather than using `alloc_zeroed`. This makes a massive difference in
//! benchmarks on my machine (30%) for some reason. I suspect it's a
//! benchmarking artifact.
//!
//! The table _always_ examines bucket 0 during lookups without even checking
//! the neighborhood layout. During testing, bucket 0 was found to almost always
//! contain at least one item, and frequently was filled with 16 items mapped to
//! their home bucket. Unconditionally checking this bucket first seems to
//! improve benchmarks by a lot by skipping the extra lookup/cmp/mask
//! required to examine the neighbors bitmap when 80-90% of lookups will have a
//! hit without looking further.
//!
//! ## Safety Invariants
//!
//! The implementation relies on the following key invariants:
//!
//! 1. **Index Bounds**: All indices are validated through the following
//!    relationships:
//!    - `hop_bucket <= max_root_mask` (root buckets are valid)
//!    - `absolute_index = hop_bucket * LANES + offset` where `offset <
//!      HOP_RANGE * LANES`
//!    - `max_root_mask = capacity.saturating_sub(HOP_RANGE).wrapping_sub(1)`
//!      ensures that `hop_bucket + HOP_RANGE` never exceeds allocated bounds
//!
//! 2. **Initialization**: A tag value of `EMPTY` indicates an uninitialized
//!    slot; any other tag value indicates the slot contains an initialized
//!    value of type `V`.
//!
//! 3. **Neighborhood Consistency**: For each entry at absolute index `idx`:
//!    - Its root bucket is `root = (hash as usize) & max_root_mask`
//!    - Its neighbor index is `n = (idx - root * LANES) / LANES`
//!    - `n < HOP_RANGE` (entries are always within their root's neighborhood)
//!    - `hopmap[root].neighbors[n]` tracks the count of entries in neighbor
//!      slot `n`
//!
//! 4. **Bubbling**: During insertion, empty slots found beyond the neighborhood
//!    are "bubbled" backward by finding elements that can move forward without
//!    leaving their own neighborhoods. The invariant `empty_idx >=
//!    hopmap_index` is maintained throughout (empty slots are always at or
//!    ahead of the root).
//!
//! [`HashMap<K, V>`]: crate::hash_map::HashMap
//! [`HashSet<V>`]: crate::hash_set::HashSet

use alloc::alloc::handle_alloc_error;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::Layout;
#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;
use core::error::Error;
use core::fmt::Debug;
use core::fmt::Display;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use cfg_if::cfg_if;

/// Errors that can occur during a `try_entry` operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryEntryError {
    /// The table is full and cannot accommodate more entries. The table needs a
    /// resize.
    CapacityTooSmall,
    /// No free slot was found in the table that was a candidate for insertion.
    /// The table needs a resize.
    NoFreeSlot,
}

impl Display for TryEntryError {
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        match self {
            TryEntryError::CapacityTooSmall => write!(f, "table is full"),
            TryEntryError::NoFreeSlot => write!(f, "no free slot found"),
        }
    }
}

impl Error for TryEntryError {}

cfg_if! {
    // Try to save someone if they are in a situation where multiple versions of the crate
    // specify eight-way, density-ninety-two, and density-ninety-seven.
    if #[cfg(all(feature = "density-ninety-two", feature = "eight-way"))] {
        const TARGET_LOAD: f32 = 0.92;
    } else if #[cfg(feature = "density-ninety-seven")] {
        const TARGET_LOAD: f32 = 0.97;
    } else if #[cfg(feature = "density-ninety-two")] {
        const TARGET_LOAD: f32 = 0.92;
    } else if #[cfg(feature = "density-eighty-seven-point-five")] {
        const TARGET_LOAD: f32 = 0.875;
    } else {
        const TARGET_LOAD: f32 = 0.875;
    }
}

#[inline(always)]
fn target_load_factor(capacity: usize) -> usize {
    (capacity as f32 * TARGET_LOAD) as usize
}

#[inline(always)]
fn target_load_factor_inverse(capacity: usize) -> usize {
    (capacity as f32 / TARGET_LOAD) as usize
}

/// Prefetches data into the cache.
///
/// # Safety
///
/// The caller must ensure that `ptr` points to a memory location that is safe
/// to read from. While `_mm_prefetch` might not fault on invalid addresses,
/// the behavior is undefined if the address is not valid for reads.
#[inline(always)]
unsafe fn prefetch<T>(ptr: *const T) {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "sse"
    ))]
    {
        unsafe {
            _mm_prefetch(ptr as *const i8, _MM_HINT_T0);
        }
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
/// improve benchmarks.
const EMPTY: u8 = 0x80;

// Number of neighbors tracked per bucket. Could be larger for wider SIMD
// operations, but we only support SSE2 + it wastes a lot of space if it's
// wider than 16.
cfg_if! {
    if #[cfg(feature = "sixteen-way")] {
        const HOP_RANGE: usize = 16;
    } else if #[cfg(feature = "eight-way")] {
        const HOP_RANGE: usize = 8;
    // If they didn't specify and picked density-ninety-seven, assume they want 16-way
    // since 8-way with 97% is highly likely to over-allocate for large tables.
    } else if #[cfg(feature = "density-ninety-seven")] {
        const HOP_RANGE: usize = 16;
    }else {
        const HOP_RANGE: usize = 8;
    }
}

const LANES: usize = 16;

#[inline(always)]
fn hashtag(tag: u64) -> u8 {
    (tag >> 57) as u8
}

/// Search for a movable index in the bubble range
///
/// # Safety
/// - `values` must point to a slice of `MaybeUninit<V>` with length greater
///   than or equal to `empty_idx`.
/// - The range `[bubble_base, empty_idx)` must be initialized.
/// - Caller must ensure `0 <= bubble_base < empty_idx <= values.len()`.
/// - `max_root_mask` must match the table’s current mask; roots are
///   `0..=max_root_mask` and map to absolute indices as `root*16`.
#[inline(always)]
unsafe fn find_next_movable_index<V>(
    values: &[MaybeUninit<V>],
    bubble_base: usize,
    empty_idx: usize,
    max_root_mask: usize,
    rehash: &dyn Fn(&V) -> u64,
) -> Option<(usize, u64)> {
    for idx in bubble_base..empty_idx {
        // SAFETY: The caller guarantees that `idx` is within `bubble_base..empty_idx`
        // and that `empty_idx` is within the bounds of `values`, making
        // `get_unchecked` safe. The caller also ensures that elements in this range
        // are initialized, making `assume_init_ref` safe.
        // Using `wrapping_sub` because `empty_idx` is guaranteed to be
        // >= `hopmap_index` by the hopscotch algorithm invariant (empty slots are
        // always found forward from or at the root bucket position). The wrapping
        // behavior handles the algebraic calculation without overflow concerns.
        unsafe {
            let hash = rehash(values.get_unchecked(idx).assume_init_ref());
            let hopmap_index = (hash as usize & max_root_mask) * LANES;

            let distance = empty_idx.wrapping_sub(hopmap_index);
            if distance < HOP_RANGE * LANES {
                return Some((idx, hash));
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
    neighbors: [u8; 16],
}

impl HopInfo {
    #[inline(always)]
    fn candidates(&self) -> u16 {
        cfg_if! {
            if #[cfg(all(
                any(target_arch = "x86_64", target_arch = "x86"),
                target_feature = "sse2"
            ))] {
                // SAFETY: We have ensure that we are on x86/x86_64 with SSE2 support
                unsafe { self.candidates_sse2() }
            } else {
                let mut bits: u16 = 0;
                for i in 0..HOP_RANGE {
                    if self.neighbors[i] > 0 {
                        bits |= 1 << i;
                    }
                }
                bits
            }
        }
    }

    /// Get a bitmask of neighbor slots that are occupied (non-zero).
    ///
    /// # Safety
    /// - Caller must ensure the CPU supports SSE2 instructions.
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "x86"),
        target_feature = "sse2"
    ))]
    #[inline(always)]
    unsafe fn candidates_sse2(&self) -> u16 {
        // SAFETY: We have ensured that `HopInfo` is `#[repr(C, align(16))]`,
        // with `neighbors` at offset 0. This guarantees 16-byte alignment,
        // making it safe to load via `_mm_load_si128`.
        unsafe {
            let data = _mm_load_si128(self.neighbors.as_ptr() as *const __m128i);
            let cmp = _mm_cmpgt_epi8(data, _mm_setzero_si128());
            _mm_movemask_epi8(cmp) as u16
        }
    }

    /// Clear neighbor count at the given index
    ///
    /// # Safety
    ///
    /// The caller must ensure `n_index` is within the bounds of the neighbors
    /// array (less than `HOP_RANGE`).
    #[inline(always)]
    unsafe fn clear(
        &mut self,
        n_index: usize,
    ) {
        // SAFETY: Caller ensures `n_index` is within bounds of the neighbors array
        unsafe {
            debug_assert!(self.neighbors[n_index] > 0);
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
    unsafe fn set(
        &mut self,
        n_index: usize,
    ) {
        // SAFETY: Caller ensures `n_index` is within bounds of the neighbors array
        unsafe {
            debug_assert!(self.neighbors[n_index] < LANES as u8);
            *self.neighbors.get_unchecked_mut(n_index) += 1;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DataLayout {
    layout: Layout,
    hopmap_offset: usize,
    tags_offset: usize,

    buckets_offset: usize,
}

impl DataLayout {
    fn new<V>(capacity: Capacity) -> Self {
        let hopmap_layout = Layout::array::<HopInfo>(capacity.max_root_mask().wrapping_add(1))
            .expect("allocation size overflow");
        let tags_layout =
            Layout::array::<u8>(capacity.base * LANES).expect("allocation size overflow");
        let buckets_layout = Layout::array::<MaybeUninit<V>>(capacity.base * LANES)
            .expect("allocation size overflow");

        let (layout, hopmap_offset) = Layout::new::<()>().extend(hopmap_layout).unwrap();
        let (layout, tags_offset) = layout.extend(tags_layout).unwrap();
        let (layout, buckets_offset) = layout.extend(buckets_layout).unwrap();

        DataLayout {
            layout,
            hopmap_offset,
            tags_offset,
            buckets_offset,
        }
    }
}

/// Debug statistics for hash table analysis.
#[cfg(feature = "stats")]
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
    /// Load factor (populated / capacity)
    pub load_factor: f64,
    /// Slot utilization (occupied_slots / total_slots)
    pub slot_utilization: f64,
    /// Total memory in bytes used by the table
    pub total_bytes: usize,
    /// Estimated wasted memory in bytes
    pub wasted_bytes: usize,
}

#[cfg(feature = "stats")]
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

/// Probe histogram for analyzing probe lengths.
#[cfg(feature = "stats")]
pub struct ProbeHistogram {
    #[cfg_attr(not(feature = "std"), allow(dead_code))]
    populated: usize,
    #[cfg_attr(not(feature = "std"), allow(dead_code))]
    buckets: usize,
    /// Histogram of probe lengths by number of buckets probed.
    pub probe_length_by_bucket: [usize; HOP_RANGE],
    /// Histogram of the total number of items in probed buckets, indexed by
    /// probe length.
    ///
    /// This can be interpreted as a measure of the "work" required for lookups.
    /// For a given probe length `L` (the number of neighbor buckets that need
    /// to be scanned for items belonging to a root bucket), the value at
    /// `[L-1]` is the sum of the number of items in all those `L` neighbor
    /// buckets.
    ///
    /// For example, if a root bucket has entries in 3 of its neighbor buckets
    /// (probe length = 3), and those neighbor buckets contain 2, 4, and 1 items
    /// respectively, then `probe_length_by_count[2]` (for L=3) would be
    /// incremented by 7 (2+4+1).
    pub probe_length_by_count: [usize; HOP_RANGE],
    /// Distribution of number of entries in each bucket relative to its root.
    /// This shows how tightly clustered entries are around their ideal bucket
    /// (bucket 0).
    pub bucket_distribution: [usize; HOP_RANGE],
}

#[cfg(feature = "stats")]
impl ProbeHistogram {
    /// Pretty-print the probe histogram.
    #[cfg(feature = "std")]
    pub fn print(&self) {
        let max = *self
            .probe_length_by_bucket
            .iter()
            .max()
            .unwrap_or(&0)
            .max(self.probe_length_by_count.iter().max().unwrap_or(&0))
            .max(self.bucket_distribution.iter().max().unwrap_or(&0));
        if max == 0 {
            println!("probe histogram: empty");
            return;
        }

        let max_bar = 60usize;
        let total_units = max_bar * 8;
        println!(
            "probe length by bucket ({} entries, {}x16 slots):",
            self.populated, self.buckets
        );

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

        for (i, &count) in self
            .probe_length_by_bucket
            .iter()
            .take(HOP_RANGE)
            .enumerate()
        {
            let label = alloc::format!("{:>2}", i + 1);
            let bar = make_bar(count);
            println!("{} | {} ({})", label, bar, count);
        }

        println!("Probe length by count (in-table entries):");
        for (i, &count) in self
            .probe_length_by_count
            .iter()
            .take(HOP_RANGE)
            .enumerate()
        {
            let label = alloc::format!("{:>2}", i + 1);
            let bar = make_bar(count);
            println!("{} | {} ({})", label, bar, count);
        }

        println!("Bucket distribution (in-table entries):");
        for (i, &count) in self.bucket_distribution.iter().enumerate() {
            let label = alloc::format!("{:>2}", i);
            let bar = make_bar(count);
            println!("{} | {} ({})", label, bar, count);
        }
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
/// - **Memory**: 2 bytes per entry overhead (1 byte for tags, 1 byte for hop
///   metadata), plus the size of `V`. Note that the table maintains a minimum
///   capacity of 272 entries (144 for 8-way) due to padding requirements.
/// - **Insertion**: Amortized O(1). Individual insertions may trigger bubbling
///   operations or resizing, but the cost is amortized across insertions.
/// - **Lookup**: O(1) with a bounded probe distance of at most 16 buckets (8
///   for 8-way).
/// - **Deletion**: O(1) with the same bounded probe distance as lookup.
pub struct HashTable<V> {
    layout: DataLayout,
    alloc: NonNull<u8>,

    populated: usize,
    max_pop: usize,
    max_root_mask: usize,

    _phantom: core::marker::PhantomData<V>,
}

impl<V> Debug for HashTable<V> {
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        use alloc::format;
        use alloc::string::ToString;

        if self.is_empty() {
            return f
                .debug_struct("HashTable")
                .field("metadata", &"empty")
                .field("populated", &self.populated)
                .field("capacity", &self.max_pop)
                .finish();
        }

        // SAFETY: The `unsafe` block is safe because the `if self.is_empty()`
        // check ensures that this code only runs on a non-empty (and therefore
        // initialized) table. An initialized table guarantees that `self.alloc`
        // points to a valid allocation matching `self.layout`, making the calls to
        // `hopmap_ptr` and `tags_ptr` safe.
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
                        .chunks(LANES)
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
                .finish()
        }
    }
}

impl<V> Clone for HashTable<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        let new_table = Self {
            layout: self.layout,
            alloc: if self.layout.layout.size() == 0 {
                NonNull::dangling()
            } else {
                // SAFETY: We have validated that the layout size is non-zero. The `alloc`
                // function returns a valid pointer, and we handle allocation errors
                // if it returns null.
                unsafe {
                    let raw_alloc = alloc::alloc::alloc(self.layout.layout);
                    if raw_alloc.is_null() {
                        handle_alloc_error(self.layout.layout);
                    }

                    core::ptr::copy_nonoverlapping(
                        self.alloc.as_ptr(),
                        raw_alloc,
                        self.layout.buckets_offset,
                    );

                    NonNull::new_unchecked(raw_alloc)
                }
            },
            populated: self.populated,
            max_pop: self.max_pop,
            max_root_mask: self.max_root_mask,
            _phantom: core::marker::PhantomData,
        };

        // SAFETY: The new table has the same capacity and layout as the source
        // table. We iterate through the tags, and for each occupied slot, we clone
        // the value. This is safe because:
        // 1. `get_unchecked` is safe as we iterate up to `src_tags.len()`, which is
        //    within the bounds of all allocated slices.
        // 2. `assume_init_ref` is safe because a non-`EMPTY` tag guarantees that the
        //    corresponding bucket is initialized.
        // 3. `write` to `dst_buckets` is safe because the destination is uninitialized
        //    and within bounds.
        unsafe {
            let src_buckets = self.buckets_ptr().as_ref();
            let dst_buckets = new_table.buckets_ptr().as_mut();
            let src_tags = self.tags_ptr().as_ref();

            for i in 0..src_tags.len() {
                let tag = *src_tags.get_unchecked(i);
                if tag != EMPTY {
                    dst_buckets
                        .get_unchecked_mut(i)
                        .write(src_buckets.get_unchecked(i).assume_init_ref().clone());
                }
            }

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
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity: Capacity = target_load_factor_inverse(capacity.div_ceil(LANES)).into();

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
            populated: 0,
            max_pop: target_load_factor(capacity.base * LANES),
            max_root_mask: capacity.max_root_mask(),
            _phantom: core::marker::PhantomData,
        }
    }

    fn hopmap_ptr(&self) -> NonNull<[HopInfo]> {
        // SAFETY: This is safe because `self.alloc` is guaranteed to point to a
        // valid allocation with a layout described by `self.layout`. The offset
        // `self.layout.hopmap_offset` and the length `self.max_root_mask + 1` are
        // derived from the capacity and are guaranteed to be within the bounds of
        // the allocated memory block.
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.hopmap_offset).cast(),
                self.max_root_mask.wrapping_add(1),
            )
        }
    }

    fn buckets_ptr(&self) -> NonNull<[MaybeUninit<V>]> {
        // SAFETY: This is safe because `self.alloc` is guaranteed to point to a
        // valid allocation with a layout described by `self.layout`. The offset
        // `self.layout.buckets_offset` and the calculated length are derived from
        // the capacity and are guaranteed to be within the bounds of the allocated
        // memory block.
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.buckets_offset).cast(),
                if self.layout.layout.size() == 0 {
                    0
                } else {
                    (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * LANES
                },
            )
        }
    }

    fn tags_ptr(&self) -> NonNull<[u8]> {
        // SAFETY: This is safe because `self.alloc` is guaranteed to point to a
        // valid allocation with a layout described by `self.layout`. The offset
        // `self.layout.tags_offset` and the calculated length are derived from
        // the capacity and are guaranteed to be within the bounds of the allocated
        // memory block.
        unsafe {
            NonNull::slice_from_raw_parts(
                self.alloc.add(self.layout.tags_offset).cast(),
                if self.layout.layout.size() == 0 {
                    0
                } else {
                    (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * LANES
                },
            )
        }
    }

    /// Returns an iterator over all values in the table.
    ///
    /// The iterator yields `&V` references in an arbitrary order.
    /// The iteration order is not specified and may change between versions.
    pub fn iter(&self) -> Iter<'_, V> {
        Iter {
            table: self,
            bucket_index: 0,
        }
    }

    /// Returns a mutable iterator over all values in the table.
    ///
    /// The iterator yields `&mut V` references in an arbitrary order.
    /// The iteration order is not specified and may change between versions.
    pub fn iter_mut(&mut self) -> IterMut<'_, V> {
        // SAFETY: We have ensured that `self` is a valid mutable reference to
        // the hash table. The `IterMut` struct will only yield mutable references
        // to values within the table, and the lifetime is tied to `&mut self`,
        // ensuring no other mutable references can exist simultaneously.
        unsafe {
            IterMut {
                tags: self.tags_ptr().as_ref(),
                values: self.buckets_ptr().as_mut(),
            }
        }
    }

    /// Returns an iterator that removes and yields all values from the table.
    ///
    /// After calling `drain()`, the table will be empty. The iterator yields
    /// owned values in an arbitrary order.
    ///
    /// Calling `mem::forget` on the iterator will leak all unyielded values in
    /// the table without dropping them. This will cause memory to be leaked
    /// _even if the type does not allocate heap memory_ as the iterator
    /// allocates memory to track its internal state.
    pub fn drain(&mut self) -> Drain<'_, V> {
        let total_slots = if self.layout.layout.size() == 0 {
            0
        } else {
            (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * LANES
        };

        if total_slots == 0 {
            return Drain {
                total_slots,
                occupied: Box::new([]),
                table: self,
                bucket_index: 0,
            };
        }

        let old_populated = self.tags_ptr();
        let mut occupied = Box::new_uninit_slice(old_populated.len());

        // SAFETY: We have ensured that `old_populated` and `occupied` point to
        // valid memory regions of the same length. We copy the tags from
        // `old_populated` to `occupied`, then zero out the hopmap and mark all buckets
        // as empty so we don't double-drop. Finally, we assume that `occupied`
        // is initialized since we just copied data into it.
        let occupied = unsafe {
            core::ptr::copy_nonoverlapping(
                old_populated.as_ref().as_ptr(),
                occupied.as_mut_ptr().cast(),
                old_populated.len(),
            );

            core::ptr::write_bytes(self.alloc.as_ptr(), 0x0, self.layout.tags_offset);
            core::ptr::write_bytes(
                self.alloc.as_ptr().add(self.layout.tags_offset),
                EMPTY,
                self.layout.buckets_offset - self.layout.tags_offset,
            );

            occupied.assume_init()
        };

        self.populated = 0;

        Drain {
            total_slots,
            occupied,
            table: self,
            bucket_index: 0,
        }
    }

    /// Returns `true` if the table contains no elements.
    pub fn is_empty(&self) -> bool {
        self.populated == 0
    }

    /// Returns the number of elements in the table.
    pub fn len(&self) -> usize {
        self.populated
    }

    /// Removes all elements from the table.
    ///
    /// This operation preserves the table's allocated capacity. All values are
    /// properly dropped if they implement `Drop`. After calling `clear()`, the
    /// table will be empty but maintain its current capacity.
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
    }

    /// Shrinks the capacity of the hash table as much as possible.
    ///
    /// This method will shrink the table's capacity to just fit the current
    /// number of elements, potentially freeing up significant amounts of
    /// memory.
    ///
    /// If the table is empty, it will be completely deallocated and reset to
    /// a zero-capacity state.
    pub fn shrink_to_fit(
        &mut self,
        rehash: impl Fn(&V) -> u64,
    ) {
        if self.populated == 0 {
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

        let required = self.populated;
        let new_capacity: Capacity = target_load_factor_inverse(required.div_ceil(LANES)).into();
        if new_capacity.max_root_mask() < self.max_root_mask {
            self.do_resize_rehash(new_capacity, &rehash);
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
    pub fn reserve(
        &mut self,
        additional: usize,
        rehash: impl Fn(&V) -> u64,
    ) {
        let required = self.populated.saturating_add(additional);
        if required > self.max_pop {
            let new_capacity: Capacity =
                target_load_factor_inverse(required.div_ceil(LANES)).into();
            self.do_resize_rehash(new_capacity, &rehash);
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
    pub fn remove(
        &mut self,
        hash: u64,
        eq: impl Fn(&V) -> bool,
    ) -> Option<V> {
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
            let bucket_ref = unsafe { self.buckets_ptr().as_ref().get_unchecked(index) };
            // SAFETY: We have confirmed that the value at this index is initialized due to
            // an occupied tag.
            let value = unsafe { bucket_ref.assume_init_read() };

            // SAFETY: `search_neighborhood` guarantees that `index` is within the
            // neighborhood of `hop_bucket`, which means `index >= hop_bucket * LANES`.
            // This ensures the subtraction is safe and produces a valid offset.
            let offset = index - hop_bucket * LANES;
            let n_index = offset / LANES;
            // SAFETY: We have validated that `index` is a valid slot index from
            // `search_neighborhood`, `hop_bucket` is also valid, `index >= hop_bucket *
            // LANES` (established by neighborhood invariant), and `n_index <
            // HOP_RANGE` (derived from the offset), ensuring it is a valid
            // neighbor index.
            unsafe {
                self.hopmap_ptr()
                    .as_mut()
                    .get_unchecked_mut(hop_bucket)
                    .clear(n_index);
                self.clear_occupied(index);
            }

            return Some(value);
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
    #[inline(always)]
    pub fn entry(
        &mut self,
        hash: u64,
        eq: impl Fn(&V) -> bool,
        rehash: impl Fn(&V) -> u64,
    ) -> Entry<'_, V> {
        self.maybe_resize_rehash(&rehash);
        // SAFETY: We have ensured that the table is properly initialized and has
        // sufficient capacity through `maybe_resize_rehash`.
        unsafe { self.entry_impl(hash, eq, &rehash) }
    }

    /// Gets an entry for the given hash and equality predicate without
    /// triggering a resize.
    ///
    /// This method returns an `Entry` enum that allows for efficient insertion
    /// or modification of values, similar to [`entry`](HashTable::entry), but
    /// without automatically resizing the table. If the table is full or cannot
    /// accommodate the entry without resizing or bubbling, an error is returned
    /// instead.
    ///
    /// This method provides a constant-time worst-case bound for insertion,
    /// provided you can tolerate failures. It will at most search the
    /// neighborhood of the root bucket for an empty slot (rather than the
    /// entire table), and will not attempt to resize or bubble entries.o
    ///
    /// For medium to large tables with a reasonable load factor, this has a
    /// very high success rate, as most insertions will find a free slot in
    /// the neighborhood.
    ///
    /// # Arguments
    ///
    /// * `hash` - The hash value for the entry
    /// * `eq` - A predicate function that returns `true` for matching values
    ///
    /// # Errors
    ///
    /// Returns a [`TryEntryError`] if:
    /// - The table is at capacity
    ///   ([`CapacityTooSmall`](TryEntryError::CapacityTooSmall))
    /// - No free slot is available ([`NoFreeSlot`](TryEntryError::NoFreeSlot))
    pub fn try_entry(
        &mut self,
        hash: u64,
        eq: impl Fn(&V) -> bool,
    ) -> Result<Entry<'_, V>, TryEntryError> {
        if self.max_pop == 0 {
            return Err(TryEntryError::CapacityTooSmall);
        }

        let hop_bucket = self.hopmap_index(hash);

        // SAFETY: We have validated that `hop_bucket` is within bounds through
        // `hopmap_index`, which derives it from the hash and `max_root_mask`. We have a
        // non-zero capacity, so there is at least one bucket.
        if let Some(index) = unsafe { self.search_neighborhood(hash, hop_bucket, &eq) } {
            return Ok(Entry::Occupied(OccupiedEntry {
                n_index: index - hop_bucket * LANES,
                table: self,
                root_index: hop_bucket,
            }));
        }

        if self.populated >= self.max_pop {
            return Err(TryEntryError::CapacityTooSmall);
        }

        let absolute_empty_idx;
        // SAFETY: We have validated that `hop_bucket` is within bounds through
        // `hopmap_index`, which derives it from the hash and `max_root_mask`.
        // We know that there are HOP_RANGE * LANES slots in the neighborhood, due to
        // how capacity is calculated.
        unsafe {
            let Some(empty_idx) =
                self.find_next_unoccupied_in_range(self.absolute_index(hop_bucket, 0))
            else {
                return Err(TryEntryError::NoFreeSlot);
            };
            absolute_empty_idx = empty_idx;
        };

        // SAFETY: We have validated `absolute_empty_idx` through
        // `find_next_unoccupied_in_range`.
        debug_assert!(unsafe { !self.is_occupied(absolute_empty_idx) });

        Ok(Entry::Vacant(VacantEntry {
            table: self,
            hopmap_root: hop_bucket,
            hash,
            n_index: absolute_empty_idx - hop_bucket * LANES,
        }))
    }

    /// Internal entry implementation that performs the actual lookup.
    ///
    /// # Safety
    ///
    /// The capacity must not be zero.
    #[inline]
    unsafe fn entry_impl(
        &mut self,
        hash: u64,
        eq: impl Fn(&V) -> bool,
        rehash: &dyn Fn(&V) -> u64,
    ) -> Entry<'_, V> {
        let hop_bucket = self.hopmap_index(hash);

        // SAFETY: We have ensured that `hop_bucket` is within bounds, as it is derived
        // from the hash and mask.
        let index = unsafe { self.search_neighborhood(hash, hop_bucket, &eq) };
        if let Some(index) = index {
            return Entry::Occupied(OccupiedEntry {
                n_index: index - hop_bucket * LANES,
                table: self,
                root_index: hop_bucket,
            });
        }

        // SAFETY: We have ensured `hop_bucket` is within bounds, as it is derived from
        // the hash and mask.
        Entry::Vacant(unsafe { self.do_vacant_lookup(hash, hop_bucket, rehash) })
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
        let tag = hashtag(hash);
        let base = bucket * LANES;

        // SAFETY: Caller ensures that `bucket` is within bounds, as it is derived from
        // the hash and `max_root_mask`.
        unsafe {
            prefetch(self.hopmap_ptr().as_ref().as_ptr().add(bucket));
            prefetch(self.tags_ptr().as_ref().as_ptr().add(base + LANES));
        }

        // SAFETY: We have ensured `base` is valid, calculated from a validated bucket
        // and an index within the neighborhood.
        if let Some(value) = unsafe { self.search_tags(&eq, tag, base) } {
            return Some(value);
        }

        // SAFETY: Caller ensures that `bucket` is within bounds, as it is derived from
        // the hash and `max_root_mask`.
        let mut neighborhood_mask = unsafe {
            self.hopmap_ptr()
                .as_ref()
                .get_unchecked(bucket)
                .candidates()
        };

        let mut index;
        let mut next_index = neighborhood_mask.trailing_zeros() as usize;

        while neighborhood_mask != 0 {
            index = next_index;
            neighborhood_mask ^= 1 << index;
            next_index = neighborhood_mask.trailing_zeros() as usize;

            // SAFETY: Caller ensures that `bucket` is within bounds, as it is derived
            // from the hash and `max_root_mask`.
            unsafe {
                prefetch(
                    self.tags_ptr()
                        .as_ref()
                        .as_ptr()
                        .add(base + next_index * LANES * usize::from(neighborhood_mask != 0)),
                );
            }

            if index != 0 {
                let base = base + index * LANES;

                // SAFETY: We have ensured `base` is valid, calculated from a validated bucket
                // and an index within the neighborhood.
                if let Some(value) = unsafe { self.search_tags(&eq, tag, base) } {
                    return Some(value);
                }
            }
        }
        None
    }

    /// Search 16 tags starting at base for matching tags and values.
    ///
    /// # Safety
    ///
    /// The caller must ensure `base` is within a valid range, such that
    /// `base + 16` does not exceed the bounds of the tags array or buckets
    /// array.
    #[inline(always)]
    unsafe fn search_tags(
        &self,
        eq: impl Fn(&V) -> bool,
        tag: u8,
        base: usize,
    ) -> Option<usize> {
        let mut tags = unsafe { self.scan_tags(base, tag) };
        let mut index;
        let mut next_index = tags.trailing_zeros() as usize;

        while tags != 0 {
            index = next_index;
            tags ^= 1 << index;
            next_index = tags.trailing_zeros() as usize;

            let slot = base + index;

            // SAFETY: Caller ensures that `base` is within bounds, as it is derived from
            // a validated bucket and an index within the neighborhood.
            unsafe {
                prefetch(
                    self.buckets_ptr()
                        .as_ref()
                        .as_ptr()
                        .add(base + next_index * usize::from(tags != 0)),
                );
            }

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

        None
    }

    /// Scan 16 bytes starting at bucket for matching tags
    ///
    /// # Safety
    ///
    /// The caller must ensure `bucket` is within a valid range, such that
    /// `bucket + 16` does not exceed the bounds of the tags array.
    #[inline(always)]
    unsafe fn scan_tags(
        &self,
        bucket: usize,
        tag: u8,
    ) -> u16 {
        cfg_if! {
            if #[cfg(all(
                any(target_arch = "x86_64", target_arch = "x86"),
                target_feature = "sse2"
            ))] {
                // SAFETY: We have validated the bucket bounds, as per the requirements of
                // `scan_tags`.
                unsafe { self.scan_tags_sse2(bucket, tag) }
            } else {
                let meta_ptr = self.tags_ptr();
                let mut tags: u16 = 0;
                for i in 0..LANES {
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
    }

    /// SSE2 optimized version of scan_tags
    ///
    /// # Safety
    ///
    /// The caller must ensure `bucket` is within a valid range, such that
    /// `bucket + 16` does not exceed the bounds of the tags array. This
    /// relies on `EMPTY` (0x80) using the sign bit for complementary SIMD
    /// scans.
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "x86"),
        target_feature = "sse2"
    ))]
    #[inline(always)]
    unsafe fn scan_tags_sse2(
        &self,
        bucket: usize,
        tag: u8,
    ) -> u16 {
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
    fn hopmap_index(
        &self,
        hash: u64,
    ) -> usize {
        (hash as usize) & self.max_root_mask
    }

    #[inline(always)]
    fn absolute_index(
        &self,
        hop_bucket: usize,
        n_index: usize,
    ) -> usize {
        hop_bucket * LANES + n_index
    }

    /// Perform a vacant lookup, finding or creating a suitable slot for
    /// insertion
    ///
    /// # Safety
    ///
    /// The caller must ensure that `hop_bucket` is within the bounds of the
    /// hopmap array.
    #[inline]
    unsafe fn do_vacant_lookup(
        &mut self,
        hash: u64,
        hop_bucket: usize,
        rehash: &dyn Fn(&V) -> u64,
    ) -> VacantEntry<'_, V> {
        debug_assert!(hop_bucket <= self.max_root_mask);
        let empty_idx = unsafe { self.find_next_unoccupied(self.absolute_index(hop_bucket, 0)) };

        if empty_idx.is_none()
            || empty_idx.unwrap() >= self.absolute_index(self.max_root_mask + 1 + HOP_RANGE, 0)
        {
            self.resize_rehash(rehash);
            // SAFETY: After resizing, the table has a new `max_root_mask`. The call to
            // `self.hopmap_index(hash)` computes a *new* `hop_bucket` that is valid for
            // the resized table (guaranteed by `hopmap_index` to be <= new
            // `max_root_mask`). This new bucket is then safely passed to the
            // recursive `do_vacant_lookup` call.
            return unsafe { self.do_vacant_lookup(hash, self.hopmap_index(hash), rehash) };
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
                n_index: absolute_empty_idx - hop_bucket * LANES,
            };
        }

        while absolute_empty_idx >= self.absolute_index(hop_bucket + HOP_RANGE, 0) {
            let bubble_base = absolute_empty_idx - (HOP_RANGE - 1) * LANES;

            // SAFETY: We have ensured that `bubble_base` and `absolute_empty_idx` are
            // within the table bounds.
            if let Some((absolute_idx, moved_hash)) = unsafe {
                find_next_movable_index(
                    self.buckets_ptr().as_ref(),
                    bubble_base,
                    absolute_empty_idx,
                    self.max_root_mask,
                    rehash,
                )
            } {
                // SAFETY: We have validated `absolute_idx` through `find_next_movable_index`,
                // ensuring it is within bounds.
                unsafe {
                    let buckets_ptr = self.buckets_ptr().as_mut().as_mut_ptr();
                    debug_assert_ne!(absolute_idx, absolute_empty_idx);

                    core::ptr::copy_nonoverlapping(
                        buckets_ptr.add(absolute_idx),
                        buckets_ptr.add(absolute_empty_idx),
                        1,
                    );

                    let hopmap_root = self.hopmap_index(moved_hash);
                    let hopmap_abs_idx = self.absolute_index(hopmap_root, 0);

                    let old_off_abs = absolute_idx - hopmap_abs_idx;
                    let old_n_index = old_off_abs / LANES;
                    let new_off_abs = absolute_empty_idx - hopmap_abs_idx;
                    let new_n_index = new_off_abs / LANES;

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
                self.resize_rehash(rehash);
                // SAFETY: We have ensured `hop_bucket` is within the hopmap bounds.
                return unsafe { self.do_vacant_lookup(hash, self.hopmap_index(hash), rehash) };
            }
        }

        // SAFETY: We have validated `absolute_empty_idx` through
        // `find_next_unoccupied`.
        debug_assert!(unsafe { !self.is_occupied(absolute_empty_idx) });
        VacantEntry {
            n_index: absolute_empty_idx - hop_bucket * LANES,
            table: self,
            hopmap_root: hop_bucket,
            hash,
        }
    }

    /// Check if the slot at index is occupied
    ///
    /// # Safety
    ///
    /// The caller must ensure `index` is within the bounds of the tags array.
    #[inline(always)]
    unsafe fn is_occupied(
        &self,
        index: usize,
    ) -> bool {
        // SAFETY: Caller ensures `index` is within bounds of the tags array
        unsafe { *self.tags_ptr().as_ref().get_unchecked(index) != EMPTY }
    }

    /// Clear the occupied tag at index
    ///
    /// # Safety
    ///
    /// The caller must ensure `index` is within the bounds of the tags array.
    #[inline(always)]
    unsafe fn clear_occupied(
        &mut self,
        index: usize,
    ) {
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
    unsafe fn set_occupied(
        &mut self,
        index: usize,
        tag: u8,
    ) {
        // SAFETY: Caller ensures `index` is within bounds of the tags array
        unsafe {
            debug_assert!(tag != EMPTY);
            *self.tags_ptr().as_mut().get_unchecked_mut(index) = tag;
        }
    }

    /// Find the next unoccupied index in the range starting from `start` and
    /// only examining the next `HOP_RANGE * LANES` slots.
    ///
    /// # Safety
    ///
    /// The caller must ensure `start` is within the bounds of the tags array
    /// and that there are at least `HOP_RANGE * LANES` slots available from
    /// `start`.
    #[inline(always)]
    unsafe fn find_next_unoccupied_in_range(
        &self,
        start: usize,
    ) -> Option<usize> {
        cfg_if! {
            if #[cfg(all(
                any(target_arch = "x86_64", target_arch = "x86"),
                target_feature = "sse2"
            ))] {
                // SAFETY: Caller ensures `start` is within bounds and there are enough slots
                // to examine.
                unsafe { self.find_next_unoccupied_in_range_sse2(start) }
            } else {
                let end = start + HOP_RANGE * LANES;
                let meta_ptr = self.tags_ptr();
                for i in start..end {
                    // SAFETY: Caller ensures `i` is within bounds of the tags array
                    let t = unsafe { *meta_ptr.as_ref().get_unchecked(i) };
                    if t == EMPTY {
                        return Some(i);
                    }
                }
                None
            }
        }
    }

    /// SSE2 optimized version of find_next_unoccupied_in_range
    ///
    /// # Safety
    ///
    /// The caller must ensure `start` is within the bounds of the tags array
    /// and that there are at least `HOP_RANGE * LANES` slots available from
    /// `start`. This relies on `EMPTY` (0x80) having the sign bit set for
    /// `movemask` to find empty slots.
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "x86"),
        target_feature = "sse2"
    ))]
    unsafe fn find_next_unoccupied_in_range_sse2(
        &self,
        start: usize,
    ) -> Option<usize> {
        unsafe {
            let meta_ptr = self.tags_ptr();
            let tags_ptr = meta_ptr.as_ref().as_ptr().add(start);
            let len = (meta_ptr.as_ref().len()).saturating_sub(start);
            let end = HOP_RANGE * LANES.min(len);

            let mut offset = 0;
            while offset + LANES <= end {
                let data = _mm_loadu_si128(tags_ptr.add(offset) as *const __m128i);
                let mask = _mm_movemask_epi8(data) as u16;

                if mask != 0 {
                    let tz = mask.trailing_zeros() as usize;
                    return Some(start + offset + tz);
                }

                offset += LANES;
            }

            while offset < end {
                let byte = *tags_ptr.add(offset);
                if byte == EMPTY {
                    return Some(start + offset);
                }
                offset += 1;
            }

            None
        }
    }

    /// Find the next unoccupied index starting from `start`
    ///
    /// # Safety
    ///
    /// The caller must ensure `start` is within the bounds of the tags array.
    #[inline(always)]
    unsafe fn find_next_unoccupied(
        &self,
        start: usize,
    ) -> Option<usize> {
        // SAFETY: start is validated to be within table bounds by caller
        unsafe {
            cfg_if! {
                if #[cfg(all(
                    any(target_arch = "x86_64", target_arch = "x86"),
                    target_feature = "sse2"
                ))] {
                    self.find_next_unoccupied_sse2(start)
                } else {
                    self.tags_ptr().as_ref()[start..]
                        .iter()
                        .position(|&b| b == EMPTY)
                        .map(|idx| idx + start)
                }
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
    #[inline(always)]
    unsafe fn find_next_unoccupied_sse2(
        &self,
        start: usize,
    ) -> Option<usize> {
        unsafe {
            let meta_ptr = self.tags_ptr();
            let tags_ptr = meta_ptr.as_ref().as_ptr().add(start);
            let len = (meta_ptr.as_ref().len()).saturating_sub(start);

            let mut offset = 0;
            while offset + LANES <= len {
                let data = _mm_loadu_si128(tags_ptr.add(offset) as *const __m128i);
                let mask = _mm_movemask_epi8(data) as u16;

                if mask != 0 {
                    let tz = mask.trailing_zeros() as usize;
                    return Some(start + offset + tz);
                }

                offset += LANES;
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
    #[inline]
    pub fn find(
        &self,
        hash: u64,
        eq: impl Fn(&V) -> bool,
    ) -> Option<&V> {
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

        None
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
    #[inline]
    pub fn find_mut(
        &mut self,
        hash: u64,
        eq: impl Fn(&V) -> bool,
    ) -> Option<&mut V> {
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

        None
    }

    #[inline]
    fn maybe_resize_rehash(
        &mut self,
        rehash: &dyn Fn(&V) -> u64,
    ) {
        if self.populated >= self.max_pop {
            self.resize_rehash(rehash);
        }
    }

    #[inline]
    #[cold]
    fn resize_rehash(
        &mut self,
        rehash: &dyn Fn(&V) -> u64,
    ) {
        let capacity = self.max_root_mask.wrapping_add(1).max(HOP_RANGE) + 1;
        let capacity: Capacity = capacity.into();

        self.do_resize_rehash(capacity, rehash);
    }

    #[inline]
    fn do_resize_rehash(
        &mut self,
        capacity: Capacity,
        rehash: &dyn Fn(&V) -> u64,
    ) {
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
        let old_empty_words = old_base * LANES;
        self.max_pop = target_load_factor(capacity.base * LANES);
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
        let mut needing_resize = Vec::new();
        // SAFETY: old_alloc valid, old_empty_words calculated from valid old capacity
        let old_emptymap: NonNull<[u8]> = unsafe {
            NonNull::slice_from_raw_parts(
                old_alloc.add(old_layout.tags_offset).cast(),
                old_empty_words,
            )
        };
        let old_buckets: NonNull<[MaybeUninit<V>]> = unsafe {
            NonNull::slice_from_raw_parts(
                old_alloc.add(old_layout.buckets_offset).cast(),
                old_empty_words,
            )
        };

        // SAFETY: This block moves all initialized values from the old allocation to
        // the new one. The safety of this operation relies on the following:
        // - The old allocation is valid and contains `self.populated` initialized
        //   elements, which are correctly identified by the `old_emptymap` (tags).
        // - We iterate through the old tags. For each non-empty tag, we read the value
        //   with `assume_init_read`, which is safe because the tag marks it as
        //   initialized.
        // - Each value is then re-inserted into the new table. The insertion logic,
        //   including bubbling, involves `unsafe` operations (`get_unchecked`, pointer
        //   arithmetic, calls to other `unsafe fn`). These are safe because all
        //   accesses are bounded by the new table's capacity, and the logic correctly
        //   maintains the hopscotch invariants.
        // - After moving, the old allocation is deallocated without dropping the
        //   moved-out values, which is correct as ownership has been transferred.
        unsafe {
            // Ownership note: We move values (V) out of the old allocation into the new
            // one. The old allocation is then deallocated without running destructors for
            // moved-out contents; only the new table will drop values.
            self.populated = 0;

            'tags: for (bucket_index, &tag) in old_emptymap.as_ref().iter().enumerate() {
                if tag == EMPTY {
                    continue;
                }

                if old_base < capacity.base {
                    // Prefetch the two possible destination locations for this item.
                    // Due to power-of-2 mask expansion, an item at old root bucket R will
                    // hash to either new root bucket R or R + old_max_root.
                    let old_root_bucket = bucket_index / LANES;
                    prefetch(self.hopmap_ptr().as_ref().as_ptr().add(old_root_bucket));
                    prefetch(
                        self.hopmap_ptr()
                            .as_ref()
                            .as_ptr()
                            .add(old_root_bucket + old_max_root),
                    );
                    prefetch(self.buckets_ptr().as_ref().as_ptr().add(bucket_index));
                    prefetch(
                        self.buckets_ptr()
                            .as_ref()
                            .as_ptr()
                            .add(bucket_index + old_max_root * LANES),
                    );
                }

                let value = old_buckets
                    .as_ref()
                    .get_unchecked(bucket_index)
                    .assume_init_read();

                let hash = rehash(&value);

                let bucket = self.hopmap_index(hash);
                let base = self.absolute_index(bucket, 0);

                let absolute_empty_idx = match self.find_next_unoccupied(base) {
                    Some(mut idx) => {
                        debug_assert!(!self.is_occupied(idx));
                        // Bubble the empty slot backward until it's within the neighborhood.
                        // Loop invariant: `idx` remains a valid slot index throughout, initially
                        // found by `find_next_unoccupied` and updated by `find_next_movable_index`
                        // to maintain `idx < absolute_index(max_root_mask + 1 + HOP_RANGE, 0)`.
                        while idx >= self.absolute_index(bucket + HOP_RANGE, 0) {
                            let bubble_base = idx - (HOP_RANGE - 1) * LANES;

                            if let Some((absolute_idx, moved_hash)) = find_next_movable_index(
                                self.buckets_ptr().as_ref(),
                                bubble_base,
                                idx,
                                self.max_root_mask,
                                &rehash,
                            ) {
                                core::ptr::copy_nonoverlapping(
                                    self.buckets_ptr().as_ref().as_ptr().add(absolute_idx),
                                    self.buckets_ptr().as_mut().as_mut_ptr().add(idx),
                                    1,
                                );

                                let hopmap_root = self.hopmap_index(moved_hash);
                                let hopmap_abs_idx = self.absolute_index(hopmap_root, 0);

                                let old_off_abs = absolute_idx - hopmap_abs_idx;
                                let old_n_index = old_off_abs / LANES;
                                let new_off_abs = idx - hopmap_abs_idx;
                                let new_n_index = new_off_abs / LANES;

                                self.hopmap_ptr()
                                    .as_mut()
                                    .get_unchecked_mut(hopmap_root)
                                    .clear(old_n_index);
                                self.hopmap_ptr()
                                    .as_mut()
                                    .get_unchecked_mut(hopmap_root)
                                    .set(new_n_index);

                                self.clear_occupied(absolute_idx);
                                self.set_occupied(idx, hashtag(moved_hash));
                                idx = absolute_idx;
                            } else {
                                needing_resize.push((value, hash));
                                continue 'tags;
                            }
                        }
                        idx
                    }
                    None => {
                        needing_resize.push((value, hash));
                        continue;
                    }
                };

                self.populated += 1;

                let n_index = (absolute_empty_idx - base) / LANES;

                self.set_occupied(absolute_empty_idx, hashtag(hash));
                self.hopmap_ptr()
                    .as_mut()
                    .get_unchecked_mut(bucket)
                    .set(n_index);

                self.buckets_ptr()
                    .as_mut()
                    .get_unchecked_mut(absolute_empty_idx)
                    .write(value);
            }

            for (needs_resize, hash) in needing_resize {
                let bucket = self.hopmap_index(hash);
                self.do_vacant_lookup(hash, bucket, rehash)
                    .insert(needs_resize);
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
    /// # Load Factor
    ///
    /// The table maintains a load factor of approximately 87.5% (configurable
    /// up to 97%) before triggering a resize operation.
    pub fn capacity(&self) -> usize {
        self.max_pop
    }

    /// Computes a histogram of probe lengths and bucket distribution for the
    /// current table state.
    ///
    /// This method is intended for debugging and performance analysis. It
    /// returns a [`ProbeHistogram`] struct containing detailed statistics
    /// about probe lengths and how entries are distributed relative to
    /// their ideal buckets.
    #[cfg(feature = "stats")]
    pub fn probe_histogram(&self) -> ProbeHistogram {
        let mut probe_hist = ProbeHistogram {
            populated: self.populated,
            buckets: self.max_root_mask.wrapping_add(1) + HOP_RANGE,
            probe_length_by_bucket: [0; HOP_RANGE],
            probe_length_by_count: [0; HOP_RANGE],
            bucket_distribution: [0; HOP_RANGE],
        };

        if self.populated == 0 {
            return probe_hist;
        }

        // SAFETY: The call to `hopmap_ptr().as_ref()` is unsafe, but is safe here
        // because `self` is a valid `HashTable`. An initialized table guarantees
        // that `hopmap_ptr()` returns a valid pointer and length for the hopmap slice.
        // The rest of the operations are safe as they operate on the valid slice.
        unsafe {
            for bucket in self.hopmap_ptr().as_ref().iter() {
                let mut mask = bucket.candidates();
                if mask != 0 {
                    probe_hist.probe_length_by_bucket[mask.count_ones() as usize - 1] += bucket
                        .neighbors
                        .iter()
                        .copied()
                        .map(|n| usize::from(n > 0))
                        .sum::<usize>();
                    probe_hist.probe_length_by_count[mask.count_ones() as usize - 1] += bucket
                        .neighbors
                        .iter()
                        .copied()
                        .map(|n| n as usize)
                        .sum::<usize>();

                    while mask != 0 {
                        let n_index = mask.trailing_zeros() as usize;
                        mask ^= 1 << n_index;
                        probe_hist.bucket_distribution[n_index] +=
                            bucket.neighbors[n_index] as usize;
                    }
                }
            }
        }

        probe_hist
    }

    /// Returns detailed performance and utilization statistics for debugging.
    #[cfg(feature = "stats")]
    pub fn debug_stats(&self) -> DebugStats {
        let total_slots = if self.max_root_mask == usize::MAX {
            0
        } else {
            (self.max_root_mask.wrapping_add(1) + HOP_RANGE) * LANES
        };

        let mut occupied_slots = 0;

        if total_slots > 0 {
            // SAFETY: The call to the `unsafe` function `is_occupied` is safe here
            // because we are iterating from `0` to `total_slots`, which is the
            // exact size of the tags array. This ensures that the index `i` is
            // always within bounds.
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

    /// Retains only the elements specified by the predicate.
    ///
    /// The predicate is a closure that takes a reference to a value and returns
    /// `true` if the value should be kept in the table, or `false` if it should
    /// be removed.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that determines whether to retain each value
    /// * `rehash` - A closure that computes the hash for a value, used to
    ///   update the hopmap when removing entries
    pub fn retain(
        &mut self,
        mut f: impl FnMut(&V) -> bool,
        rehash: impl Fn(&V) -> u64,
    ) {
        self.retain_mut(|v| f(v), rehash);
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// The predicate is a closure that takes a reference to a value and returns
    /// `true` if the value should be kept in the table, or `false` if it should
    /// be removed.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that determines whether to retain each value
    /// * `rehash` - A closure that computes the hash for a value, used to
    ///   update the hopmap when removing entries
    #[inline]
    pub fn retain_mut(
        &mut self,
        mut f: impl FnMut(&mut V) -> bool,
        rehash: impl Fn(&V) -> u64,
    ) {
        if self.populated == 0 {
            return;
        }

        for idx in 0..(self.max_root_mask.wrapping_add(1) + HOP_RANGE) * LANES {
            // SAFETY: The call to the `unsafe` function `is_occupied` is safe here
            // because we are iterating from `0` to the total number of slots,
            // which is the exact size of the tags array. This ensures that the
            // index `idx` is always within bounds.
            if unsafe { self.is_occupied(idx) } {
                // SAFETY: We have validated `idx` through `is_occupied`, and the bucket
                // is confirmed to be initialized by an occupied tag.
                let value = unsafe {
                    self.buckets_ptr()
                        .as_mut()
                        .get_unchecked_mut(idx)
                        .assume_init_mut()
                };
                if !f(value) {
                    self.populated -= 1;
                    // SAFETY: We have validated `idx` through `is_occupied`, and the bucket
                    // is confirmed to be initialized by an occupied tag.
                    unsafe {
                        self.clear_occupied(idx);
                        let hash = rehash(value);
                        let hop_bucket = self.hopmap_index(hash);
                        self.hopmap_ptr()
                            .as_mut()
                            .get_unchecked_mut(hop_bucket)
                            .clear((idx - hop_bucket * LANES) / LANES);

                        self.buckets_ptr()
                            .as_mut()
                            .get_unchecked_mut(idx)
                            .assume_init_drop();
                    }
                }
            }
        }
    }

    /// Creates an iterator that removes all elements matching a predicate.
    ///
    /// The predicate is a closure that takes a reference to a value and returns
    /// `true` if the value should be removed from the table, or `false` if it
    /// should be kept.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that determines whether to extract each value
    /// * `rehash` - A closure that computes the hash for a value, used to
    ///   update the hopmap when removing entries
    pub fn extract_if<F, R>(
        &mut self,
        f: F,
        rehash: R,
    ) -> ExtractIf<'_, V, F, R>
    where
        F: FnMut(&mut V) -> bool,
        R: Fn(&V) -> u64,
    {
        ExtractIf {
            table: self,
            index: 0,
            filter: f,
            rehash,
        }
    }
}

impl<V> IntoIterator for HashTable<V> {
    type IntoIter = IntoIter<V>;
    type Item = V;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            table: self,
            index: 0,
        }
    }
}

/// A view into a single entry in the hash table, which may be vacant or
/// occupied.
///
/// This enum is constructed from the [`entry`] method on [`HashTable`].
/// It provides efficient APIs for insertion and modification operations.
///
/// [`entry`]: HashTable::entry
#[derive(Debug)]
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
    pub fn or_insert(
        self,
        default: V,
    ) -> &'a mut V {
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
    pub fn or_insert_with(
        self,
        default: impl FnOnce() -> V,
    ) -> &'a mut V {
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
    pub fn and_modify(
        self,
        f: impl FnOnce(&mut V),
    ) -> Option<&'a mut V> {
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
#[derive(Debug)]
pub struct VacantEntry<'a, V> {
    table: &'a mut HashTable<V>,
    hopmap_root: usize,
    hash: u64,
    n_index: usize,
}

impl<'a, V> VacantEntry<'a, V> {
    /// Inserts a value into the vacant entry and returns a mutable reference to
    /// it.
    ///
    /// The value is inserted at the position determined by the hash and
    /// hopscotch algorithm.
    pub fn insert(
        self,
        value: V,
    ) -> &'a mut V {
        self.table.populated += 1;

        // SAFETY: A `VacantEntry` is only constructed by `do_vacant_lookup` with:
        // - A valid `hopmap_root` where `hopmap_root <= max_root_mask`, ensuring it
        //   indexes a valid root bucket in the hopmap array.
        // - A valid, unoccupied `n_index` that is guaranteed to be in the
        //   hop-neighborhood (n_index < HOP_RANGE * LANES), ensuring the entry stays
        //   within the root's neighborhood.
        // This guarantees that `neighbor = n_index / LANES` is a valid neighbor index
        // (< HOP_RANGE) and that `target_index = hopmap_root * LANES + n_index` is a
        // valid, unoccupied slot within the table's bounds. Therefore, the `unsafe`
        // operations (`set`, `set_occupied`, `get_unchecked_mut`, and `write`) are
        // safe.
        unsafe {
            let neighbor = self.n_index / LANES;
            debug_assert!(neighbor < HOP_RANGE);
            self.table
                .hopmap_ptr()
                .as_mut()
                .get_unchecked_mut(self.hopmap_root)
                .set(neighbor);

            let target_index = self.hopmap_root * LANES + self.n_index;
            self.table.set_occupied(target_index, hashtag(self.hash));

            self.table
                .buckets_ptr()
                .as_mut()
                .get_unchecked_mut(target_index)
                .write(value)
        }
    }
}

/// A view into an occupied entry in the hash table.
///
/// This struct is created by the [`entry`] method on [`HashTable`] when the
/// requested key is present in the table. It provides methods to access,
/// modify, or remove the existing value.
///
/// [`entry`]: HashTable::entry
#[derive(Debug)]
pub struct OccupiedEntry<'a, V> {
    table: &'a mut HashTable<V>,
    root_index: usize,
    n_index: usize,
}

// Safety invariant for OccupiedEntry methods:
// An `OccupiedEntry` for an in-table element is only created after
// `search_neighborhood` finds a valid, occupied slot. This guarantees that:
// 1. The calculated absolute index (`root_index * LANES + n_index`) is within
//    the bounds of the `buckets` array.
// 2. The slot is occupied, meaning the `MaybeUninit<V>` contains an initialized
//    value.
// Therefore, `get_unchecked`, `get_unchecked_mut`, `assume_init_ref`, and
// `assume_init_mut` are all safe operations when accessing in-table entries.
impl<'a, V> OccupiedEntry<'a, V> {
    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &V {
        // SAFETY: See safety invariant comment above `impl` block.
        unsafe {
            self.table
                .buckets_ptr()
                .as_ref()
                .get_unchecked(self.root_index * LANES + self.n_index)
                .assume_init_ref()
        }
    }

    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut V {
        // SAFETY: See safety invariant comment above `impl` block.
        unsafe {
            self.table
                .buckets_ptr()
                .as_mut()
                .get_unchecked_mut(self.root_index * LANES + self.n_index)
                .assume_init_mut()
        }
    }

    /// Converts the entry into a mutable reference to the value with the
    /// lifetime of the entry.
    pub fn into_mut(self) -> &'a mut V {
        // SAFETY: See safety invariant comment above `impl` block.
        unsafe {
            self.table
                .buckets_ptr()
                .as_mut()
                .get_unchecked_mut(self.root_index * LANES + self.n_index)
                .assume_init_mut()
        }
    }

    /// Removes the entry from the table and returns the value.
    pub fn remove(self) -> V {
        self.table.populated -= 1;

        // SAFETY: This is safe for the same reasons as `get()`: the entry is
        // guaranteed to point to a valid, initialized element. We can therefore
        // safely read the value with `assume_init_read`. The subsequent calls to
        // `clear` and `clear_occupied` are also safe because the indices are
        // guaranteed to be valid by the invariants of `OccupiedEntry`.
        unsafe {
            let bucket_mut = self
                .table
                .buckets_ptr()
                .as_ref()
                .get_unchecked(self.root_index * LANES + self.n_index);
            let value = bucket_mut.assume_init_read();
            let neighbor = self.n_index / LANES;
            // SAFETY: `self.n_index` is the offset from the root bucket, and is
            // guaranteed to be within the hop-neighborhood by `search_neighborhood`.
            // Therefore, `neighbor` will be a valid neighbor index (< HOP_RANGE).
            self.table
                .hopmap_ptr()
                .as_mut()
                .get_unchecked_mut(self.root_index)
                .clear(neighbor);

            self.table
                .clear_occupied(self.root_index * LANES + self.n_index);

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
pub struct Iter<'a, V> {
    table: &'a HashTable<V>,
    bucket_index: usize,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        if self.table.is_empty() {
            return None;
        }

        // SAFETY: The `unsafe` block is safe because we are iterating through the
        // table's slots within the valid bounds (`0..total_slots`).
        // - We guarded against an empty table at the start of `next()`.
        // - `is_occupied` is safe to call because `self.bucket_index` is always less
        //   than `total_slots`.
        // - `get_unchecked` is safe for the same reason.
        // - `assume_init_ref` is safe because we only call it after `is_occupied`
        //   returns true, which guarantees the slot contains an initialized value.
        unsafe {
            let total_slots = (self.table.max_root_mask.wrapping_add(1) + HOP_RANGE) * LANES;
            while self.bucket_index < total_slots {
                let prefetch_index = self.bucket_index + 1;
                if prefetch_index < total_slots {
                    prefetch(
                        self.table
                            .buckets_ptr()
                            .as_ref()
                            .as_ptr()
                            .add(prefetch_index),
                    );
                }

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

            None
        }
    }
}

/// A mutable iterator over the values in a [`HashTable`].
///
/// This struct is created by the [`iter_mut`] method on [`HashTable`].
/// It yields `&mut V` references in an arbitrary order.
///
/// [`iter_mut`]: HashTable::iter_mut
pub struct IterMut<'a, V> {
    tags: &'a [u8],
    values: &'a mut [MaybeUninit<V>],
}

impl<'a, V> Iterator for IterMut<'a, V> {
    type Item = &'a mut V;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.tags.is_empty() {
            let (first_tag, tags) = core::mem::take(&mut self.tags).split_first().unwrap();
            self.tags = tags;
            if *first_tag != EMPTY {
                let (first_value, values) =
                    core::mem::take(&mut self.values).split_first_mut().unwrap();
                self.values = values;
                return Some(unsafe { first_value.assume_init_mut() });
            } else {
                let (_first_value, values) =
                    core::mem::take(&mut self.values).split_first_mut().unwrap();
                self.values = values;
            }
        }

        None
    }
}

/// A draining iterator over the values in a [`HashTable`].
///
/// This struct is created by the [`drain`] method on [`HashTable`].
/// It yields owned `V` values and empties the table as it iterates.
///
/// [`drain`]: HashTable::drain
pub struct Drain<'a, V> {
    occupied: Box<[u8]>,
    total_slots: usize,
    table: &'a mut HashTable<V>,
    bucket_index: usize,
}

impl<V> Drop for Drain<'_, V> {
    fn drop(&mut self) {
        if core::mem::needs_drop::<V>() {
            for _ in &mut *self {}
        }
    }
}

impl<V> Iterator for Drain<'_, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: The `unsafe` block is safe because we are iterating through the
        // table's slots within the valid bounds (`0..total_slots`).
        // - total_slots is initialized to zero if the table is empty
        // - occupied.get_unchecked is safe because `self.bucket_index` is always less
        //   than `total_slots`.
        // - buckets_ptr.`get_unchecked` is safe for the same reason.
        // - `assume_init_read` is safe because we only call it after `is_occupied`
        //   returns true, and we take ownership of the value.
        unsafe {
            while self.bucket_index < self.total_slots {
                let prefetch_index = self.bucket_index + 1;
                if prefetch_index < self.total_slots {
                    prefetch(
                        self.table
                            .buckets_ptr()
                            .as_ref()
                            .as_ptr()
                            .add(prefetch_index),
                    );
                }

                if *self.occupied.get_unchecked(self.bucket_index) != EMPTY {
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

            None
        }
    }
}

/// An owning iterator over the values in a [`HashTable`].
///
/// This struct is created by the `into_iter` method on [`HashTable`].
/// It yields owned `V` values and consumes the table as it iterates.
pub struct IntoIter<V> {
    table: HashTable<V>,
    index: usize,
}

impl<V> Iterator for IntoIter<V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        if self.table.is_empty() {
            return None;
        }

        // SAFETY: The `unsafe` block is safe because we are iterating through the
        // table's slots within the valid bounds (`0..total_slots`).
        // - We guarded against an empty table at the start of `next()`.
        // - `is_occupied` is safe to call because `self.index` is always less than
        //   `total_slots`.
        // - `get_unchecked` is safe for the same reason.
        // - `assume_init_read` is safe because we only call it after `is_occupied`
        //   returns true, which guarantees the slot contains an initialized value.
        unsafe {
            let total_slots = (self.table.max_root_mask.wrapping_add(1) + HOP_RANGE) * LANES;
            while self.index < total_slots {
                if self.table.is_occupied(self.index) {
                    self.table.clear_occupied(self.index);
                    let bucket = self.table.buckets_ptr().as_ref().get_unchecked(self.index);
                    self.index += 1;
                    return Some(bucket.assume_init_read());
                }
                self.index += 1;
            }

            None
        }
    }
}

/// An iterator that removes and yields all elements matching a predicate.
pub struct ExtractIf<'a, V, F, R> {
    table: &'a mut HashTable<V>,
    index: usize,
    filter: F,
    rehash: R,
}

impl<V, F, R> Iterator for ExtractIf<'_, V, F, R>
where
    F: FnMut(&mut V) -> bool,
    R: Fn(&V) -> u64,
{
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        if self.table.is_empty() {
            return None;
        }

        while self.index < (self.table.max_root_mask.wrapping_add(1) + HOP_RANGE) * LANES {
            let idx = self.index;
            self.index += 1;
            // SAFETY: The call to the `unsafe` function `is_occupied` is safe here
            // because we are iterating from `0` to the total number of slots,
            // which is the exact size of the tags array. This ensures that the
            // index `idx` is always within bounds.
            if unsafe { self.table.is_occupied(idx) } {
                // SAFETY: We have validated `idx` through `is_occupied`, and the bucket
                // is confirmed to be initialized by an occupied tag.
                let value = unsafe {
                    self.table
                        .buckets_ptr()
                        .as_mut()
                        .get_unchecked_mut(idx)
                        .assume_init_mut()
                };
                if (self.filter)(value) {
                    self.table.populated -= 1;
                    // SAFETY: We have validated `idx` through `is_occupied`, and the bucket
                    // is confirmed to be initialized by an occupied tag.
                    unsafe {
                        self.table.clear_occupied(idx);
                        let hash = (self.rehash)(value);
                        let hop_bucket = self.table.hopmap_index(hash);
                        self.table
                            .hopmap_ptr()
                            .as_mut()
                            .get_unchecked_mut(hop_bucket)
                            .clear((idx - hop_bucket * LANES) / LANES);

                        return Some(
                            self.table
                                .buckets_ptr()
                                .as_mut()
                                .get_unchecked_mut(idx)
                                .assume_init_read(),
                        );
                    };
                }
            }
        }

        None
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

    fn hash_key(
        state: &HashState,
        key: u64,
    ) -> u64 {
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
            match table.entry(hash, |v: &Item| v.key == k, |v| hash_key(&state, v.key)) {
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

        match table.entry(hash, |v| v.key == k, |v| hash_key(&state, v.key)) {
            Entry::Vacant(v) => {
                v.insert(Item { key: k, value: 7 });
            }
            Entry::Occupied(_) => panic!("should be vacant first time"),
        }

        match table.entry(hash, |v| v.key == k, |v| hash_key(&state, v.key)) {
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
            match table.entry(hash, |v| v.key == k, |v| hash_key(&state, v.key)) {
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
            match table.entry(hash, |v| v.key == k, |v| hash_key(&state, v.key)) {
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
            match table.entry(hash, |v| v.key == k, |v| hash_key(&state, v.key)) {
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
    fn iter_and_drain() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        for k in 10..20u64 {
            let hash = hash_key(&state, k);
            match table.entry(hash, |v| v.key == k, |v| hash_key(&state, v.key)) {
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

    fn hash_string_key(
        state: &HashState,
        key: &str,
    ) -> u64 {
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
            match table.entry(
                hash,
                |v: &StringItem| v.key == *k,
                |v| hash_string_key(&state, &v.key),
            ) {
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
            match table.entry(hash, |v| v.key == *k, |v| hash_string_key(&state, &v.key)) {
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
            table
                .entry(hash, |v| v.key == *k, |v| hash_string_key(&state, &v.key))
                .or_insert(StringItem {
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
            table
                .entry(hash, |v| v.key == *k, |v| hash_string_key(&state, &v.key))
                .or_insert(StringItem {
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
            .entry(hash, |v| v.key == key, |v| hash_string_key(&state, &v.key))
            .or_insert_with(|| StringItem {
                key: key.to_string(),
                value: 42,
            });
        assert_eq!(value_ref.value, 42);

        let existing_ref = table
            .entry(hash, |v| v.key == key, |v| hash_string_key(&state, &v.key))
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
            .entry(
                hash,
                |s: &String| s == "key",
                |v| hash_string_key(&state, v),
            )
            .or_insert("key".to_string());

        let value_ref = match table.entry(
            hash,
            |s: &String| s == "key",
            |v| hash_string_key(&state, v),
        ) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(_) => unreachable!("Entry should be occupied: {:#?}", table),
        };
        *value_ref = "new_value".to_string();
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
                .entry(hash, |v| v.key == *key, |v| hash_string_key(&state, &v.key))
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
    fn test_shrink_to_fit_empty_table() {
        let mut table: HashTable<Item> = HashTable::with_capacity(100);
        let initial_capacity = table.capacity();

        assert!(initial_capacity > 0);
        assert_eq!(table.len(), 0);

        table.shrink_to_fit(|_| panic!("should not be called"));

        assert_eq!(table.len(), 0);
        assert_eq!(table.capacity(), 0);
    }

    #[test]
    fn test_shrink_to_fit_with_items() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(1000);

        for i in 0..50 {
            let hash = hash_key(&state, i);
            table
                .entry(hash, |v| v.key == i, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: i,
                    value: i as i32,
                });
        }

        let initial_capacity = table.capacity();
        assert_eq!(table.len(), 50);
        assert!(initial_capacity >= 1000);

        table.shrink_to_fit(|k| hash_key(&state, k.key));

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
            table
                .entry(hash, |v| v.key == i, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: i,
                    value: i as i32,
                });
        }

        assert_eq!(table.len(), 100);
        let capacity_with_items = table.capacity();

        table.clear();
        assert_eq!(table.len(), 0);
        assert_eq!(table.capacity(), capacity_with_items);

        table.shrink_to_fit(|k| hash_key(&state, k.key));

        assert_eq!(table.len(), 0);
        assert_eq!(table.capacity(), 0);
    }

    #[test]
    fn test_shrink_to_fit_after_removals() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(1000);

        for i in 0..200 {
            let hash = hash_key(&state, i);
            table
                .entry(hash, |v| v.key == i, |v| hash_key(&state, v.key))
                .or_insert(Item {
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

        table.shrink_to_fit(|k| hash_key(&state, k.key));

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
    fn test_shrink_to_fit_no_change_when_optimal() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for i in 0..50 {
            let hash = hash_key(&state, i);
            table
                .entry(hash, |v| v.key == i, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: i,
                    value: i as i32,
                });
        }

        table.shrink_to_fit(|k| hash_key(&state, k.key));
        let optimal_capacity = table.capacity();

        table.shrink_to_fit(|k| hash_key(&state, k.key));
        let capacity_after_second_shrink = table.capacity();

        assert_eq!(table.len(), 50);
        assert_eq!(optimal_capacity, capacity_after_second_shrink);

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
            table
                .entry(hash, |v| v.key == i, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: i,
                    value: i as i32,
                });
        }

        table.shrink_to_fit(|k| hash_key(&state, k.key));

        let new_hash = hash_key(&state, 999);
        table
            .entry(new_hash, |v| v.key == 999, |v| hash_key(&state, v.key))
            .or_insert(Item {
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

    #[test]
    fn comprehensive_full_drain_with_reuse() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..8u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        assert_eq!(table.len(), 8);

        let drained: Vec<Item> = table.drain().collect();
        assert_eq!(drained.len(), 8);
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());

        let drained_keys: Vec<u64> = drained.iter().map(|item| item.key).collect();
        for k in 0..8u64 {
            assert!(drained_keys.contains(&k), "Should have drained key {}", k);
        }

        for k in 20..25u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: (k as i32) + 100,
                });
        }

        assert_eq!(table.len(), 5);
        assert!(!table.is_empty());

        for k in 20..25u64 {
            let hash = hash_key(&state, k);
            let found = table.find(hash, |v| v.key == k);
            assert!(found.is_some(), "Should find new key {}", k);
            assert_eq!(found.unwrap().value, (k as i32) + 100);
        }

        for k in 0..8u64 {
            let hash = hash_key(&state, k);
            assert!(
                table.find(hash, |v| v.key == k).is_none(),
                "Should not find old key {}",
                k
            );
        }
    }

    #[test]
    fn comprehensive_drain_edge_cases() {
        let state = HashState::default();

        {
            let mut empty_table: HashTable<Item> = HashTable::with_capacity(0);
            let drained: Vec<Item> = empty_table.drain().collect();
            assert_eq!(drained.len(), 0);
            assert_eq!(empty_table.len(), 0);
            assert!(empty_table.is_empty());
        }

        {
            let mut single_table: HashTable<Item> = HashTable::with_capacity(0);
            let hash = hash_key(&state, 42);
            single_table
                .entry(hash, |v| v.key == 42, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: 42,
                    value: 123,
                });

            assert_eq!(single_table.len(), 1);

            let drained: Vec<Item> = single_table.drain().collect();
            assert_eq!(drained.len(), 1);
            assert_eq!(drained[0].key, 42);
            assert_eq!(drained[0].value, 123);
            assert_eq!(single_table.len(), 0);
            assert!(single_table.is_empty());
        }

        {
            let mut single_table: HashTable<Item> = HashTable::with_capacity(0);
            let hash = hash_key(&state, 99);
            single_table
                .entry(hash, |v| v.key == 99, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: 99,
                    value: 456,
                });

            let mut drainer = single_table.drain();
            let item = drainer.next().unwrap();
            assert_eq!(item.key, 99);
            assert_eq!(item.value, 456);

            assert!(drainer.next().is_none());
            drop(drainer);

            assert_eq!(single_table.len(), 0);
            assert!(single_table.is_empty());
        }
    }

    #[test]
    fn retain_empty_table() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        table.retain(|_| true, |v| hash_key(&state, v.key));
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn retain_all() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        assert_eq!(table.len(), 100);
        table.retain(|_| true, |v| hash_key(&state, v.key));
        assert_eq!(table.len(), 100);

        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            assert!(table.find(hash, |v| v.key == k).is_some());
        }
    }

    #[test]
    fn retain_none() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        assert_eq!(table.len(), 100);
        table.retain(|_| false, |v| hash_key(&state, v.key));
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());

        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            assert!(table.find(hash, |v| v.key == k).is_none());
        }
    }

    #[test]
    fn retain_even_keys() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        assert_eq!(table.len(), 100);
        table.retain(|item| item.key % 2 == 0, |v| hash_key(&state, v.key));
        assert_eq!(table.len(), 50);

        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            if k % 2 == 0 {
                assert!(
                    table.find(hash, |v| v.key == k).is_some(),
                    "even key {} should be present",
                    k
                );
            } else {
                assert!(
                    table.find(hash, |v| v.key == k).is_none(),
                    "odd key {} should be absent",
                    k
                );
            }
        }
    }

    #[test]
    fn retain_by_value() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: (k as i32) * 10,
                });
        }

        table.retain(|item| item.value < 250, |v| hash_key(&state, v.key));
        assert_eq!(table.len(), 25);

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            let expected_value = (k as i32) * 10;
            if expected_value < 250 {
                assert!(table.find(hash, |v| v.key == k).is_some());
            } else {
                assert!(table.find(hash, |v| v.key == k).is_none());
            }
        }
    }

    #[test]
    fn retain_mut_empty_table() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        table.retain_mut(|_| true, |v| hash_key(&state, v.key));
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn retain_mut_modify_and_keep() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        table.retain_mut(
            |item| {
                item.value *= 2;
                true
            },
            |v| hash_key(&state, v.key),
        );

        assert_eq!(table.len(), 50);
        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            let item = table.find(hash, |v| v.key == k).unwrap();
            assert_eq!(item.value, (k as i32) * 2);
        }
    }

    #[test]
    fn retain_mut_modify_and_filter() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        table.retain_mut(
            |item| {
                item.value += 1;
                item.key % 2 == 0
            },
            |v| hash_key(&state, v.key),
        );

        assert_eq!(table.len(), 50);
        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            if k % 2 == 0 {
                let item = table.find(hash, |v| v.key == k).unwrap();
                assert_eq!(item.value, (k as i32) + 1);
            } else {
                assert!(table.find(hash, |v| v.key == k).is_none());
            }
        }
    }

    #[test]
    fn retain_mut_conditional_modification() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        table.retain_mut(
            |item| {
                if item.value < 25 {
                    item.value = -item.value;
                }
                true
            },
            |v| hash_key(&state, v.key),
        );

        assert_eq!(table.len(), 50);
        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            let item = table.find(hash, |v| v.key == k).unwrap();
            if k < 25 {
                assert_eq!(item.value, -(k as i32));
            } else {
                assert_eq!(item.value, k as i32);
            }
        }
    }

    #[test]
    fn extract_if_empty_table() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);
        let extracted: Vec<Item> = table
            .extract_if(|_| true, |v| hash_key(&state, v.key))
            .collect();
        assert_eq!(extracted.len(), 0);
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn extract_if_none() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        let extracted: Vec<Item> = table
            .extract_if(|_| false, |v| hash_key(&state, v.key))
            .collect();

        assert_eq!(extracted.len(), 0);
        assert_eq!(table.len(), 50);

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            assert!(table.find(hash, |v| v.key == k).is_some());
        }
    }

    #[test]
    fn extract_if_all() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        let extracted: Vec<Item> = table
            .extract_if(|_| true, |v| hash_key(&state, v.key))
            .collect();

        assert_eq!(extracted.len(), 50);
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());

        for k in 0..50u64 {
            assert!(extracted.iter().any(|item| item.key == k));
        }
    }

    #[test]
    fn extract_if_odd_keys() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        let extracted: Vec<Item> = table
            .extract_if(|item| item.key % 2 == 1, |v| hash_key(&state, v.key))
            .collect();

        assert_eq!(extracted.len(), 50);
        assert_eq!(table.len(), 50);

        for item in &extracted {
            assert_eq!(item.key % 2, 1);
        }

        for k in (0..100u64).step_by(2) {
            let hash = hash_key(&state, k);
            assert!(table.find(hash, |v| v.key == k).is_some());
        }

        for k in (1..100u64).step_by(2) {
            let hash = hash_key(&state, k);
            assert!(table.find(hash, |v| v.key == k).is_none());
        }
    }

    #[test]
    fn extract_if_by_value() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: (k as i32) * 10,
                });
        }

        let extracted: Vec<Item> = table
            .extract_if(|item| item.value >= 250, |v| hash_key(&state, v.key))
            .collect();

        assert_eq!(extracted.len(), 25);
        assert_eq!(table.len(), 25);

        for item in &extracted {
            assert!(item.value >= 250);
        }

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            let expected_value = (k as i32) * 10;
            if expected_value >= 250 {
                assert!(table.find(hash, |v| v.key == k).is_none());
            } else {
                assert!(table.find(hash, |v| v.key == k).is_some());
            }
        }
    }

    #[test]
    fn extract_if_partial_consumption() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..100u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        let original_len = table.len();
        let mut extractor = table.extract_if(|item| item.key % 2 == 0, |v| hash_key(&state, v.key));

        let mut extracted = Vec::new();
        for _ in 0..10 {
            if let Some(item) = extractor.next() {
                extracted.push(item);
            }
        }

        assert_eq!(extracted.len(), 10);

        assert!(table.len() < original_len);
        assert!(!table.is_empty());
    }

    #[test]
    fn extract_if_with_mutation() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..50u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        let extracted: Vec<Item> = table
            .extract_if(
                |item| {
                    let original_key = item.key;
                    item.value += 100;
                    original_key >= 25
                },
                |v| hash_key(&state, v.key),
            )
            .collect();

        assert_eq!(extracted.len(), 25);

        for item in &extracted {
            assert!(item.value >= 125);
            assert!(item.key >= 25);
        }

        assert_eq!(table.len(), 25);
        for item in table.iter() {
            assert!(item.key < 25);
            assert!(item.value >= 100);
            assert!(item.value < 125);
        }
    }

    #[test]
    fn retain_with_drop_counting() {
        use alloc::rc::Rc;
        use core::cell::RefCell;

        let state = HashState::default();
        let mut table: HashTable<Rc<RefCell<usize>>> = HashTable::with_capacity(0);

        for k in 0..50u64 {
            let tracked = Rc::new(RefCell::new(k as usize));
            let hash = hash_key(&state, k);
            table
                .entry(
                    hash,
                    |v| *v.borrow() == k as usize,
                    |v| hash_key(&state, *v.borrow() as u64),
                )
                .or_insert(tracked);
        }

        let initial_len = table.len();
        table.retain(
            |item| *item.borrow() < 25,
            |v| hash_key(&state, *v.borrow() as u64),
        );

        assert_eq!(table.len(), 25);
        assert_eq!(initial_len, 50);
    }

    #[test]
    fn extract_if_maintains_table_integrity() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(0);

        for k in 0..500u64 {
            let hash = hash_key(&state, k);
            table
                .entry(hash, |v| v.key == k, |v| hash_key(&state, v.key))
                .or_insert(Item {
                    key: k,
                    value: k as i32,
                });
        }

        let _extracted: Vec<Item> = table
            .extract_if(|item| item.key % 3 == 0, |v| hash_key(&state, v.key))
            .collect();

        for k in 0..500u64 {
            let hash = hash_key(&state, k);
            let found = table.find(hash, |v| v.key == k);
            if k % 3 == 0 {
                assert!(found.is_none(), "key {} should be extracted", k);
            } else {
                assert!(found.is_some(), "key {} should remain", k);
                assert_eq!(found.unwrap().value, k as i32);
            }
        }
    }

    #[test]
    fn try_entry_vacant_success() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(100);

        let k = 42u64;
        let hash = hash_key(&state, k);

        match table.try_entry(hash, |v| v.key == k) {
            Ok(Entry::Vacant(v)) => {
                v.insert(Item { key: k, value: 100 });
            }
            Ok(Entry::Occupied(_)) => panic!("should be vacant"),
            Err(e) => panic!("should succeed: {:?}", e),
        }

        assert_eq!(table.len(), 1);
        assert_eq!(
            table.find(hash, |v| v.key == k),
            Some(&Item { key: k, value: 100 })
        );
    }

    #[test]
    fn try_entry_occupied_success() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(100);

        let k = 42u64;
        let hash = hash_key(&state, k);

        match table.try_entry(hash, |v| v.key == k) {
            Ok(Entry::Vacant(v)) => {
                v.insert(Item { key: k, value: 100 });
            }
            _ => panic!("first should be vacant"),
        }

        match table.try_entry(hash, |v| v.key == k) {
            Ok(Entry::Occupied(mut occ)) => {
                assert_eq!(occ.get().value, 100);
                occ.get_mut().value = 200;
            }
            Ok(Entry::Vacant(_)) => panic!("should be occupied"),
            Err(e) => panic!("should succeed: {:?}", e),
        }

        assert_eq!(table.len(), 1);
        assert_eq!(
            table.find(hash, |v| v.key == k),
            Some(&Item { key: k, value: 200 })
        );
    }

    #[test]
    fn try_entry_capacity_too_small() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(10);

        let capacity = table.capacity();
        for k in 0..capacity as u64 {
            let hash = hash_key(&state, k);
            match table.try_entry(hash, |v| v.key == k) {
                Ok(Entry::Vacant(v)) => {
                    v.insert(Item {
                        key: k,
                        value: k as i32,
                    });
                }
                Err(e) => panic!("should succeed before capacity: {:?}", e),
                _ => panic!("should be vacant"),
            }
        }

        assert_eq!(table.len(), capacity);

        let k = capacity as u64;
        let hash = hash_key(&state, k);
        match table.try_entry(hash, |v| v.key == k) {
            Err(TryEntryError::CapacityTooSmall) => {}
            Ok(_) => panic!("should fail with CapacityTooSmall"),
            Err(e) => panic!("wrong error: {:?}", e),
        }
    }

    #[test]
    fn try_entry_no_free_slot_in_range() {
        let mut table: HashTable<Item> = HashTable::with_capacity(100);

        let capacity = table.capacity();
        let target_fill = (capacity * 9) / 10;

        let mut inserted = 0;
        let hash = 0u64;

        while inserted < target_fill {
            match table.try_entry(hash, |_| false) {
                Ok(Entry::Vacant(v)) => {
                    v.insert(Item {
                        key: inserted as u64,
                        value: inserted as i32,
                    });
                    inserted += 1;
                }
                Ok(Entry::Occupied(_)) => {}
                Err(TryEntryError::NoFreeSlot) => break,
                Err(TryEntryError::CapacityTooSmall) => break,
            }
        }

        assert_eq!(
            table.try_entry(hash, |_| false).err(),
            Some(TryEntryError::NoFreeSlot)
        );
        assert!(!table.is_empty());
        assert!(table.len() <= capacity);
    }

    #[test]
    fn try_entry_multiple_insertions() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(50);

        let keys = [1u64, 5, 10, 15, 20, 25, 30];

        for &k in &keys {
            let hash = hash_key(&state, k);
            match table.try_entry(hash, |v| v.key == k) {
                Ok(Entry::Vacant(v)) => {
                    v.insert(Item {
                        key: k,
                        value: (k * 10) as i32,
                    });
                }
                Err(e) => panic!("insertion failed for key {}: {:?}", k, e),
                _ => panic!("should be vacant for key {}", k),
            }
        }

        assert_eq!(table.len(), keys.len());

        for &k in &keys {
            let hash = hash_key(&state, k);
            assert_eq!(
                table.find(hash, |v| v.key == k),
                Some(&Item {
                    key: k,
                    value: (k * 10) as i32
                })
            );
        }
    }

    #[test]
    fn try_entry_does_not_resize() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(10);

        let initial_capacity = table.capacity();

        for k in 0..initial_capacity as u64 {
            let hash = hash_key(&state, k);
            let _ = table.try_entry(hash, |v| v.key == k).map(|e| {
                e.or_insert(Item {
                    key: k,
                    value: k as i32,
                })
            });
        }

        assert_eq!(table.capacity(), initial_capacity);

        let k = initial_capacity as u64;
        let hash = hash_key(&state, k);
        assert!(table.try_entry(hash, |v| v.key == k).is_err());

        assert_eq!(table.capacity(), initial_capacity);
    }

    #[test]
    fn try_entry_vacant_or_insert() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(50);

        let k = 100u64;
        let hash = hash_key(&state, k);

        let result = table
            .try_entry(hash, |v| v.key == k)
            .map(|e| e.or_insert(Item { key: k, value: 500 }));

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, 500);
        assert_eq!(table.len(), 1);

        let result = table
            .try_entry(hash, |v| v.key == k)
            .map(|e| e.or_insert(Item { key: k, value: 999 }));

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, 500);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn try_entry_with_collision() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(50);

        for k in 0..20u64 {
            let hash = hash_key(&state, k);
            match table.try_entry(hash, |v| v.key == k) {
                Ok(Entry::Vacant(v)) => {
                    v.insert(Item {
                        key: k,
                        value: k as i32,
                    });
                }
                Err(e) => panic!("should succeed: {:?}", e),
                _ => panic!("should be vacant"),
            }
        }

        for k in 0..20u64 {
            let hash = hash_key(&state, k);
            match table.try_entry(hash, |v| v.key == k) {
                Ok(Entry::Occupied(occ)) => {
                    assert_eq!(occ.get().key, k);
                    assert_eq!(occ.get().value, k as i32);
                }
                _ => panic!("should be occupied for key {}", k),
            }
        }
    }

    #[test]
    fn try_entry_empty_table() {
        let state = HashState::default();
        let mut table: HashTable<Item> = HashTable::with_capacity(10);

        let k = 1u64;
        let hash = hash_key(&state, k);

        assert_eq!(table.len(), 0);

        match table.try_entry(hash, |v| v.key == k) {
            Ok(Entry::Vacant(v)) => {
                v.insert(Item { key: k, value: 42 });
            }
            _ => panic!("should be vacant in empty table"),
        }

        assert_eq!(table.len(), 1);
    }
}
