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

impl<T, S> PartialEq for HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter().all(|v| other.contains(v))
    }
}

impl<T, S> Eq for HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use std::collections::hash_map::RandomState;
    ///
    /// use hop_hash::hash_set::HashSet;
    ///
    /// let set: HashSet<i32, _> = HashSet::with_hasher(RandomState::new());
    /// assert!(set.is_empty());
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use std::collections::hash_map::RandomState;
    ///
    /// use hop_hash::hash_set::HashSet;
    ///
    /// let set: HashSet<i32, _> = HashSet::with_capacity_and_hasher(100, RandomState::new());
    /// assert!(set.capacity() >= 100);
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// assert_eq!(set.len(), 0);
    /// set.insert(1);
    /// assert_eq!(set.len(), 1);
    /// # }
    /// ```
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// assert!(set.is_empty());
    /// set.insert(1);
    /// assert!(!set.is_empty());
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let set: HashSet<i32> = HashSet::with_capacity(100);
    /// assert!(set.capacity() >= 100);
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// assert!(!set.is_empty());
    /// set.clear();
    /// assert!(set.is_empty());
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::with_capacity(1000);
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
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// assert_eq!(set.insert(37), true);
    /// assert_eq!(set.insert(37), false);
    /// assert_eq!(set.len(), 1);
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// assert!(set.contains(&1));
    /// assert!(!set.contains(&2));
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// assert_eq!(set.remove(&1), true);
    /// assert_eq!(set.remove(&1), false);
    /// # }
    /// ```
    pub fn remove(&mut self, value: &T) -> bool {
        let hash = self.hash_builder.hash_one(value);
        self.table.remove(hash, |v| v == value).is_some()
    }

    /// Adds a value to the set, replacing the existing value, if any, that is
    /// equal to the given one. Returns the replaced value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// assert_eq!(set.replace(1), Some(1));
    /// assert_eq!(set.replace(2), None);
    /// assert_eq!(set.len(), 2);
    /// # }
    /// ```
    pub fn replace(&mut self, value: T) -> Option<T> {
        let hash = self.hash_builder.hash_one(&value);
        match self
            .table
            .entry(hash, |v| v == &value, |v| self.hash_builder.hash_one(v))
        {
            crate::hash_table::Entry::Occupied(mut entry) => {
                Some(core::mem::replace(entry.get_mut(), value))
            }
            crate::hash_table::Entry::Vacant(entry) => {
                entry.insert(value);
                None
            }
        }
    }

    /// Removes and returns the value in the set, if any, that is equal to the
    /// given one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// assert_eq!(set.take(&1), Some(1));
    /// assert_eq!(set.take(&1), None);
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// assert_eq!(set.get(&1), Some(&1));
    /// assert_eq!(set.get(&2), None);
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// set.insert(2);
    ///
    /// for value in set.iter() {
    ///     println!("Value: {}", value);
    /// }
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// set.insert(2);
    ///
    /// let values: Vec<_> = set.drain().collect();
    /// assert!(set.is_empty());
    /// assert_eq!(values.len(), 2);
    /// # }
    /// ```
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain {
            inner: self.table.drain(),
        }
    }

    /// Returns `true` if the set contains no elements in common with `other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut a: HashSet<i32> = HashSet::new();
    /// a.insert(1);
    /// a.insert(2);
    ///
    /// let mut b: HashSet<i32> = HashSet::new();
    /// b.insert(3);
    /// b.insert(4);
    ///
    /// assert!(a.is_disjoint(&b));
    /// # }
    /// ```
    pub fn is_disjoint(&self, other: &HashSet<T, S>) -> bool {
        if self.len() <= other.len() {
            self.iter().all(|v| !other.contains(v))
        } else {
            other.iter().all(|v| !self.contains(v))
        }
    }

    /// Returns `true` if the set is a subset of another, i.e., `other`
    /// contains at least all the elements in `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut a: HashSet<i32> = HashSet::new();
    /// a.insert(1);
    /// a.insert(2);
    ///
    /// let mut b: HashSet<i32> = HashSet::new();
    /// b.insert(1);
    /// b.insert(2);
    /// b.insert(3);
    ///
    /// assert!(a.is_subset(&b));
    /// # }
    /// ```
    pub fn is_subset(&self, other: &HashSet<T, S>) -> bool {
        if self.len() > other.len() {
            return false;
        }
        self.iter().all(|v| other.contains(v))
    }

    /// Returns `true` if the set is a superset of another, i.e., `self`
    /// contains at least all the elements in `other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut a: HashSet<i32> = HashSet::new();
    /// a.insert(1);
    /// a.insert(2);
    /// a.insert(3);
    ///
    /// let mut b: HashSet<i32> = HashSet::new();
    /// b.insert(1);
    /// b.insert(2);
    ///
    /// assert!(a.is_superset(&b));
    /// # }
    /// ```
    pub fn is_superset(&self, other: &HashSet<T, S>) -> bool {
        other.is_subset(self)
    }

    /// Returns an iterator over the union of `self` and `other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut a: HashSet<i32> = HashSet::new();
    /// a.insert(1);
    /// a.insert(2);
    ///
    /// let mut b: HashSet<i32> = HashSet::new();
    /// b.insert(2);
    /// b.insert(3);
    ///
    /// let union: Vec<_> = a.union(&b).copied().collect();
    /// assert_eq!(union.len(), 3);
    /// # }
    /// ```
    pub fn union<'a>(&'a self, other: &'a HashSet<T, S>) -> Union<'a, T, S> {
        Union {
            iter: self.iter(),
            other_iter: other.iter(),
            other_set: self,
        }
    }

    /// Returns an iterator over the intersection of `self` and `other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut a: HashSet<i32> = HashSet::new();
    /// a.insert(1);
    /// a.insert(2);
    ///
    /// let mut b: HashSet<i32> = HashSet::new();
    /// b.insert(2);
    /// b.insert(3);
    ///
    /// let intersection: Vec<_> = a.intersection(&b).copied().collect();
    /// assert_eq!(intersection.len(), 1);
    /// # }
    /// ```
    pub fn intersection<'a>(&'a self, other: &'a HashSet<T, S>) -> Intersection<'a, T, S> {
        if self.len() <= other.len() {
            Intersection {
                iter: self.iter(),
                other,
            }
        } else {
            Intersection {
                iter: other.iter(),
                other: self,
            }
        }
    }

    /// Returns an iterator over the difference of `self` and `other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut a: HashSet<i32> = HashSet::new();
    /// a.insert(1);
    /// a.insert(2);
    ///
    /// let mut b: HashSet<i32> = HashSet::new();
    /// b.insert(2);
    /// b.insert(3);
    ///
    /// let difference: Vec<_> = a.difference(&b).copied().collect();
    /// assert_eq!(difference.len(), 1);
    /// # }
    /// ```
    pub fn difference<'a>(&'a self, other: &'a HashSet<T, S>) -> Difference<'a, T, S> {
        Difference {
            iter: self.iter(),
            other,
        }
    }

    /// Returns an iterator over the symmetric difference of `self` and `other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut a: HashSet<i32> = HashSet::new();
    /// a.insert(1);
    /// a.insert(2);
    ///
    /// let mut b: HashSet<i32> = HashSet::new();
    /// b.insert(2);
    /// b.insert(3);
    ///
    /// let sym_diff: Vec<_> = a.symmetric_difference(&b).copied().collect();
    /// assert_eq!(sym_diff.len(), 2);
    /// # }
    /// ```
    pub fn symmetric_difference<'a>(
        &'a self,
        other: &'a HashSet<T, S>,
    ) -> SymmetricDifference<'a, T, S> {
        SymmetricDifference {
            iter: self.difference(other).chain(other.difference(self)),
        }
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` for which `f(&e)` returns
    /// `false`. The elements are visited in unsorted (and unspecified)
    /// order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// set.insert(2);
    /// set.insert(3);
    /// set.insert(4);
    ///
    /// set.retain(|&x| x % 2 == 0);
    /// assert_eq!(set.len(), 2);
    /// assert!(set.contains(&2));
    /// assert!(set.contains(&4));
    /// # }
    /// ```
    pub fn retain(&mut self, f: impl FnMut(&T) -> bool) {
        self.table.retain(f, |k| self.hash_builder.hash_one(k));
    }

    /// Creates an iterator that removes and yields elements from the set for
    /// which the predicate returns `true`.
    ///
    /// If the iterator is not exhausted, e.g. because it is dropped, the
    /// remaining elements satisfying the predicate will still be removed
    /// from the set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let mut set: HashSet<i32> = HashSet::new();
    /// set.insert(1);
    /// set.insert(2);
    /// set.insert(3);
    /// set.insert(4);
    ///
    /// let extracted: Vec<_> = set.extract_if(|&mut x| x % 2 == 0).collect();
    /// assert_eq!(set.len(), 2);
    /// assert_eq!(extracted.len(), 2);
    /// assert!(set.contains(&1));
    /// assert!(set.contains(&3));
    /// # }
    /// ```
    pub fn extract_if<F>(&mut self, f: F) -> ExtractIf<'_, T, F>
    where
        F: FnMut(&mut T) -> bool,
    {
        ExtractIf {
            inner: self
                .table
                .extract_if(f, Box::new(|k| self.hash_builder.hash_one(k))),
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let set: HashSet<i32> = HashSet::new();
    /// assert!(set.is_empty());
    /// # }
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
    /// # #[cfg(any(feature = "std", feature = "foldhash"))]
    /// # {
    /// use hop_hash::HashSet;
    ///
    /// let set: HashSet<i32> = HashSet::with_capacity(100);
    /// assert!(set.capacity() >= 100);
    /// # }
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

/// A consuming iterator over the values of a `HashSet`.
pub struct IntoIter<T> {
    inner: crate::hash_table::IntoIter<T>,
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

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<T, S> IntoIterator for HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    type IntoIter = IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.table.into_iter(),
        }
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

impl<T, S> FromIterator<T> for HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = HashSet::new();
        for value in iter {
            set.insert(value);
        }
        set
    }
}

