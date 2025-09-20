# hop-hash

A high-performance hash table implementation in Rust, utilizing a 16-way hopscotch hashing scheme with
16-entry buckets, inspired by Google's Swiss Table design.

## Features

- **High Performance**: Optimized for fast lookups, insertions, and removals
- **SIMD Acceleration**: Uses SSE2 instructions on x86_64 for parallel bucket scanning
- **Predictable Performance**: Short, bounded probe distances with hopscotch hashing
- **Zero Dependencies**: Pure Rust implementation with no external dependencies

### Basic Usage

```rust
use std::hash::{Hash, Hasher, DefaultHasher};
use hop_hash::HashTable;

#[derive(Debug, PartialEq)]
struct Item {
    key: u64,
    value: String,
}

fn hash_key(key: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

fn main() {
    let mut table = HashTable::with_capacity(16);

    // Insert an item
    let hash = hash_key(42);
    match table.entry(hash, |item: &Item| item.key == 42) {
        hop_hash::Entry::Vacant(entry) => {
            entry.insert(Item {
                key: 42,
                value: "Hello, World!".to_string(),
            });
        }
        hop_hash::Entry::Occupied(mut entry) => {
            // Key already exists, update value
            entry.get_mut().value = "Updated!".to_string();
        }
    }

    // Find an item
    if let Some(item) = table.find(hash, |item| item.key == 42) {
        println!("Found: {:?}", item);
    }

    // Remove an item
    if let Some(removed) = table.remove(hash, |item| item.key == 42) {
        println!("Removed: {:?}", removed);
    }
}
```

## Architecture

`hop-hash` is designed for high performance and low latency. It achieves this through a combination
of modern hash table design principles:

### 16-Way Hopscotch Hashing
Each entry is guaranteed to be within a "neighborhood" of 16 entries from its ideal location (or
rarely in an "overflow" area if there are enough hash collisions to fill an entire 16x16-entry
bucket even with resizing). This keeps probe distances short and predictable, leading to fast
lookups, insertions, and removals.

### 16-Entry Buckets
The hash table is organized into 16-entry buckets. This structure is highly amenable to SIMD
optimizations, allowing for parallel probing of multiple slots at once:

- Each bucket is 16-byte aligned for optimal SIMD access
- Tag bytes enable fast filtering before expensive equality checks
- Bucket-based organization improves cache locality

### SIMD-Accelerated Lookups
On `x86_64` architectures with SSE2 support, `hop-hash` uses SIMD instructions to scan for matching
entries within a bucket in parallel, significantly speeding up lookups:

- **Parallel Tag Scanning**: Compare 16 tag bytes simultaneously
- **Efficient Filtering**: Quickly eliminate non-matching entries
- **Fallback Support**: Graceful degradation on non-SIMD platforms

### Overflow Handling
In the rare case that an entry cannot be placed within its neighborhood it is stored in an overflow
vector. This ensures that the table can handle high load factors and hash collisions without
failing:

- **Graceful Degradation**: Maintains correctness under extreme conditions
- **Rare Occurrence**: Overflow is uncommon with good hash functions
- **Transparent API**: Overflow entries are accessible through the same interface

## Performance Characteristics

### Time Complexity

| Operation | Average Case | Worst Case |
| --------- | ------------ | ---------- |
| Insert    | O(1)         | O(1)       |
| Lookup    | O(1)         | O(N)       |
| Remove    | O(1)         | O(N)       |


### Space Complexity

- **Memory Overhead**: 2 bytes per entry for the tags + hop info. This also implementation also stores full hash values, adding 8 bytes per entry.
- **Load Factor**: Maintains ~93.75% occupancy (15/16)

## no_std Support

`hop-hash` can be used in `no_std` environments:

```toml
[dependencies]
hop-hash = { version = "0.1", default-features = false }
```

In `no_std` mode:
- Uses `alloc` for memory allocation
- All core functionality remains available
- Compatible with embedded systems and kernel code

## Implementation Details

### Memory Layout

The hash table uses a carefully designed memory layout for optimal performance:

```txt
[HopInfo Array][Tag Bytes][Value Buckets][Hash Storage]
```

- **HopInfo Array**: Tracks which neighborhood slots are occupied
- **Tag Bytes**: High bits of hash for fast filtering
- **Value Buckets**: Actual stored values
- **Hash Storage**: Full hash values for verification

### Resize Strategy

The table automatically resizes when the load factor exceeds 93.75%:

1. **Double Capacity**: New table has 2x the bucket count
2. **Rehash All Entries**: All entries are rehashed and reinserted
3. **Does Not Preserve Order**: Insertion order is not preserved (by design)
4. **Atomic Transition**: Old table is dropped only after successful migration

### SIMD Optimizations

On x86_64 with SSE2 support:

- **Tag Scanning**: 16 tags compared in a single instruction
- **Occupancy Checking**: Parallel detection of empty slots
- **Branch Reduction**: Fewer conditional jumps in hot paths

## Limitations

### Hash Quality Dependency
Performance is highly dependent on hash function quality. Poor hash functions can lead to:
- Increased collisions
- More overflow entries
- Degraded performance

### Memory Usage
- Not suitable for very small tables (< 16x16 entries)
- Memory usage grows in powers of 2

### Key Constraints
- Equality function must be consistent with hash

## License

This project is licensed under the terms of the MIT license or the Apache License (Version 2.0), at your
option.
