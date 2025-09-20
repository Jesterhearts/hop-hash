#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// A HashMap implementation using hopscotch hashing.
///
/// This module provides a `HashMap` that wraps the `HashTable` and provides
/// a standard key-value map interface with configurable hashers.
pub mod hash_map;

/// A low-level hash table implementation using hopscotch hashing.
///
/// This module provides the core `HashTable<V>` data structure that implements
/// a hash table using the hopscotch hashing algorithm. Unlike traditional hash
/// tables, hopscotch hashing ensures that all items belonging to the same
/// bucket are stored within a small, fixed-size neighborhood.
///
/// ## Key Components
///
/// - [`HashTable<V>`]: The main hash table structure that stores values of type
///   `V`
/// - [`Entry`]: An enum for vacant/occupied entry manipulation
/// - [`VacantEntry`] and [`OccupiedEntry`]: Entry types for insertion and
///   modification
/// - [`Iter`] and [`Drain`]: Iterator types for traversing and consuming the
///   table
///
/// ## Algorithm Details
///
/// The hopscotch algorithm maintains a "hop info" bitmap for each bucket that
/// tracks which slots in the neighborhood contain items belonging to that
/// bucket. This allows for:
///
/// - **Constant-time worst-case insertion**: Bounded by the neighborhood size
///   (16)
/// - **Excellent cache locality**: Items are kept close to their ideal
///   positions
/// - **SIMD optimization**: Uses SSE2 instructions when available for fast
///   scanning
///
/// ## Performance Characteristics
///
/// - **Memory overhead**: 2 bytes per slot plus hash storage
///
/// ## When to Use
///
/// Use `HashTable` directly when:
/// - You need maximum control over hashing
/// - You want to provide custom equality predicates for each operation
/// - You're implementing your own map-like data structure
///
/// For most use cases, prefer [`HashMap`] which provides a more convenient
/// key-value interface built on top of this hash table.
///
/// ## Safety
///
/// The hash table implementation uses unsafe code internally for performance,
/// but provides a safe public API. All unsafe operations are carefully
/// documented and bounded by runtime assertions in debug builds.
///
/// [`HashTable<V>`]: hash_table::HashTable
/// [`Entry`]: hash_table::Entry
/// [`VacantEntry`]: hash_table::VacantEntry
/// [`OccupiedEntry`]: hash_table::OccupiedEntry
/// [`Iter`]: hash_table::Iter
/// [`Drain`]: hash_table::Drain
/// [`HashMap`]: HashMap
pub mod hash_table;

/// A hash set implementation using hopscotch hashing.
///
/// This module provides a `HashSet` that wraps the `HashTable` and provides
/// a standard set interface with configurable hashers.
pub mod hash_set;

pub use hash_map::Entry;
pub use hash_map::HashMap;
pub use hash_set::HashSet;
pub use hash_table::HashTable;