impl<T, S> Extend<T> for HashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for value in iter {
            self.insert(value);
        }
    }
}

/// An iterator over the union of two sets.
pub struct Union<'a, T, S> {
    iter: Iter<'a, T>,
    other_iter: Iter<'a, T>,
    other_set: &'a HashSet<T, S>,
}

impl<'a, T, S> Iterator for Union<'a, T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(v) = self.iter.next() {
            return Some(v);
        }
        loop {
            let v = self.other_iter.next()?;
            if !self.other_set.contains(v) {
                return Some(v);
            }
        }
    }
}

/// An iterator over the intersection of two sets.
pub struct Intersection<'a, T, S> {
    iter: Iter<'a, T>,
    other: &'a HashSet<T, S>,
}

impl<'a, T, S> Iterator for Intersection<'a, T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let v = self.iter.next()?;
            if self.other.contains(v) {
                return Some(v);
            }
        }
    }
}

/// An iterator over the difference of two sets.
pub struct Difference<'a, T, S> {
    iter: Iter<'a, T>,
    other: &'a HashSet<T, S>,
}

impl<'a, T, S> Iterator for Difference<'a, T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let v = self.iter.next()?;
            if !self.other.contains(v) {
                return Some(v);
            }
        }
    }
}

/// An iterator over the symmetric difference of two sets.
pub struct SymmetricDifference<'a, T, S> {
    iter: core::iter::Chain<Difference<'a, T, S>, Difference<'a, T, S>>,
}

