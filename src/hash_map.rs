use core::hash::BuildHasher;
use core::hash::Hash;

use crate::hash_table::Entry as TableEntry;
use crate::hash_table::HashTable;

/// A hash map implemented using the hopscotch HashTable as the underlying
/// storage.
///
/// `HashMap<K, V, S>` stores key-value pairs where keys implement `Hash + Eq`
/// and uses a configurable hasher builder `S` to hash keys. The underlying
/// storage uses the high-performance hopscotch hashing algorithm provided by
/// the `HashTable`.
///
/// # Performance Characteristics
///
/// - **Insert**: O(1) average, O(1) worst case (bounded by neighborhood size)
/// - **Lookup**: O(1) average, O(N) worst case (typically 16 probes)
/// - **Remove**: O(1) average, O(N) worst case
/// - **Memory**: 2 bytes per entry overhead, plus the size of `(K, V)` plus a
///   u64 for the hash
pub struct HashMap<K, V, S> {
    table: HashTable<(K, V)>,
    hash_builder: S,
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    /// Creates a new hash map with the given hasher builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let map: HashMap<i32, String, _> = HashMap::with_hasher(SimpleHasher);
    /// assert!(map.is_empty());
    /// ```
    pub fn with_hasher(hash_builder: S) -> Self {
        Self::with_capacity_and_hasher(0, hash_builder)
    }

    /// Creates a new hash map with the specified capacity and hasher builder.
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
    /// # use hop_hash::HashMap;
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
    /// let map: HashMap<i32, String, _> = HashMap::with_capacity_and_hasher(100, SimpleHasher);
    /// assert!(map.capacity() >= 100);
    /// ```
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            table: HashTable::with_capacity(capacity),
            hash_builder,
        }
    }

    /// Returns the number of elements in the map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// assert_eq!(map.len(), 0);
    /// map.insert(1, "a");
    /// assert_eq!(map.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns `true` if the map contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// assert!(map.is_empty());
    /// map.insert(1, "a");
    /// assert!(!map.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the current capacity of the map.
    ///
    /// The capacity represents the maximum number of elements the map can
    /// hold before it needs to resize.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let map: HashMap<i32, String, _> = HashMap::with_capacity_and_hasher(100, SimpleHasher);
    /// assert!(map.capacity() >= 100);
    /// ```
    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    /// Removes all elements from the map.
    ///
    /// This operation preserves the map's allocated capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// assert!(!map.is_empty());
    /// map.clear();
    /// assert!(map.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.table.clear();
    }

    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        self.table.reserve(additional);
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// assert_eq!(map.insert(37, "a"), None);
    /// assert_eq!(map.insert(37, "b"), Some("a"));
    /// assert_eq!(map.get(&37), Some(&"b"));
    /// ```
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = self.hash_builder.hash_one(&key);
        match self.table.entry(hash, |(k, _)| k == &key) {
            TableEntry::Occupied(mut entry) => {
                let old_value = core::mem::replace(&mut entry.get_mut().1, value);
                Some(old_value)
            }
            TableEntry::Vacant(entry) => {
                entry.insert((key, value));
                None
            }
        }
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), None);
    /// ```
    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = self.hash_builder.hash_one(key);
        self.table.find(hash, |(k, _)| k == key).map(|(_, v)| v)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// if let Some(x) = map.get_mut(&1) {
    ///     *x = "b";
    /// }
    /// assert_eq!(map.get(&1), Some(&"b"));
    /// ```
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let hash = self.hash_builder.hash_one(key);
        self.table.find_mut(hash, |(k, _)| k == key).map(|(_, v)| v)
    }

    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// assert!(map.contains_key(&1));
    /// assert!(!map.contains_key(&2));
    /// ```
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// assert_eq!(map.remove(&1), Some("a"));
    /// assert_eq!(map.remove(&1), None);
    /// ```
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let hash = self.hash_builder.hash_one(key);
        self.table.remove(hash, |(k, _)| k == key).map(|(_, v)| v)
    }

    /// Removes a key from the map, returning the stored key and value if the
    /// key was previously in the map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// assert_eq!(map.remove_entry(&1), Some((1, "a")));
    /// assert_eq!(map.remove_entry(&1), None);
    /// ```
    pub fn remove_entry(&mut self, key: &K) -> Option<(K, V)> {
        let hash = self.hash_builder.hash_one(key);
        self.table.remove(hash, |(k, _)| k == key)
    }

    /// Gets the given key's corresponding entry in the map for in-place
    /// manipulation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    ///
    /// map.entry(1).or_insert("a");
    /// map.entry(2).or_insert("b");
    ///
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), Some(&"b"));
    /// ```
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        let hash = self.hash_builder.hash_one(&key);
        match self.table.entry(hash, |(k, _)| k == &key) {
            TableEntry::Occupied(entry) => Entry::Occupied(OccupiedEntry { entry }),
            TableEntry::Vacant(entry) => Entry::Vacant(VacantEntry { entry, key }),
        }
    }

    /// Returns an iterator over the key-value pairs of the map.
    ///
    /// The iterator yields `(&K, &V)` pairs in an arbitrary order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// map.insert(2, "b");
    ///
    /// for (key, value) in map.iter() {
    ///     println!("Key: {}, Value: {}", key, value);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.table.iter(),
        }
    }

    /// Returns an iterator over the keys of the map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// map.insert(2, "b");
    ///
    /// let keys: Vec<_> = map.keys().collect();
    /// assert_eq!(keys.len(), 2);
    /// ```
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }

    /// Returns an iterator over the values of the map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// map.insert(2, "b");
    ///
    /// let values: Vec<_> = map.values().collect();
    /// assert_eq!(values.len(), 2);
    /// ```
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }

    /// Returns an iterator that removes and yields all key-value pairs from the
    /// map.
    ///
    /// After calling `drain()`, the map will be empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// let mut map = HashMap::with_hasher(SimpleHasher);
    /// map.insert(1, "a");
    /// map.insert(2, "b");
    ///
    /// let pairs: Vec<_> = map.drain().collect();
    /// assert!(map.is_empty());
    /// assert_eq!(pairs.len(), 2);
    /// ```
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        Drain {
            inner: self.table.drain(),
        }
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher + Default,
{
    /// Creates a new hash map using the default hasher builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// #
    /// let map: HashMap<i32, String, SimpleHasher> = HashMap::new();
    /// assert!(map.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::with_hasher(S::default())
    }

    /// Creates a new hash map with the specified capacity using the default
    /// hasher builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::hash::BuildHasher;
    /// # use siphasher::sip::SipHasher;
    /// #
    /// # use hop_hash::HashMap;
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
    /// #
    /// let map: HashMap<i32, String, SimpleHasher> = HashMap::with_capacity(100);
    /// assert!(map.capacity() >= 100);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, S::default())
    }
}

