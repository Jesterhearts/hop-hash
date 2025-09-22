# hop-hash

A high-performance hash table implementation in Rust, utilizing a 16-way hopscotch hashing scheme.

## Features

- **High Performance**: Optimized for fast lookups, insertions, and removals with a target load
  factor of 92%.
- **Bounded Probing**: Hopscotch hashing guarantees entries are within a small neighborhood of their
  ideal location, ensuring short and predictable probe distances.
- **SIMD Acceleration**: Leverages SSE2 instructions on `x86_64` for parallel scanning of 16-entry
  buckets, significantly accelerating lookups.
- **Efficient Memory Layout**: A compact memory structure with a low overhead of 2 bytes per entry
  for metadata.
- **Zero Dependencies**: Pure Rust implementation with no external dependencies.

## Design

`hop-hash` combines several design principles for high performance:

- **16-Way Hopscotch Hashing**: Each entry is stored within a "neighborhood" of 16 slots from its
  initial bucket. This ensures that probe sequences are short and bounded.
- **16-Entry Buckets**: The table is organized into 16-entry buckets, a structure amenable to SIMD
  optimizations. Each bucket is 16-byte aligned, and entries are associated with an 8-bit tag
  (derived from the hash) for fast filtering during lookups, minimizing expensive key comparisons.
- **SIMD-Accelerated Lookups**: On `x86_64` architectures with SSE2 support, the implementation uses
  SIMD instructions to compare 16 tags in parallel. This allows for the rapid elimination of
  non-matching entries. A scalar fallback is provided for non-SIMD platforms.
- **Overflow Handling**: In the rare event that an entry cannot be placed within its 16-slot
  neighborhood (e.g., due to extreme hash collisions), it is stored in a separate overflow vector.
  This maintains correctness under high load factors.
- **Resize Strategy**: The table automatically doubles its capacity when the load factor exceeds
  92%. All entries are rehashed and reinserted into the new table. Insertion order is not preserved.

## Limitations

- **Hash Function Dependency**: Performance is highly dependent on the quality of the hash function.
  A poor hash function can lead to increased collisions and degrade performance.
- **Memory Usage**: The table's capacity grows in powers of two and is not optimized for very small
  data sets due to a minimum reservation size.
- **Key Constraints**: The `Eq` and `Hash` implementations for keys must be consistent.

## License

This project is dual-licensed under the MIT license and the Apache License (Version 2.0), at your option.