# hop-hash

A high-performance hash table implementation in Rust, utilizing a 16-way hopscotch hashing scheme.

Hopscotch hashing is a technique which attempts to place an item within a fixed distance (a
"neighborhood") of its ideal bucket during insertion. If this fails, an empty spot is located and
bubbled backwards until it is within the neighborhood. This provides the nice effect that lookups
and removals have constant-time worst-case behavior, and insertion has amortized constant-time
behavior.

This crate provides `HashMap` and `HashSet` implementations built on top of a lower-level
`HashTable` structure.

## Features

- **High Performance**: Optimized for fast lookups, insertions, and removals with a default target
  load factor of 92%.
- **Constant-Time Lookups**: Hopscotch hashing guarantees entries are within a small, fixed-size
  neighborhood of their ideal location, ensuring short and predictable probe distances.
- **SIMD Acceleration**: Leverages SSE2 instructions for parallel scanning of 16-entry buckets,
  significantly accelerating lookups.
- **Efficient Memory Layout**: A compact, contiguous memory structure with a low overhead of 2 bytes
  per entry for metadata.
- **Robust Overflow Handling**: Includes an overflow mechanism to gracefully handle pathological
  hash inputs without uncontrolled memory growth, at the cost of degraded performance in such
  scenarios.
- **Few Dependencies**: Pure Rust implementation with one external dependency (`cfg-if`).

## Design

`hop-hash` combines several design principles for high performance.

### Hopscotch Hashing Principle
Each entry is stored within a "neighborhood" of 16 slots from its initial "root" bucket, which is
determined by its hash. This ensures that probe sequences for lookups are short and bounded. If an
item cannot be placed in its neighborhood during insertion, the table finds a nearby empty slot and
"bubbles" it back by swapping it with other items until the empty slot is inside the required
neighborhood.

### Memory Layout
All data is stored in a single, contiguous, type-erased allocation with the structure:
`[ HopInfo | Tags | Values ]`. This layout was found to have better iteration performance than an
array-of-structs approach.

### Neighborhood and Occupancy (`HopInfo`)
For each 16-entry root bucket, a corresponding `HopInfo` struct tracks the occupancy of the 16
neighbor buckets. This allows for fast scans to see which neighbors need to be probed during
lookups.

### Tags and SIMD
Inspired by SwissTable/Hashbrown, each entry is associated with a 7-bit tag derived from the top 7
bits of its hash. The most significant bit is reserved to mark empty slots (`0x80`). This allows the
implementation to use a single SIMD load/mask operation to find empty slots and a simple
load/cmp/mask identify potential matches across 16 entries in parallel, significantly speeding up
insertions and lookups.

### Sizing and Padding
Table sizes are always a power of two, allowing for fast bitwise masking (`hash & mask`) to
determine an item's root bucket instead of a slower modulo operation. An additional pad of
`HOP_RANGE` (16) buckets is added to the end of the table to allow the final neighborhood to span a
full 16 buckets without needing complex and expensive wrapping logic.

### Overflow Handling
In the rare event that an entry cannot be placed within its neighborhood (e.g., due to extreme hash
collisions), it is stored in a separate overflow vector. This is a safety measure to avoid an
infinite resize loop and out-of-memory errors in the face of pathological hash inputs. The odds of
this overflow being used with a decent hash function are effectively zero.

## Implementation Notes & Quirks
Some non-obvious micro-optimizations are used to improve performance:

- **`ptr::write_bytes` for Initialization**: The `HopInfo` arrays are initialized using
  `ptr::write(0)` rather than `alloc_zeroed`. On my machine, this showed a benchmark improvement of
  up to 30%, and was kept for that reason.
- **Load Factor Choice**: The table doesn't support a load factor of 87.5% (7/8). While easy to
  implement, it showed no significant benchmark impact, so the slightly higher memory usage was not
  deemed worth it.
- **Bucket 0 Optimization**: During lookups, bucket 0 is _always_ checked unconditionally. Testing
  revealed that this bucket is almost always occupied, and checking it first skips the overhead of
  looking up the neighborhood info, improving performance for the common case where an item is in
  its ideal bucket.

## Choosing a Neighborhood Size
The default choice of a 16-entry neighborhood balances performance and memory usage effectively. A
smaller neighborhood (e.g., 8 entries, via the `eight-way` feature) would reduce the fixed memory
overhead of the padding buckets and put a tighter bound on maximum probe length, but it has a
slightly increased risk of the table over-allocating space due to failed attempts to bubble an empty
slot into the neighborhood.

In benchmarks, the choice of neighborhood size (8 vs 16) has a negligible impact on performance for
larger tables, but can greatly improve performance for small tables. You should use 8-entry
neighborhoods if you want to minimize your worst-case probe length and are okay with a slightly
increased risk of over-allocation or are using smaller tables.

## Choosing a Target Load Factor
The default target load factor of 92% (`density-ninety-two` feature) is chosen to balance memory
efficiency and performance. If you prioritize memory efficiency and are willing to accept a slight
performance trade-off, you might consider using a target load factor of 97% (`density-ninety-seven`
feature). This trades about 3-5% performance for about 5% decreased memory usage in benchmarks. Note
that when combined with the `eight-way` feature, you significantly increase the risk of
over-allocation, so be careful combining those two features if you are trying to conserve memory.

## Probe Length Debugging
The `HashTable` struct includes a `probe_histogram` method (feature `stats`) that returns a
histogram of probe lengths for all entries in the table. This can be useful for debugging and
performance tuning, as it provides insight into how well the hash function is distributing entries.

## Limitations

- **Hash Function Dependency**: Performance is highly dependent on the quality of the hash function.
  A poor hash function can lead to increased collisions and degrade performance. That being said,
  this table design is more resilient to poor hash functions than many other designs.
- **Memory Usage**: The table's capacity grows in powers of two and is not optimized for very small
  data sets due to a minimum reservation size.
- **Key Constraints**: The `Eq` and `Hash` implementations for keys must be consistent.

## A Note on Benchmarks

[Benchmark Results](benches/README.md)

Benchmarks comparing `hop-hash` to `hashbrown` are available in the `benches` directory. These
benchmarks demonstrate the performance characteristics of `hop-hash` under various workloads and
configurations.

The benchmarks use randomized data, which I feel better represents real-world usage than sequential
data. With this randomized data, the two crates benchmark very closely, with `hop-hash`
outperforming `hashbrown` in some scenarios and vice versa. However, if you have a use case of
sequential data, where you are reading the same set of keys in the same order multiple times,
`hashbrown` will far outperform `hop-hash`. The same is true if you have small or medium-small
tables (<16-32k elements), and you can pre-allocate the table to the correct size.

## License

This project is dual-licensed under the MIT license and the Apache License (Version 2.0), at your
option.