impl<K, V, S> Default for HashMap<K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A view into a single entry in the map, which may either be vacant or
/// occupied.
///
/// This enum is constructed from the [`entry`] method on [`HashMap`].
///
/// [`entry`]: HashMap::entry
pub enum Entry<'a, K, V> {
    /// A vacant entry.
    Vacant(VacantEntry<'a, K, V>),
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, K, V>),
}

impl<'a, K, V> Entry<'a, K, V> {
    /// Inserts a default value if the entry is vacant and returns a mutable
    /// reference.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Inserts a value computed from a closure if the entry is vacant and
    /// returns a mutable reference.
    pub fn or_insert_with<F>(self, default: F) -> &'a mut V
    where
        F: FnOnce() -> V,
    {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts.
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }

    /// Returns a reference to this entry's key.
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(entry) => entry.key(),
            Entry::Vacant(entry) => entry.key(),
        }
    }
}

impl<'a, K, V> Entry<'a, K, V>
where
    V: Default,
{
    /// Inserts the default value if the entry is vacant and returns a mutable
    /// reference.
    pub fn or_default(self) -> &'a mut V {
        self.or_insert_with(Default::default)
    }
}

/// A view into a vacant entry in the map.
pub struct VacantEntry<'a, K, V> {
    entry: crate::hash_table::VacantEntry<'a, (K, V)>,
    key: K,
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    /// Gets a reference to the key that would be used when inserting a value.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Take ownership of the key.
    pub fn into_key(self) -> K {
        self.key
    }

    /// Inserts the value into the map and returns a mutable reference to it.
    pub fn insert(self, value: V) -> &'a mut V {
        &mut self.entry.insert((self.key, value)).1
    }
}

/// A view into an occupied entry in the map.
pub struct OccupiedEntry<'a, K, V> {
    entry: crate::hash_table::OccupiedEntry<'a, (K, V)>,
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &K {
        &self.entry.get().0
    }

    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &V {
        &self.entry.get().1
    }

    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.entry.get_mut().1
    }

    /// Converts the entry into a mutable reference to the value.
    pub fn into_mut(self) -> &'a mut V {
        &mut self.entry.into_mut().1
    }

    /// Inserts a value into the entry and returns the old value.
    pub fn insert(&mut self, value: V) -> V {
        core::mem::replace(&mut self.entry.get_mut().1, value)
    }

    /// Removes the entry from the map and returns the value.
    pub fn remove(self) -> V {
        self.entry.remove().1
    }

    /// Removes the entry from the map and returns the key and value.
    pub fn remove_entry(self) -> (K, V) {
        self.entry.remove()
    }
}

/// An iterator over the key-value pairs of a `HashMap`.
pub struct Iter<'a, K, V> {
    inner: crate::hash_table::Iter<'a, (K, V)>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (k, v))
    }
}

/// An iterator over the keys of a `HashMap`.
pub struct Keys<'a, K, V> {
    inner: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }
}

/// An iterator over the values of a `HashMap`.
pub struct Values<'a, K, V> {
    inner: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

/// A draining iterator over the key-value pairs of a `HashMap`.
pub struct Drain<'a, K, V> {
    inner: crate::hash_table::Drain<'a, (K, V)>,
}

impl<'a, K, V> Iterator for Drain<'a, K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, K, V> Drop for Drain<'a, K, V> {
    fn drop(&mut self) {
        for _ in self {}
    }
}
