use core::fmt::Debug;
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
/// - **Memory**: 2 bytes per entry overhead, plus the size of `T`.
#[derive(Clone)]
pub struct HashSet<T, S> {
    table: HashTable<T>,
    hash_builder: S,
}

impl<T, S> Debug for HashSet<T, S>
where
    T: Debug + Hash + Eq,
    S: BuildHasher,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
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

    /// Shrinks the capacity of the set as much as possible.
    ///
    /// This method will shrink the underlying storage to fit the current number
    /// of elements, potentially freeing unused memory. The resulting capacity
    /// will be at least as large as the number of elements in the set, but may
    /// be larger due to the bucket-based organization of the underlying
    /// HashTable.
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
    /// #
    /// let mut set = HashSet::with_capacity_and_hasher(1000, SimpleHasher);
    /// set.insert(1);
    /// set.insert(2);
    ///
    /// // The set has a large capacity but only 2 elements
    /// assert!(set.capacity() >= 1000);
    /// assert_eq!(set.len(), 2);
    ///
    /// set.shrink_to_fit();
    ///
    /// // The capacity is now much smaller, but still fits the elements
    /// assert!(set.capacity() >= 2);
    /// assert!(set.capacity() < 1000);
    /// assert_eq!(set.len(), 2);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        self.table.shrink_to_fit(|k| self.hash_builder.hash_one(k));
    }

    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        self.table
            .reserve(additional, |k| self.hash_builder.hash_one(k));
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
        match self
            .table
            .entry(hash, |v| v == &value, |v| self.hash_builder.hash_one(v))
        {
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
    /// Calling `mem::forget` on the returned iterator will leak all values in
    /// the set that have not yet been yielded. This can cause memory leaks.
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

impl<T> Iterator for Drain<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<T> Drop for Drain<'_, T> {
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

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::hash::BuildHasher;

    use rand::TryRngCore;
    use rand::rngs::OsRng;
    use siphasher::sip::SipHasher;

    use super::*;

    #[derive(Clone)]
    struct SipHashBuilder {
        k1: u64,
        k2: u64,
    }

    impl BuildHasher for SipHashBuilder {
        type Hasher = SipHasher;

        fn build_hasher(&self) -> Self::Hasher {
            SipHasher::new_with_keys(self.k1, self.k2)
        }
    }

    impl Default for SipHashBuilder {
        fn default() -> Self {
            Self {
                k1: OsRng.try_next_u64().unwrap_or(0),
                k2: OsRng.try_next_u64().unwrap_or(0),
            }
        }
    }

    #[test]
    fn test_new_and_with_hasher() {
        let set: HashSet<i32, SipHashBuilder> = HashSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);

        let set2 = HashSet::<i32, _>::with_hasher(SipHashBuilder::default());
        assert!(set2.is_empty());
        assert_eq!(set2.len(), 0);
    }

    #[test]
    fn test_with_capacity() {
        let set: HashSet<i32, SipHashBuilder> = HashSet::with_capacity(100);
        assert!(set.capacity() >= 100);
        assert!(set.is_empty());

        let set2 = HashSet::<i32, _>::with_capacity_and_hasher(200, SipHashBuilder::default());
        assert!(set2.capacity() >= 200);
        assert!(set2.is_empty());
    }

    #[test]
    fn test_insert_and_contains() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());

        assert!(set.insert(1));
        assert_eq!(set.len(), 1);
        assert!(!set.is_empty());
        assert!(set.contains(&1));

        assert!(!set.insert(1));
        assert_eq!(set.len(), 1);
        assert!(set.contains(&1));

        assert!(set.insert(2));
        assert_eq!(set.len(), 2);
        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(!set.contains(&3));
    }

    #[test]
    fn test_remove() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());
        set.insert(1);
        set.insert(2);
        set.insert(3);

        assert!(set.remove(&2));
        assert_eq!(set.len(), 2);
        assert!(set.contains(&1));
        assert!(!set.contains(&2));
        assert!(set.contains(&3));

        assert!(!set.remove(&2));
        assert!(!set.remove(&4));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_take() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());
        set.insert(1);
        set.insert(2);

        assert_eq!(set.take(&1), Some(1));
        assert_eq!(set.len(), 1);
        assert!(!set.contains(&1));
        assert!(set.contains(&2));

        assert_eq!(set.take(&1), None);
        assert_eq!(set.take(&3), None);
    }

    #[test]
    fn test_get() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());
        set.insert(42);

        assert_eq!(set.get(&42), Some(&42));
        assert_eq!(set.get(&1), None);
    }

    #[test]
    fn test_clear() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());
        set.insert(1);
        set.insert(2);
        set.insert(3);

        assert_eq!(set.len(), 3);
        set.clear();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
        assert!(!set.contains(&1));
        assert!(!set.contains(&2));
        assert!(!set.contains(&3));
    }

    #[test]
    fn test_reserve() {
        let mut set = HashSet::<i32, _>::with_hasher(SipHashBuilder::default());
        let initial_capacity = set.capacity();

        set.reserve(1000);
        assert!(set.capacity() >= initial_capacity + 1000);
    }

    #[test]
    fn test_iter() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());
        set.insert(1);
        set.insert(2);
        set.insert(3);

        let values: Vec<i32> = set.iter().copied().collect();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&1));
        assert!(values.contains(&2));
        assert!(values.contains(&3));
    }

    #[test]
    fn test_into_iterator() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());
        set.insert(1);
        set.insert(2);
        set.insert(3);

        let values: Vec<i32> = (&set).into_iter().copied().collect();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&1));
        assert!(values.contains(&2));
        assert!(values.contains(&3));
    }

    #[test]
    fn test_drain() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());
        set.insert(1);
        set.insert(2);
        set.insert(3);

        let drained: Vec<i32> = set.drain().collect();
        assert_eq!(drained.len(), 3);
        assert!(set.is_empty());

        assert!(drained.contains(&1));
        assert!(drained.contains(&2));
        assert!(drained.contains(&3));
    }

    #[test]
    fn test_multiple_insertions() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());

        for i in 0..100 {
            assert!(set.insert(i));
        }

        assert_eq!(set.len(), 100);

        for i in 0..100 {
            assert!(set.contains(&i));
        }

        for i in 0..100 {
            assert!(!set.insert(i));
        }

        assert_eq!(set.len(), 100);
    }

    #[test]
    fn test_collision_handling() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());

        for i in 0..1000 {
            assert!(set.insert(i));
        }

        assert_eq!(set.len(), 1000);

        for i in 0..1000 {
            assert!(set.contains(&i));
        }

        for i in (0..1000).step_by(2) {
            assert!(set.remove(&i));
        }

        assert_eq!(set.len(), 500);

        for i in (1..1000).step_by(2) {
            assert!(set.contains(&i));
        }

        for i in (0..1000).step_by(2) {
            assert!(!set.contains(&i));
        }
    }

    #[test]
    fn test_string_values() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());

        assert!(set.insert("hello".to_string()));
        assert!(set.insert("world".to_string()));
        assert!(set.insert("rust".to_string()));

        assert!(set.contains(&"hello".to_string()));
        assert!(set.contains(&"world".to_string()));
        assert!(set.contains(&"rust".to_string()));
        assert!(!set.contains(&"missing".to_string()));

        assert_eq!(set.len(), 3);

        assert!(!set.insert("hello".to_string()));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_default_trait() {
        let set: HashSet<i32, SipHashBuilder> = HashSet::default();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_complex_values() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());

        let vec1 = vec![1, 2, 3];
        let vec2 = vec![4, 5, 6];
        let vec3 = vec![1, 2, 3];

        assert!(set.insert(vec1.clone()));
        assert!(set.insert(vec2.clone()));
        assert!(!set.insert(vec3));

        assert_eq!(set.len(), 2);
        assert!(set.contains(&vec1));
        assert!(set.contains(&vec2));
    }

    #[test]
    fn test_edge_cases() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());

        let empty_set = HashSet::<i32, _>::with_capacity_and_hasher(0, SipHashBuilder::default());
        assert_eq!(empty_set.len(), 0);

        assert!(!set.remove(&1));
        assert_eq!(set.take(&1), None);

        assert_eq!(set.get(&1), None);

        set.clear();
        assert!(set.is_empty());

        assert_eq!(set.iter().count(), 0);

        assert_eq!(set.drain().count(), 0);
    }

    #[test]
    fn test_insert_remove_cycle() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());

        for _ in 0..10 {
            for i in 0..50 {
                assert!(set.insert(i));
            }
            assert_eq!(set.len(), 50);

            for i in 0..50 {
                assert!(set.remove(&i));
            }
            assert_eq!(set.len(), 0);
            assert!(set.is_empty());
        }
    }

    #[test]
    fn test_large_values() {
        let mut set = HashSet::with_hasher(SipHashBuilder::default());

        for i in 0..100 {
            let large_string = "x".repeat(1000) + &i.to_string();
            assert!(set.insert(large_string.clone()));
            assert!(set.contains(&large_string))
        }

        assert_eq!(set.len(), 100);
    }

    #[test]
    fn test_numeric_types() {
        let mut u8_set = HashSet::with_hasher(SipHashBuilder::default());
        let mut u64_set = HashSet::with_hasher(SipHashBuilder::default());
        let mut i32_set = HashSet::with_hasher(SipHashBuilder::default());

        for i in 0u8..=255u8 {
            u8_set.insert(i);
        }
        assert_eq!(u8_set.len(), 256);

        for i in 0u64..100u64 {
            u64_set.insert(i * 1_000_000_000);
        }
        assert_eq!(u64_set.len(), 100);

        for i in -50i32..50i32 {
            i32_set.insert(i);
        }
        assert_eq!(i32_set.len(), 100);
    }
}
