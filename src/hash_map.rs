use core::fmt::Debug;
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
/// - **Memory**: 2 bytes per entry overhead, plus the size of `(K, V)` plus a
///   u64 for the hash
#[derive(Clone)]
pub struct HashMap<K, V, S> {
    table: HashTable<(K, V)>,
    hash_builder: S,
}

impl<K, V, S> Debug for HashMap<K, V, S>
where
    K: Debug + Hash + Eq,
    V: Debug,
    S: BuildHasher,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut map = f.debug_map();
        for (k, v) in self.iter() {
            map.entry(k, v);
        }
        map.finish()
    }
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

    /// Shrinks the capacity of the map as much as possible.
    ///
    /// This method will shrink the underlying storage to fit the current number
    /// of key-value pairs, potentially freeing unused memory. The resulting
    /// capacity will be at least as large as the number of elements in the map,
    /// but may be larger due to the bucket-based organization of the underlying
    /// HashTable.
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
    /// let mut map = HashMap::with_capacity_and_hasher(100, SimpleHasher);
    /// map.insert(1, "one");
    /// map.insert(2, "two");
    ///
    /// // The map has a large capacity but only 2 elements
    /// assert!(map.capacity() >= 100);
    /// assert_eq!(map.len(), 2);
    ///
    /// map.shrink_to_fit();
    ///
    /// // The capacity is now much smaller, but still fits the elements
    /// assert!(map.capacity() >= 2);
    /// assert!(map.capacity() < 100);
    /// assert_eq!(map.len(), 2);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        self.table.shrink_to_fit();
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

