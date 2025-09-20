use core::hash::BuildHasher;
use core::hash::Hash;

use crate::hash_table::HashTable;

/// A hash set implemented using the hopscotch HashTable as the underlying
/// storage.
///
/// `HashSet<T, S>` stores values of type `T` where `T` implements `Hash + Eq`
/// and uses a configurable hasher builder `S` to hash values. The underlying
/// storage uses the high-performance hopscotch hashing algorithm provided by
/// the `HashTable`.
///
/// # Performance Characteristics
///
/// - **Insert**: O(1) average, O(1) worst case (bounded by neighborhood size)
/// - **Lookup**: O(1) average, O(N) worst case (typically 16 probes)
/// - **Remove**: O(1) average, O(N) worst case
/// - **Memory**: 2 bytes per entry overhead, plus the size of `T` plus a u64
///   for the hash
pub struct HashSet<T, S> {
    table: HashTable<T>,
    hash_builder: S,
}

impl<T, S> HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    /// Creates a new hash set with the given hasher builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let set: HashSet<i32, _> = HashSet::with_hasher(SimpleHasher);
    /// assert!(set.is_empty());
    /// ```
    pub fn with_hasher(hash_builder: S) -> Self {
        Self::with_capacity_and_hasher(0, hash_builder)
    }

    /// Creates a new hash set with the specified capacity and hasher builder.
    ///
    /// The actual capacity may be larger than requested due to the bucket-based
    /// organization of the underlying HashTable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let set: HashSet<i32, _> = HashSet::with_capacity_and_hasher(100, SimpleHasher);
    /// assert!(set.capacity() >= 100);
    /// ```
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            table: HashTable::with_capacity(capacity),
            hash_builder,
        }
    }

    /// Returns the number of elements in the set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use core::hash::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// assert_eq!(set.len(), 0);
    /// set.insert(1);
    /// assert_eq!(set.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// assert!(set.is_empty());
    /// set.insert(1);
    /// assert!(!set.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the current capacity of the set.
    ///
    /// The capacity represents the maximum number of elements the set can
    /// hold before it needs to resize.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use core::hash::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let set: HashSet<i32, _> = HashSet::with_capacity_and_hasher(100, SimpleHasher);
    /// assert!(set.capacity() >= 100);
    /// ```
    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    /// Removes all elements from the set.
    ///
    /// This operation preserves the set's allocated capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use core::hash::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// set.insert(1);
    /// assert!(!set.is_empty());
    /// set.clear();
    /// assert!(set.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.table.clear();
    }

    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        self.table.reserve(additional);
    }

    /// Adds a value to the set.
    ///
    /// Returns whether the value was newly inserted. That is:
    ///
    /// - If the set did not previously contain this value, `true` is returned.
    /// - If the set already contained this value, `false` is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use core::hash::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// assert_eq!(set.insert(37), true);
    /// assert_eq!(set.insert(37), false);
    /// assert_eq!(set.len(), 1);
    /// ```
    pub fn insert(&mut self, value: T) -> bool {
        let hash = self.hash_builder.hash_one(&value);
        match self.table.entry(hash, |v| v == &value) {
            crate::hash_table::Entry::Occupied(_) => false,
            crate::hash_table::Entry::Vacant(entry) => {
                entry.insert(value);
                true
            }
        }
    }

    /// Returns `true` if the set contains a value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// set.insert(1);
    /// assert!(set.contains(&1));
    /// assert!(!set.contains(&2));
    /// ```
    pub fn contains(&self, value: &T) -> bool {
        let hash = self.hash_builder.hash_one(value);
        self.table.find(hash, |v| v == value).is_some()
    }

    /// Removes a value from the set. Returns whether the value was
    /// present in the set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// set.insert(1);
    /// assert_eq!(set.remove(&1), true);
    /// assert_eq!(set.remove(&1), false);
    /// ```
    pub fn remove(&mut self, value: &T) -> bool {
        let hash = self.hash_builder.hash_one(value);
        self.table.remove(hash, |v| v == value).is_some()
    }

    /// Removes and returns the value in the set, if any, that is equal to the
    /// given one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// set.insert(1);
    /// assert_eq!(set.take(&1), Some(1));
    /// assert_eq!(set.take(&1), None);
    /// ```
    pub fn take(&mut self, value: &T) -> Option<T> {
        let hash = self.hash_builder.hash_one(value);
        self.table.remove(hash, |v| v == value)
    }

    /// Returns a reference to the value in the set, if any, that is equal to
    /// the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// set.insert(1);
    /// assert_eq!(set.get(&1), Some(&1));
    /// assert_eq!(set.get(&2), None);
    /// ```
    pub fn get(&self, value: &T) -> Option<&T> {
        let hash = self.hash_builder.hash_one(value);
        self.table.find(hash, |v| v == value)
    }

    /// Returns an iterator over the values of the set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// set.insert(1);
    /// set.insert(2);
    ///
    /// for value in set.iter() {
    ///     println!("Value: {}", value);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            inner: self.table.iter(),
        }
    }

    /// Returns an iterator that removes and yields all values from the
    /// set.
    ///
    /// After calling `drain()`, the set will be empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let mut set = HashSet::with_hasher(SimpleHasher);
    /// set.insert(1);
    /// set.insert(2);
    ///
    /// let values: Vec<_> = set.drain().collect();
    /// assert!(set.is_empty());
    /// assert_eq!(values.len(), 2);
    /// ```
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain {
            inner: self.table.drain(),
        }
    }
}

impl<T, S> HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher + Default,
{
    /// Creates a new hash set using the default hasher builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # #[derive(Default)]
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let set: HashSet<i32, SimpleHasher> = HashSet::new();
    /// assert!(set.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::with_hasher(S::default())
    }

    /// Creates a new hash set with the specified capacity using the default
    /// hasher builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashSet;
    /// #
    /// # #[derive(Default)]
    /// # struct SimpleHasher;
    /// # impl BuildHasher for SimpleHasher {
    /// #     type Hasher = SipHasher;
    /// #
    /// #     fn build_hasher(&self) -> Self::Hasher {
    /// #         SipHasher::new()
    /// #     }
    /// # }
    ///
    /// let set: HashSet<i32, SimpleHasher> = HashSet::with_capacity(100);
    /// assert!(set.capacity() >= 100);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, S::default())
    }
}

impl<T, S> Default for HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

/// An iterator over the values of a `HashSet`.
pub struct Iter<'a, T> {
    inner: crate::hash_table::Iter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// A draining iterator over the values of a `HashSet`.
pub struct Drain<'a, T> {
    inner: crate::hash_table::Drain<'a, T>,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, T> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        for _ in self {}
    }
}

impl<'a, T, S> IntoIterator for &'a HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
