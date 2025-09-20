# hop-hash

A high-performance hash map implementation in Rust, utilizing a 16-way hopscotch hashing scheme with
16-entry buckets, inspired by Google's Swiss Table design.

## Architecture

`hop-hash` is designed for high performance and low latency. It achieves this through a combination
of modern hash table design principles:

*   **16-Way Hopscotch Hashing:** Each entry is guaranteed to be within a "neighborhood" of 16
    entries from its ideal location. This keeps probe distances short and predictable, leading to
    fast lookups, insertions, and removals.
*   **16-Entry Buckets:** The hash table is organized into 16-entry buckets. This structure is
    highly amenable to SIMD optimizations, allowing for parallel probing of multiple slots at once.
*   **SIMD-Accelerated Lookups:** On `x86_64` architectures with SSE2 support, `hop-hash` uses SIMD
    instructions to scan for matching entries within a bucket in parallel, significantly speeding up
    lookups.
*   **Overflow Handling:** In the rare case that an entry cannot be placed within its neighborhood,
    it is stored in an overflow vector. This ensures that the table can handle high load factors
    and hash collisions without failing.


## License

This project is licensed under the terms of the MIT license or the Apache License (Version 2.0), at your
option.