impl<'a, T, S> Iterator for SymmetricDifference<'a, T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// An iterator that removes and yields all values from the set that satisfy
/// a given predicate.
pub struct ExtractIf<'a, T, F> {
    #[allow(clippy::type_complexity)]
    inner: crate::hash_table::ExtractIf<'a, T, F, Box<dyn Fn(&T) -> u64 + 'a>>,
}

impl<T, F> Iterator for ExtractIf<'_, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
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

    #[test]
    fn test_is_disjoint() {
        let mut a = HashSet::with_hasher(SipHashBuilder::default());
        a.insert(1);
        a.insert(2);
        a.insert(3);

        let mut b = HashSet::with_hasher(SipHashBuilder::default());
        b.insert(4);
        b.insert(5);
        b.insert(6);

        assert!(a.is_disjoint(&b));
        assert!(b.is_disjoint(&a));

        b.insert(2);
        assert!(!a.is_disjoint(&b));
        assert!(!b.is_disjoint(&a));
    }

    #[test]
    fn test_is_subset() {
        let mut a = HashSet::with_hasher(SipHashBuilder::default());
        a.insert(1);
        a.insert(2);

        let mut b = HashSet::with_hasher(SipHashBuilder::default());
        b.insert(1);
        b.insert(2);
        b.insert(3);

        assert!(a.is_subset(&b));
        assert!(!b.is_subset(&a));
        assert!(a.is_subset(&a));
    }

    #[test]
    fn test_is_superset() {
        let mut a = HashSet::with_hasher(SipHashBuilder::default());
        a.insert(1);
        a.insert(2);
        a.insert(3);

        let mut b = HashSet::with_hasher(SipHashBuilder::default());
        b.insert(1);
        b.insert(2);

        assert!(a.is_superset(&b));
        assert!(!b.is_superset(&a));
        assert!(a.is_superset(&a));
    }

    #[test]
    fn test_union() {
        let mut a = HashSet::with_hasher(SipHashBuilder::default());
        a.insert(1);
        a.insert(2);
        a.insert(3);

        let mut b = HashSet::with_hasher(SipHashBuilder::default());
        b.insert(3);
        b.insert(4);
        b.insert(5);

        let union: Vec<_> = a.union(&b).copied().collect();
        assert_eq!(union.len(), 5);
        assert!(union.contains(&1));
        assert!(union.contains(&2));
        assert!(union.contains(&3));
        assert!(union.contains(&4));
        assert!(union.contains(&5));
    }

    #[test]
    fn test_intersection() {
        let mut a = HashSet::with_hasher(SipHashBuilder::default());
        a.insert(1);
        a.insert(2);
        a.insert(3);

        let mut b = HashSet::with_hasher(SipHashBuilder::default());
        b.insert(2);
        b.insert(3);
        b.insert(4);

        let intersection: Vec<_> = a.intersection(&b).copied().collect();
        assert_eq!(intersection.len(), 2);
        assert!(intersection.contains(&2));
        assert!(intersection.contains(&3));
    }

    #[test]
    fn test_difference() {
        let mut a = HashSet::with_hasher(SipHashBuilder::default());
        a.insert(1);
        a.insert(2);
        a.insert(3);

        let mut b = HashSet::with_hasher(SipHashBuilder::default());
        b.insert(2);
        b.insert(3);
        b.insert(4);

        let difference: Vec<_> = a.difference(&b).copied().collect();
        assert_eq!(difference.len(), 1);
        assert!(difference.contains(&1));
    }

    #[test]
    fn test_symmetric_difference() {
        let mut a = HashSet::with_hasher(SipHashBuilder::default());
        a.insert(1);
        a.insert(2);
        a.insert(3);

        let mut b = HashSet::with_hasher(SipHashBuilder::default());
        b.insert(2);
        b.insert(3);
        b.insert(4);

        let sym_diff: Vec<_> = a.symmetric_difference(&b).copied().collect();
        assert_eq!(sym_diff.len(), 2);
        assert!(sym_diff.contains(&1));
        assert!(sym_diff.contains(&4));
    }
}