#[cfg(test)]
mod tests {
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
            let mut rng = OsRng;
            Self {
                k1: rng.try_next_u64().unwrap_or(0),
                k2: rng.try_next_u64().unwrap_or(0),
            }
        }
    }

    #[test]
    fn test_new_and_with_hasher() {
        let map: HashMap<i32, String, SipHashBuilder> = HashMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);

        let map2 = HashMap::<i32, String, _>::with_hasher(SipHashBuilder::default());
        assert!(map2.is_empty());
        assert_eq!(map2.len(), 0);
    }

    #[test]
    fn test_with_capacity() {
        let map: HashMap<i32, String, SipHashBuilder> = HashMap::with_capacity(100);
        assert!(map.capacity() >= 100);
        assert!(map.is_empty());

        let map2 =
            HashMap::<i32, String, _>::with_capacity_and_hasher(200, SipHashBuilder::default());
        assert!(map2.capacity() >= 200);
        assert!(map2.is_empty());
    }

    #[test]
    fn test_insert_and_get() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());

        assert_eq!(map.insert(1, "hello".to_string()), None);
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());

        assert_eq!(map.get(&1), Some(&"hello".to_string()));
        assert_eq!(map.get(&2), None);

        assert_eq!(
            map.insert(1, "world".to_string()),
            Some("hello".to_string())
        );
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&1), Some(&"world".to_string()));
    }

    #[test]
    fn test_get_mut() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());
        map.insert(1, "hello".to_string());

        if let Some(value) = map.get_mut(&1) {
            value.push_str(" world");
        }

        assert_eq!(map.get(&1), Some(&"hello world".to_string()));
        assert_eq!(map.get_mut(&2), None);
    }

    #[test]
    fn test_contains_key() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());
        assert!(!map.contains_key(&1));

        map.insert(1, "value".to_string());
        assert!(map.contains_key(&1));
        assert!(!map.contains_key(&2));
    }

    #[test]
    fn test_remove() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());
        map.insert(1, "hello".to_string());
        map.insert(2, "world".to_string());

        assert_eq!(map.remove(&1), Some("hello".to_string()));
        assert_eq!(map.len(), 1);
        assert!(!map.contains_key(&1));
        assert!(map.contains_key(&2));

        assert_eq!(map.remove(&1), None);
        assert_eq!(map.remove(&3), None);
    }

    #[test]
    fn test_remove_entry() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());
        map.insert(1, "hello".to_string());

        assert_eq!(map.remove_entry(&1), Some((1, "hello".to_string())));
        assert_eq!(map.len(), 0);
        assert_eq!(map.remove_entry(&1), None);
    }

    #[test]
    fn test_clear() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());
        map.insert(1, "hello".to_string());
        map.insert(2, "world".to_string());

        assert_eq!(map.len(), 2);
        map.clear();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
        assert!(!map.contains_key(&1));
        assert!(!map.contains_key(&2));
    }

    #[test]
    fn test_reserve() {
        let mut map = HashMap::<i32, String, _>::with_hasher(SipHashBuilder::default());
        let initial_capacity = map.capacity();

        map.reserve(1000);
        assert!(map.capacity() >= initial_capacity + 1000);
    }

    #[test]
    fn test_entry_api() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());

        let value = map.entry(1).or_insert("hello".to_string());
        assert_eq!(value, &"hello".to_string());
        assert_eq!(map.len(), 1);

        let value = map.entry(1).or_insert("world".to_string());
        assert_eq!(value, &"hello".to_string());
        assert_eq!(map.len(), 1);

        map.entry(2).or_insert_with(|| "computed".to_string());
        assert_eq!(map.get(&2), Some(&"computed".to_string()));

        map.entry(1)
            .and_modify(|v| v.push_str(" world"))
            .or_insert("default".to_string());
        assert_eq!(map.get(&1), Some(&"hello world".to_string()));

        assert_eq!(map.entry(3).key(), &3);
    }

    #[test]
    fn test_entry_or_default() {
        let mut map: HashMap<i32, Vec<i32>, SipHashBuilder> =
            HashMap::with_hasher(SipHashBuilder::default());

        map.entry(1).or_default().push(42);
        assert_eq!(map.get(&1), Some(&vec![42]));

        map.entry(1).or_default().push(24);
        assert_eq!(map.get(&1), Some(&vec![42, 24]));
    }

    #[test]
    fn test_occupied_entry() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());
        map.insert(1, "hello".to_string());

        match map.entry(1) {
            Entry::Occupied(mut entry) => {
                assert_eq!(entry.key(), &1);
                assert_eq!(entry.get(), &"hello".to_string());

                *entry.get_mut() = "world".to_string();
                assert_eq!(entry.get(), &"world".to_string());

                let old_value = entry.insert("new".to_string());
                assert_eq!(old_value, "world".to_string());
                assert_eq!(entry.get(), &"new".to_string());

                let (key, value) = entry.remove_entry();
                assert_eq!(key, 1);
                assert_eq!(value, "new".to_string());
            }
            Entry::Vacant(_) => panic!("Expected occupied entry"),
        }

        assert!(map.is_empty());
    }

    #[test]
    fn test_vacant_entry() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());

        match map.entry(1) {
            Entry::Vacant(entry) => {
                assert_eq!(entry.key(), &1);

                let value = entry.insert("hello".to_string());
                assert_eq!(value, &"hello".to_string());
            }
            Entry::Occupied(_) => panic!("Expected vacant entry"),
        }

        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&1), Some(&"hello".to_string()));
    }

    #[test]
    fn test_iterators() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());
        map.insert(1, "one".to_string());
        map.insert(2, "two".to_string());
        map.insert(3, "three".to_string());

        let pairs: std::collections::HashMap<i32, String> =
            map.iter().map(|(k, v)| (*k, v.clone())).collect();
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs.get(&1), Some(&"one".to_string()));
        assert_eq!(pairs.get(&2), Some(&"two".to_string()));
        assert_eq!(pairs.get(&3), Some(&"three".to_string()));

        let keys: std::collections::HashSet<i32> = map.keys().copied().collect();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&1));
        assert!(keys.contains(&2));
        assert!(keys.contains(&3));

        let values: std::collections::HashSet<String> = map.values().cloned().collect();
        assert_eq!(values.len(), 3);
        assert!(values.contains("one"));
        assert!(values.contains("two"));
        assert!(values.contains("three"));
    }

    #[test]
    fn test_drain() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());
        map.insert(1, "one".to_string());
        map.insert(2, "two".to_string());
        map.insert(3, "three".to_string());

        let drained: std::collections::HashMap<i32, String> = map.drain().collect();
        assert_eq!(drained.len(), 3);
        assert!(map.is_empty());

        assert_eq!(drained.get(&1), Some(&"one".to_string()));
        assert_eq!(drained.get(&2), Some(&"two".to_string()));
        assert_eq!(drained.get(&3), Some(&"three".to_string()));
    }

    #[test]
    fn test_multiple_insertions() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());

        for i in 0..100 {
            map.insert(i, format!("value_{}", i));
        }

        assert_eq!(map.len(), 100);

        for i in 0..100 {
            assert_eq!(map.get(&i), Some(&format!("value_{}", i)));
        }
    }

    #[test]
    fn test_collision_handling() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());

        for i in 0..1000 {
            map.insert(i, i * 2);
        }

        assert_eq!(map.len(), 1000);

        for i in 0..1000 {
            assert_eq!(map.get(&i), Some(&(i * 2)));
        }

        for i in (0..1000).step_by(2) {
            assert_eq!(map.remove(&i), Some(i * 2));
        }

        assert_eq!(map.len(), 500);

        for i in (1..1000).step_by(2) {
            assert_eq!(map.get(&i), Some(&(i * 2)));
        }
    }

    #[test]
    fn test_string_keys() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());

        map.insert("hello".to_string(), 1);
        map.insert("world".to_string(), 2);
        map.insert("rust".to_string(), 3);

        assert_eq!(map.get(&"hello".to_string()), Some(&1));
        assert_eq!(map.get(&"world".to_string()), Some(&2));
        assert_eq!(map.get(&"rust".to_string()), Some(&3));
        assert_eq!(map.get(&"missing".to_string()), None);
    }

    #[test]
    fn test_default_trait() {
        let map: HashMap<i32, String, SipHashBuilder> = HashMap::default();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_complex_values() {
        let mut map = HashMap::with_hasher(SipHashBuilder::default());

        let vec1 = vec![1, 2, 3];
        let vec2 = vec![4, 5, 6];

        map.insert("first".to_string(), vec1.clone());
        map.insert("second".to_string(), vec2.clone());

        assert_eq!(map.get(&"first".to_string()), Some(&vec1));
        assert_eq!(map.get(&"second".to_string()), Some(&vec2));

        if let Some(v) = map.get_mut(&"first".to_string()) {
            v.push(4);
        }

        assert_eq!(map.get(&"first".to_string()), Some(&vec![1, 2, 3, 4]));
    }
}
