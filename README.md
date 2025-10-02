# hop-hash

A high-performance hash table implementation in Rust, utilizing an 8-way hopscotch hashing scheme.

Hopscotch hashing is a technique which attempts to place an item within a fixed distance (a
"neighborhood") of its ideal bucket during insertion. If this fails, an empty spot is located and
bubbled backwards until it is within the neighborhood. This provides the nice effect that lookups
and removals have constant-time worst-case behavior, and insertion has amortized constant-time
behavior.

This crate provides `HashMap` and `HashSet` implementations built on top of a lower-level
`HashTable` structure.

## When to Use `hop-hash`

`hop-hash` is designed for scenarios where you need predictable performance characteristics with
mixed workloads. Consider using `hop-hash` when:

- **You have mixed operations.** The hopscotch algorithm works best on workloads that combine
  insertions, lookups, and deletions. For read-only workloads, `hashbrown` will perform much better.

- **You need predictable lookup latency.** Lookups are bounded to probing at most 8 buckets (16 with
  the `sixteen-way` feature), compared to unbounded probe sequences in some hash table designs. This
  provides more consistent performance characteristics, though `hashbrown` offers superior lookup
  performance in practice.

- **Memory efficiency matters for your entry size.** For tables with entries of approximately 20
  bytes or larger, a higher load factor (configurable 92% or 97% vs `hashbrown`'s 87.5%) can
  offset the additional per-entry metadata overhead (2 bytes vs 1 byte), resulting in comparable or
  better memory density. Note that this penalizes performance for small tables heavily and there is
  a minimum table capacity of 144 entries (272 with `sixteen-way`), so this advantage only applies to
  sufficiently large tables.

### Important Limitations

- **Very small tables:** The minimum capacity requirement and additional overhead means `hop-hash` is not
  suitable for very small hash tables where memory efficiency is critical.

- **Read-heavy workloads:** For workloads dominated by lookups with few modifications, `hashbrown`'s
  optimizations provide better performance.

- **Pathological hash functions:** While `hop-hash` is more resilient to poor hash functions than
  many designs, bad hash functions can still degrade performance. In the case of adversarial inputs,
  it is possible to force the table into a resize loop that results in an OOM crash. A good hash
  function will protect against this, just like it will protect any hash table from DOS attacks.

## Features

- **Worst-Case Constant-Time Lookups**: Hopscotch hashing guarantees entries are within a small,
  fixed-size neighborhood of their ideal location, ensuring short and predictable probe distances.
- **Few Dependencies**: Pure Rust implementation with two dependencies - `cfg-if`, and `foldhash`
  (optional).

## Basic Usage
```rust
use hop_hash::HashMap;

let mut map = HashMap::new();
map.insert("key1", "value1");
map.insert("key2", "value2");

assert_eq!(map.get(&"key1"), Some(&"value1"));
map.remove(&"key2");
assert_eq!(map.get(&"key2"), None);
```

## Choosing a Neighborhood Size
The default neighborhood size is 8 (`eight-way`), which provides the best overall performance across
all table sizes and workloads. **Important Caveat:** If you choose to use the `density-ninety-seven`
feature, you should also use the `sixteen-way` feature, as 8-way with 97% load factor has a high
risk of over-allocation for large tables.

## Choosing a Target Load Factor

The choice of load factor significantly impacts the performance/memory tradeoff:

- **87.5% (`density-eighty-seven-point-five`, default)**: The highest performance option. This has
  higher per-entry overhead than `hashbrown` (2 bytes vs 1 byte).

- **92% (`density-ninety-two`)**: Provides a balance between performance and memory efficiency for
  larger tables. Note that for small tables, this can harm performance by as much as 50% in
  benchmarks, with the exception of iteration, where the performance **improves** across all table
  sizes for higher densities. The impact of this setting is much more nuanced at larger table sizes,
  and may either help or harm performance by a few percent. If you have large tables (>32k entries)
  I'd recommend benchmarking both 87.5% and 92% density with your workload to see which performs
  better.

- **97% (`density-ninety-seven`)**: Maximizes memory efficiency at the cost of approximately 3-5%
  performance over `density-ninety-two`. Avoid combining with `eight-way` due to significantly
  increased over-allocation risk.

## Probe Length Debugging
The `HashTable` struct includes a `probe_histogram` method (feature `stats`) that returns a
histogram of probe lengths for all entries in the table. This can be useful for debugging and
performance tuning, as it provides insight into how well the hash function is distributing entries.

## Design

`hop-hash` combines several design principles for high performance.

### Hopscotch Hashing Principle
Each entry is stored within a "neighborhood" of 8 slots from its initial "root" bucket, which is
determined by its hash. This ensures that probe sequences for lookups are short and bounded. If an
item cannot be placed in its neighborhood during insertion, the table finds a nearby empty slot and
"bubbles" it back by swapping it with other items until the empty slot is inside the required
neighborhood.

### Memory Layout
All data is stored in a single, contiguous, type-erased allocation with the structure:
`[ HopInfo | Tags | Values ]`. This layout was found to have better iteration performance than an
array-of-structs approach.

### Neighborhood and Occupancy (`HopInfo`)
For each 16-entry root bucket, a corresponding `HopInfo` struct tracks the occupancy of the 8
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
`HOP_RANGE` (8) buckets is added to the end of the table to allow the final neighborhood to span a
full 8 buckets without needing complex and expensive wrapping logic.

## Limitations

- **Hash Function Dependency**: Performance is highly dependent on the quality of the hash function.
  A poor hash function can lead to increased collisions and degrade performance.
- **Memory Usage**: The table's capacity grows in powers of two and is not optimized for very small
  data sets due to a minimum reservation size (a minimum of 272 entries for 16-way, 144 for 8-way).
- **Key Constraints**: The `Eq` and `Hash` implementations for keys must be consistent.

## A Note on Benchmarks

[Benchmark Results](benches/README.md)

Benchmarks comparing `hop-hash` to `hashbrown` are available in the `benches` directory. These
benchmarks demonstrate the performance characteristics of `hop-hash` under various workloads and
configurations.

The benchmarks use randomized data, which I feel better represents real-world usage than sequential
data. With this randomized data, the two crates benchmark very closely, with `hop-hash`
outperforming `hashbrown` in some scenarios and vice versa. However, if you are doing only lookups
on a pre-populated table, `hashbrown` will outperform `hop-hash`, especially if you can pre-allocate
the table to the correct size. The same is true if you have small or medium-small tables (<16k
elements).

## License

This project is dual-licensed under the MIT license and the Apache License (Version 2.0), at your
option.
