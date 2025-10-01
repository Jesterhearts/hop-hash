#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

use cfg_if::cfg_if;

extern crate alloc;

cfg_if! {
    if #[cfg(all(feature = "std", feature = "foldhash"))] {
        use foldhash::fast::RandomState;
    } else if #[cfg(all(feature = "std", not(feature = "foldhash")))] {
        use std::collections::hash_map::RandomState;
    }
}

/// A HashMap implementation using hopscotch hashing.
///
/// This module provides a `HashMap` that wraps the `HashTable` and provides
/// a standard key-value map interface with configurable hashers.
pub mod hash_map;

pub mod hash_table;

/// A HashSet implementation using hopscotch hashing.
///
/// This module provides a `HashSet` that wraps the `HashTable` and provides
/// a standard set interface with configurable hashers.
pub mod hash_set;

cfg_if! {
    if #[cfg(any(feature = "std", feature = "foldhash"))] {
        /// The default `HashMap` type using `RandomState` as the hasher.
        pub type HashMap<K, V, S = RandomState> = hash_map::HashMap<K, V, S>;
    }else {
        /// The default `HashMap` type. You must provide a hasher.
        pub type HashMap<K, V, S> = hash_map::HashMap<K, V, S>;
    }
}

cfg_if! {
    if #[cfg(any(feature = "std", feature = "foldhash"))] {
        /// The default `HashSet` type using `RandomState` as the hasher.
        pub type HashSet<K, S = RandomState> = hash_set::HashSet<K, S>;
    }else {
        /// The default `HashSet` type. You must provide a hasher.
        pub type HashSet<K, S> = hash_set::HashSet<K, S>;
    }
}

pub use hash_map::Entry;
pub use hash_table::HashTable;
