# hop-hash

A high-performance hash table implementation in Rust, utilizing an 8 or 16-way hopscotch hashing scheme.

## Features

- **High Performance**: Optimized for fast lookups, insertions, and removals with a target load
  factor of 92%.
- **Bounded Probing**: Hopscotch hashing guarantees entries are within a small neighborhood of their
  ideal location, ensuring short and predictable probe distances.
- **SIMD Acceleration**: Leverages SSE2 instructions for parallel scanning of 16-entry buckets,
  significantly accelerating lookups.
- **Efficient Memory Layout**: A compact memory structure with a low overhead of 2 bytes per entry
  for metadata.
- **Few Dependencies**: Pure Rust implementation with one external dependency (`cfg-if`).

## Design

`hop-hash` combines several design principles for high performance:

- **16-Way Hopscotch Hashing**: Each entry is stored within a "neighborhood" of 16 slots from its
  initial bucket. This ensures that probe sequences are short and bounded.
- **16-Entry Buckets**: The table is organized into 16-entry buckets, a structure amenable to SIMD
  optimizations. Entries are associated with a 7-bit tag (derived from the hash, inspired by
  SwissTable/Hashbrown) for fast filtering during lookups, minimizing expensive key comparisons.
- **SIMD-Accelerated Lookups**: On architectures with SSE2 support, the implementation uses SIMD
  instructions to compare 16 tags in parallel. This allows for the rapid elimination of non-matching
  entries. A scalar fallback is provided for non-SIMD platforms.
- **Overflow Handling**: In the rare event that an entry cannot be placed within its 16-slot
  neighborhood (e.g., due to extreme hash collisions), it is stored in a separate overflow vector.
  This maintains correctness in the face of very bad hash functions (unless you have a degenerate
  hash function, the odds of this overflow being used are effectively zero however).
- **Resize Strategy**: The table automatically doubles its capacity when the load factor exceeds
  92%. All entries are rehashed and reinserted into the new table. Insertion order is not preserved.

## Choosing a Neighborhood Size
The default choice of a 16-entry neighborhood balances performance and memory usage effectively. A
smaller neighborhood (e.g., 8 entries) would reduce the fixed memory overhead of 256 buckets (down
to 128) and put a tighter bound on maximum probe length, but it has a slightly increased risk of the
table over-allocating space due to failed attempts to bubble an empty slot into the neighborhood.
The exact risk of over-allocation depends on your target load factor and the quality of your hash
function. It is rare to see over-allocation with a good hash function and a target load factor of
92%, but it is much more likely with a target load factor of 97% or with a poor hash function.

In benchmarks, the choice of neighborhood size (8 vs 16) has a negligible impact on performance for
most workloads.

Ultimately, defaulting to 16-entry neighborhoods is the best balance, and you should only use
8-entry neighborhoods if you want to minimize your worst-case probe length and are okay with a
slightly increased risk of over-allocation.

## Choosing a Target Load Factor
The default target load factor of 92% is chosen to balance memory efficiency and performance. At
this load factor, the table remains efficient in terms of memory usage while still providing fast
operations. Higher load factors can lead to increased probe lengths and more frequent collisions,
which can degrade performance. However, the hopscotch hashing scheme mitigates these issues by
ensuring that entries remain within a bounded neighborhood. In benchmarks, a target load factor of
92% consistently provides excellent performance across a variety of workloads. If you prioritize
memory efficiency and are willing to accept a slight performance trade-off, you might consider a
using a target load factor of 97%. This trades about 3-5% performance for about 5% decreased memory
usage in benchmarks on my machine. Conversely, if you want to prioritize performance and are willing
to use more memory, a lower target load factor of 87.5% can be used. This increases performance
by 3-5% in benchmarks on my machine, at the cost of about 5% increased memory usage.

## Probe Length Debugging
The `HashTable` struct includes a `probe_histogram` method that returns a histogram of probe lengths for
all entries in the table. This can be useful for debugging and performance tuning, as it provides
insight into how well the hash function is distributing entries and how effectively the hopscotch
hashing scheme is working.

## Limitations

- **Hash Function Dependency**: Performance is highly dependent on the quality of the hash function.
  A poor hash function can lead to increased collisions and degrade performance. That being said,
  this table design is more resilient to poor hash functions than many other designs. Provided that
  your hash function produces a reasonably uniform distribution, `hop-hash` should only see degraded
  performance from a bad hash function rather than pathological performance.
- **Memory Usage**: The table's capacity grows in powers of two and is not optimized for very small
  data sets due to a minimum reservation size. The overhead of 2 bytes per entry for metadata may
  also be significant for very small tables. However, when combined with the target load factor of
  92%, the effective overhead may be lower than other implementations for medium and large tables.
- **Key Constraints**: The `Eq` and `Hash` implementations for keys must be consistent.

## A Note on Benchmarks
Benchmarks comparing `hop-hash` to `hashbrown` are available in the `benches` directory. These
benchmarks demonstrate the performance characteristics of `hop-hash` under various workloads and
configurations.

The benchmarks use randomized data, which I feel better represents real-world usage than sequential
data. However, results may vary based on the specific characteristics of the data and the
environment. With this randomized data, the two crates benchmark very closely, with `hop-hash`
outperforming `hashbrown` in some scenarios and vice versa. However, if you have a use case of
sequential data, where you are reading the same set of keys in the same order multiple times,
`hashbrown` will far outperform `hop-hash`. The same is true if you have small or medium-small
tables (<16-32k elements), and you can pre-allocate the table to the correct size. The performance
difference is less stark if you cannot pre-allocate the table, but `hashbrown` will still outperform
`hop-hash` in these scenarios for some workloads.

## License

This project is dual-licensed under the MIT license and the Apache License (Version 2.0), at your option.