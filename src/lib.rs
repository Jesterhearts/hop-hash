#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// A HashMap implementation using hopscotch hashing.
///
/// This module provides a `HashMap` that wraps the `HashTable` and provides
/// a standard key-value map interface with configurable hashers.
pub mod hash_map;

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
