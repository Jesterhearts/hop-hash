use alloc::format;
use core::hash::Hash;
use core::hash::Hasher;
use core::hint::black_box;

use criterion::AxisScale;
use criterion::BatchSize;
use criterion::Criterion;
use criterion::PlotConfiguration;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use hashbrown::hash_table::Entry as HashbrownEntry;
use hashbrown::hash_table::HashTable as HashbrownHashTable;
use hop_hash::HashTable as HopHashTable;
use rand::Rng;
use rand::SeedableRng;
use rand::TryRngCore;
use rand::distr;
use rand::rngs::OsRng;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand_distr::Zipf;
use siphasher::sip::SipHasher;

extern crate alloc;

trait KeyValuePair: Clone {
    fn new(key: u64) -> Self;

    fn hash_key(&self) -> u64;
    fn eq_key(&self, other: &Self) -> bool;
}

#[derive(Clone)]
struct TestItem {
    key: String,
    _value: u64,
}

impl KeyValuePair for TestItem {
    fn new(key: u64) -> Self {
        black_box(Self {
            key: format!("key_{:016X}", key),
            _value: key,
        })
    }

    fn hash_key(&self) -> u64 {
        let mut hasher = SipHasher::new();
        self.key.hash(&mut hasher);
        hasher.finish()
    }

    fn eq_key(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

#[derive(Clone)]
struct SmallTestItem {
    key: u64,
}

impl KeyValuePair for SmallTestItem {
    fn new(key: u64) -> Self {
        black_box(Self { key })
    }

    fn hash_key(&self) -> u64 {
        let mut hasher = SipHasher::new();
        self.key.hash(&mut hasher);
        hasher.finish()
    }

    fn eq_key(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

#[derive(Clone)]
struct LargeTestItem {
    key: String,
    _value: [u8; 256],
}

impl KeyValuePair for LargeTestItem {
    fn new(key: u64) -> Self {
        let mut value = [0u8; 256];
        for (i, byte) in value.iter_mut().enumerate() {
            *byte = ((key >> ((i % 8) * 8)) & 0xFF) as u8;
        }
        black_box(Self {
            key: format!("key_{:064b}", key),
            _value: value,
        })
    }

    fn hash_key(&self) -> u64 {
        let mut hasher = SipHasher::new();
        self.key.hash(&mut hasher);
        hasher.finish()
    }

    fn eq_key(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

const SIZES: &[usize] = &[
    (1 << 10),
    (1 << 11),
    (1 << 12),
    (1 << 13),
    (1 << 14),
    (1 << 15),
    (1 << 16),
    (1 << 17),
    (1 << 18),
];

fn bench_insert_random<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "insert_random_{}",
        core::any::type_name::<TestItem>()
    ));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let mut rng = OsRng;

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|_| {
                let key = rng.try_next_u64().unwrap();
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.throughput(Throughput::Elements(hop_capacity as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = hash_and_item.clone();
                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    hash_and_item
                },
                |hash_and_item| {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.into_iter().take(hop_capacity) {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                black_box(entry.insert(item));
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }
                    black_box(table)
                },
                BatchSize::SmallInput,
            )
        });

        group.throughput(Throughput::Elements(hashbrown_capacity as u64));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = hash_and_item.clone();
                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    hash_and_item
                },
                |hash_and_item| {
                    let mut table = HashbrownHashTable::with_capacity(0);
                    for (hash, item) in hash_and_item.into_iter().take(hashbrown_capacity) {
                        match table.entry(hash, |v: &TestItem| v.eq_key(&item), |v| v.hash_key()) {
                            HashbrownEntry::Vacant(entry) => {
                                black_box(entry.insert(item));
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }
                    black_box(table)
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_insert_random_preallocated<TestItem: KeyValuePair, const MAX_SIZE: usize>(
    c: &mut Criterion,
) {
    let mut group = c.benchmark_group(format!(
        "insert_random_preallocated_{}",
        core::any::type_name::<TestItem>()
    ));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let mut rng = OsRng;

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|_| {
                let key = rng.try_next_u64().unwrap();
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.throughput(Throughput::Elements(hop_capacity as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = hash_and_item.clone();
                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    hash_and_item
                },
                |hash_and_item| {
                    let mut table = HopHashTable::<TestItem>::with_capacity(hop_capacity);
                    for (hash, item) in hash_and_item.into_iter().take(hop_capacity) {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                black_box(entry.insert(item));
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }
                    black_box(table)
                },
                BatchSize::SmallInput,
            )
        });

        group.throughput(Throughput::Elements(hashbrown_capacity as u64));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = hash_and_item.clone();
                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    hash_and_item
                },
                |hash_and_item| {
                    let mut table =
                        HashbrownHashTable::<TestItem>::with_capacity(hashbrown_capacity);
                    for (hash, item) in hash_and_item.into_iter().take(hashbrown_capacity) {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            HashbrownEntry::Vacant(entry) => {
                                black_box(entry.insert(item));
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }
                    black_box(table)
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn bench_collect_find<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "collect_find_{}",
        core::any::type_name::<TestItem>()
    ));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.throughput(Throughput::Elements(hop_capacity as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || hash_and_item.clone(),
                |hash_and_item| {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }

                    for (hash, item) in hash_and_item.iter().take(hop_capacity) {
                        let result = table.find(*hash, |v| v.eq_key(item));
                        black_box(result);
                    }
                    black_box(table)
                },
                BatchSize::SmallInput,
            )
        });

        group.throughput(Throughput::Elements(hashbrown_capacity as u64));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || hash_and_item.clone(),
                |hash_and_item| {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            HashbrownEntry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }

                    for (hash, item) in hash_and_item.iter().take(hashbrown_capacity) {
                        let result = table.find(*hash, |v| v.eq_key(item));
                        black_box(result);
                    }
                    black_box(table)
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_collect_find_preallocated<TestItem: KeyValuePair, const MAX_SIZE: usize>(
    c: &mut Criterion,
) {
    let mut group = c.benchmark_group(format!(
        "collect_find_preallocated_{}",
        core::any::type_name::<TestItem>()
    ));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.throughput(Throughput::Elements(hop_capacity as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || hash_and_item.clone(),
                |hash_and_item| {
                    let mut table = HopHashTable::<TestItem>::with_capacity(*size);
                    for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }

                    for (hash, item) in hash_and_item.iter().take(hop_capacity) {
                        let result = table.find(*hash, |v| v.eq_key(item));
                        black_box(result);
                    }
                    black_box(table)
                },
                BatchSize::SmallInput,
            )
        });

        group.throughput(Throughput::Elements(hashbrown_capacity as u64));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || hash_and_item.clone(),
                |hash_and_item| {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(*size);
                    for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            HashbrownEntry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }
                    for (hash, item) in hash_and_item.iter().take(hashbrown_capacity) {
                        let result = table.find(*hash, |v| v.eq_key(item));
                        black_box(result);
                    }
                    black_box(table)
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_find_hit_miss<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "find_hit_miss_{}",
        core::any::type_name::<TestItem>()
    ));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|key| {
                let item = TestItem::new(key as u64);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let misses_hash_and_key = (1..=hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|key| {
                let item = TestItem::new(key as u64);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let mut combined_hash_and_key = Vec::with_capacity(hop_capacity.max(hashbrown_capacity));
        for i in 0..hop_capacity.max(hashbrown_capacity) {
            if i % 2 == 0 {
                combined_hash_and_key
                    .push((hash_and_item[i / 2].0, Some(hash_and_item[i / 2].1.clone())));
            } else {
                combined_hash_and_key.push((
                    misses_hash_and_key[i / 2].0,
                    Some(misses_hash_and_key[i / 2].1.clone()),
                ));
            }
        }

        let mut hop_table = HopHashTable::<TestItem>::with_capacity(*size);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(*size);

        for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
            match hop_table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item.clone());
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements((hop_capacity) as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut combined_hash_and_key = combined_hash_and_key.clone();
                    combined_hash_and_key.shuffle(&mut SmallRng::from_os_rng());
                    combined_hash_and_key
                },
                |combined_hash_and_key| {
                    for (hash, key_opt) in combined_hash_and_key.iter().take(hop_capacity) {
                        let result = match key_opt {
                            Some(key) => hop_table.find(*hash, |v| v.eq_key(key)),
                            None => None,
                        };
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });

        for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
            match hashbrown_table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements((hashbrown_capacity) as u64));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut combined_hash_and_key = combined_hash_and_key.clone();
                    combined_hash_and_key.shuffle(&mut SmallRng::from_os_rng());
                    combined_hash_and_key
                },
                |combined_hash_and_key| {
                    for (hash, key_opt) in combined_hash_and_key.iter().take(hashbrown_capacity) {
                        let result = match key_opt {
                            Some(key) => hashbrown_table.find(*hash, |v| v.eq_key(key)),
                            None => None,
                        };
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }
}

fn bench_find_hit<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("find_hit_{}", core::any::type_name::<TestItem>()));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let mut hop_table = HopHashTable::<TestItem>::with_capacity(*size);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(*size);

        for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
            match hop_table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item.clone());
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements((hop_capacity) as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = hash_and_item.clone();
                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    hash_and_item
                },
                |hash_and_item| {
                    for (hash, item) in hash_and_item.iter().take(hop_capacity) {
                        let result = hop_table.find(*hash, |v| v.eq_key(item));
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });

        for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
            match hashbrown_table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements((hashbrown_capacity) as u64));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = hash_and_item.clone();
                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    hash_and_item
                },
                |hash_and_item| {
                    for (hash, item) in hash_and_item.iter().take(hashbrown_capacity) {
                        let result = hashbrown_table.find(*hash, |v| v.eq_key(item));
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_find_miss<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("find_miss_{}", core::any::type_name::<TestItem>()));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|key| {
                let item = TestItem::new(key as u64);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let misses_hash_and_key = (1..=hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|key| {
                let item = TestItem::new(key as u64);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let mut hop_table = HopHashTable::<TestItem>::with_capacity(*size);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(*size);

        for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
            match hop_table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item);
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements((hop_capacity) as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut misses_hash_and_key = misses_hash_and_key.clone();
                    misses_hash_and_key.shuffle(&mut SmallRng::from_os_rng());
                    misses_hash_and_key
                },
                |misses_hash_and_key| {
                    for (hash, key) in misses_hash_and_key.iter().take(hop_capacity) {
                        let result = hop_table.find(*hash, |v| v.eq_key(key));
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });

        for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
            match hashbrown_table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements((hashbrown_capacity) as u64));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut misses_hash_and_key = misses_hash_and_key.clone();
                    misses_hash_and_key.shuffle(&mut SmallRng::from_os_rng());
                    misses_hash_and_key
                },
                |misses_hash_and_key| {
                    for (hash, key) in misses_hash_and_key.iter().take(hashbrown_capacity) {
                        let result = hashbrown_table.find(*hash, |v| v.eq_key(key));
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn bench_remove<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("remove_{}", core::any::type_name::<TestItem>()));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.throughput(Throughput::Elements(hop_capacity as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = hash_and_item.clone();

                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }

                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    (table, hash_and_item)
                },
                |(mut table, hash_and_item)| {
                    for (hash, item) in hash_and_item.iter().take(hop_capacity) {
                        let result = table.remove(*hash, |v| v.eq_key(item));
                        black_box(result);
                    }
                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.throughput(Throughput::Elements(hashbrown_capacity as u64));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = hash_and_item.clone();

                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            HashbrownEntry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }

                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    (table, hash_and_item)
                },
                |(mut table, hash_and_item)| {
                    for (hash, item) in hash_and_item.iter().take(hashbrown_capacity) {
                        let result = match table.find_entry(*hash, |v| v.eq_key(item)) {
                            Ok(entry) => Some(entry.remove().0),
                            Err(_) => None,
                        };
                        black_box(result);
                    }
                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_iteration<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("iteration_{}", core::any::type_name::<TestItem>()));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let mut hop_table = HopHashTable::<TestItem>::with_capacity(0);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(0);

        for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
            match hop_table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item.clone());
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements(hop_capacity as u64));
        group.bench_function("hop_hash", |b| {
            b.iter(|| {
                let mut count = 0;
                for item in hop_table.iter() {
                    black_box(item);
                    count += 1;
                }
                black_box(count)
            })
        });

        for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
            match hashbrown_table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements(hashbrown_capacity as u64));
        group.bench_function("hashbrown", |b| {
            b.iter(|| {
                let mut count = 0;
                for item in hashbrown_table.iter() {
                    black_box(item);
                    count += 1;
                }
                black_box(count)
            })
        });
    }

    group.finish();
}

fn bench_drain<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("drain_{}", core::any::type_name::<TestItem>()));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.throughput(Throughput::Elements(hop_capacity as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }
                    table
                },
                |mut table| {
                    let mut count = 0;
                    for item in table.drain() {
                        black_box(item);
                        count += 1;
                    }
                    black_box((table, count))
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.throughput(Throughput::Elements(hashbrown_capacity as u64));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            HashbrownEntry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }
                    table
                },
                |mut table| {
                    let mut count = 0;
                    for item in table.drain() {
                        black_box(item);
                        count += 1;
                    }
                    black_box((table, count))
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_mixed_workload<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "mixed_workload_{}",
        core::any::type_name::<TestItem>()
    ));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let initial_hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let remove_hash_and_key = (0..hop_capacity.max(hashbrown_capacity))
            .step_by(2)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let final_hash_and_item = (hop_capacity.max(hashbrown_capacity)
            ..(hop_capacity.max(hashbrown_capacity) + hop_capacity.max(hashbrown_capacity) / 2))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.throughput(Throughput::Elements(hop_capacity as u64 * 2));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut initial_hash_and_item = initial_hash_and_item.clone();
                    initial_hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    let mut remove_hash_and_key = remove_hash_and_key.clone();
                    remove_hash_and_key.shuffle(&mut SmallRng::from_os_rng());
                    let mut final_hash_and_item = final_hash_and_item.clone();
                    final_hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    (
                        initial_hash_and_item,
                        remove_hash_and_key,
                        final_hash_and_item,
                    )
                },
                |(initial_hash_and_item, remove_hash_and_key, final_hash_and_item)| {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);

                    for (hash, item) in initial_hash_and_item.iter().take(hop_capacity).cloned() {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }

                    for (hash, key) in remove_hash_and_key.iter().take(hop_capacity / 2) {
                        let result = table.remove(*hash, |v| v.eq_key(key));
                        black_box(result);
                    }

                    for (hash, item) in initial_hash_and_item.iter().take(hop_capacity) {
                        let result = table.find(*hash, |v| v.eq_key(item));
                        black_box(result);
                    }

                    for (hash, item) in final_hash_and_item.into_iter().take(hop_capacity / 2) {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }

                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.throughput(Throughput::Elements(hashbrown_capacity as u64 * 2));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut initial_hash_and_item = initial_hash_and_item.clone();
                    initial_hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    let mut remove_hash_and_key = remove_hash_and_key.clone();
                    remove_hash_and_key.shuffle(&mut SmallRng::from_os_rng());
                    let mut final_hash_and_item = final_hash_and_item.clone();
                    final_hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    (
                        initial_hash_and_item,
                        remove_hash_and_key,
                        final_hash_and_item,
                    )
                },
                |(initial_hash_and_item, remove_hash_and_key, final_hash_and_item)| {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);

                    for (hash, item) in initial_hash_and_item
                        .iter()
                        .take(hashbrown_capacity)
                        .cloned()
                    {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            HashbrownEntry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }

                    for (hash, key) in remove_hash_and_key.iter().take(hashbrown_capacity / 2) {
                        let result = match table.find_entry(*hash, |v| v.eq_key(key)) {
                            Ok(entry) => Some(entry.remove().0),
                            Err(_) => None,
                        };
                        black_box(result);
                    }

                    for (hash, item) in initial_hash_and_item.iter().take(hashbrown_capacity) {
                        let result = table.find(*hash, |v| v.eq_key(item));
                        black_box(result);
                    }

                    for (hash, item) in final_hash_and_item.into_iter().take(hashbrown_capacity / 2)
                    {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            HashbrownEntry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }

                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

#[derive(Clone, Copy)]
enum Operation {
    Insert,
    Remove,
    Find,
}

fn bench_mixed_probabilistic<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "mixed_probabilistic_{}",
        core::any::type_name::<TestItem>()
    ));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    const KEY_SPACE_MULTIPLIER: u64 = 2;

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let mut rng = SmallRng::from_os_rng();

        let operations = (0..hop_capacity.max(hashbrown_capacity) * 3)
            .map(|_| {
                let op_choice: f64 = rng.sample(distr::Uniform::new(0.0, 1.0).unwrap());
                if op_choice < 0.5 {
                    Operation::Find
                } else if op_choice < 0.75 {
                    Operation::Insert
                } else {
                    Operation::Remove
                }
            })
            .collect::<Vec<Operation>>();

        let mut rng = SmallRng::from_os_rng();
        let insert_distr = Zipf::new(hop_capacity as f32 - 1.0, 1.0).unwrap();
        let find_remove_distr =
            Zipf::new(hop_capacity as f32 * KEY_SPACE_MULTIPLIER as f32 - 1.0, 1.0).unwrap();

        group.throughput(Throughput::Elements(hop_capacity as u64 * 3));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut operations = operations.clone();
                    operations.shuffle(&mut SmallRng::from_os_rng());
                    operations
                },
                |operations| {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for operation in operations.into_iter().take(hop_capacity * 3) {
                        match operation {
                            Operation::Insert => {
                                let key = rng.sample(insert_distr) as u64;
                                let item = TestItem::new(key);
                                let hash = item.hash_key();
                                match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                                    hop_hash::hash_table::Entry::Vacant(entry) => {
                                        black_box(entry.insert(item));
                                    }
                                    hop_hash::hash_table::Entry::Occupied(mut occupied) => {
                                        *occupied.get_mut() = item;
                                    }
                                }
                            }
                            Operation::Remove => {
                                let key = rng.sample(find_remove_distr) as u64;
                                let item = TestItem::new(key);
                                let hash = item.hash_key();
                                black_box(table.remove(hash, |v| v.eq_key(&item)));
                            }
                            Operation::Find => {
                                let key = rng.sample(find_remove_distr) as u64;
                                let item = TestItem::new(key);
                                let hash = item.hash_key();
                                black_box(table.find(hash, |v| v.eq_key(&item)));
                            }
                        }
                    }
                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });

        let insert_distr = Zipf::new(hashbrown_capacity as f32 - 1.0, 1.0).unwrap();
        let find_remove_distr = Zipf::new(
            hashbrown_capacity as f32 * KEY_SPACE_MULTIPLIER as f32 - 1.0,
            1.0,
        )
        .unwrap();
        group.throughput(Throughput::Elements(hashbrown_capacity as u64 * 3));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut operations = operations.clone();
                    operations.shuffle(&mut SmallRng::from_os_rng());
                    operations
                },
                |operations| {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for operation in operations.into_iter().take(hashbrown_capacity * 3) {
                        match operation {
                            Operation::Insert => {
                                let key = rng.sample(insert_distr) as u64;
                                let item = TestItem::new(key);
                                let hash = item.hash_key();
                                match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                                    HashbrownEntry::Vacant(entry) => {
                                        black_box(entry.insert(item));
                                    }
                                    HashbrownEntry::Occupied(mut occupied) => {
                                        *occupied.get_mut() = item;
                                    }
                                }
                            }
                            Operation::Remove => {
                                let key = rng.sample(find_remove_distr) as u64;
                                let item = TestItem::new(key);
                                let hash = item.hash_key();
                                let result = match table.find_entry(hash, |v| v.eq_key(&item)) {
                                    Ok(entry) => Some(entry.remove().0),
                                    Err(_) => None,
                                };
                                black_box(result);
                            }
                            Operation::Find => {
                                let key = rng.sample(find_remove_distr) as u64;
                                let item = TestItem::new(key);
                                let hash = item.hash_key();
                                black_box(table.find(hash, |v| v.eq_key(&item)));
                            }
                        }
                    }
                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_mixed_probabilistic_zipf<TestItem: KeyValuePair, const MAX_SIZE: usize>(
    c: &mut Criterion,
) {
    for exponent in [1.0, 1.3] {
        let mut group = c.benchmark_group(format!(
            "mixed_probabilistic_zipf_{:.01}_{}",
            exponent,
            core::any::type_name::<TestItem>()
        ));
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

        const KEY_SPACE_MULTIPLIER: u64 = 2;

        for size in SIZES[..=MAX_SIZE].iter() {
            let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
            let hashbrown_capacity =
                HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

            let mut rng = SmallRng::from_os_rng();

            let op_distr = Zipf::new(3.0, exponent).unwrap();

            let operations = (0..hop_capacity.max(hashbrown_capacity) * 3)
                .map(|_| {
                    let op_choice: f64 = rng.sample(op_distr);
                    if op_choice <= 1.0 {
                        Operation::Find
                    } else if op_choice <= 2.0 {
                        Operation::Insert
                    } else {
                        Operation::Remove
                    }
                })
                .collect::<Vec<Operation>>();

            let mut rng = SmallRng::from_os_rng();
            let insert_distr = Zipf::new(hop_capacity as f32 - 1.0, 1.0).unwrap();
            let find_remove_distr =
                Zipf::new(hop_capacity as f32 * KEY_SPACE_MULTIPLIER as f32 - 1.0, 1.0).unwrap();

            group.throughput(Throughput::Elements(hop_capacity as u64 * 3));
            group.bench_function("hop_hash", |b| {
                b.iter_batched(
                    || {
                        let mut operations = operations.clone();
                        operations.shuffle(&mut SmallRng::from_os_rng());
                        operations
                    },
                    |operations| {
                        let mut table = HopHashTable::<TestItem>::with_capacity(0);
                        for operation in operations.into_iter().take(hop_capacity * 3) {
                            match operation {
                                Operation::Insert => {
                                    let key = rng.sample(insert_distr) as u64;
                                    let item = TestItem::new(key);
                                    let hash = item.hash_key();
                                    match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                                        hop_hash::hash_table::Entry::Vacant(entry) => {
                                            black_box(entry.insert(item));
                                        }

                                        hop_hash::hash_table::Entry::Occupied(mut occupied) => {
                                            *occupied.get_mut() = item;
                                        }
                                    }
                                }
                                Operation::Remove => {
                                    let key = rng.sample(find_remove_distr) as u64;
                                    let item = TestItem::new(key);
                                    let hash = item.hash_key();
                                    black_box(table.remove(hash, |v| v.eq_key(&item)));
                                }
                                Operation::Find => {
                                    let key = rng.sample(find_remove_distr) as u64;
                                    let item = TestItem::new(key);
                                    let hash = item.hash_key();
                                    black_box(table.find(hash, |v| v.eq_key(&item)));
                                }
                            }
                        }
                        black_box(table)
                    },
                    criterion::BatchSize::SmallInput,
                )
            });

            let insert_distr = Zipf::new(hashbrown_capacity as f32 - 1.0, 1.0).unwrap();
            let find_remove_distr = Zipf::new(
                hashbrown_capacity as f32 * KEY_SPACE_MULTIPLIER as f32 - 1.0,
                1.0,
            )
            .unwrap();
            group.throughput(Throughput::Elements(hashbrown_capacity as u64 * 3));
            group.bench_function("hashbrown", |b| {
                b.iter_batched(
                    || {
                        let mut operations = operations.clone();
                        operations.shuffle(&mut SmallRng::from_os_rng());
                        operations
                    },
                    |operations| {
                        let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                        for operation in operations.into_iter().take(hashbrown_capacity * 3) {
                            match operation {
                                Operation::Insert => {
                                    let key = rng.sample(insert_distr) as u64;
                                    let item = TestItem::new(key);
                                    let hash = item.hash_key();
                                    match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                                        HashbrownEntry::Vacant(entry) => {
                                            black_box(entry.insert(item));
                                        }
                                        HashbrownEntry::Occupied(mut occupied) => {
                                            *occupied.get_mut() = item;
                                        }
                                    }
                                }
                                Operation::Remove => {
                                    let key = rng.sample(find_remove_distr) as u64;
                                    let item = TestItem::new(key);
                                    let hash = item.hash_key();
                                    let result = match table.find_entry(hash, |v| v.eq_key(&item)) {
                                        Ok(entry) => Some(entry.remove().0),
                                        Err(_) => None,
                                    };
                                    black_box(result);
                                }
                                Operation::Find => {
                                    let key = rng.sample(find_remove_distr) as u64;
                                    let item = TestItem::new(key);
                                    let hash = item.hash_key();
                                    black_box(table.find(hash, |v| v.eq_key(&item)));
                                }
                            }
                        }
                        black_box(table)
                    },
                    criterion::BatchSize::SmallInput,
                )
            });
        }

        group.finish();
    }
}

fn bench_churn<TestItem: KeyValuePair, const MAX_SIZE: usize>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("churn_{}", core::any::type_name::<TestItem>()));
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for size in SIZES[..=MAX_SIZE].iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let insertions_and_removals = (0..hop_capacity.max(hashbrown_capacity))
            .flat_map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = item.hash_key();
                [(hash, item.clone()), (hash, item)]
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.throughput(Throughput::Elements(hop_capacity as u64 * 2));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = insertions_and_removals.clone();
                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    hash_and_item
                },
                |hash_and_item| {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.into_iter().take(hop_capacity) {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(entry) => {
                                black_box(entry.remove());
                            }
                        }
                    }
                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.throughput(Throughput::Elements(hashbrown_capacity as u64 * 2));
        group.bench_function("hashbrown", |b| {
            b.iter_batched(
                || {
                    let mut hash_and_item = insertions_and_removals.clone();
                    hash_and_item.shuffle(&mut SmallRng::from_os_rng());
                    hash_and_item
                },
                |hash_and_item| {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.into_iter().take(hashbrown_capacity) {
                        match table.entry(hash, |v| v.eq_key(&item), |v| v.hash_key()) {
                            HashbrownEntry::Vacant(entry) => {
                                black_box(entry.insert(item));
                            }
                            HashbrownEntry::Occupied(entry) => {
                                black_box(entry.remove().0);
                            }
                        }
                    }
                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
}

criterion_group!(
    benches,
    bench_mixed_workload::<SmallTestItem, 8>,
    bench_mixed_workload::<TestItem, 8>,
    bench_mixed_workload::<LargeTestItem, 5>,
    bench_mixed_probabilistic::<SmallTestItem, 8>,
    bench_mixed_probabilistic::<TestItem, 8>,
    bench_mixed_probabilistic::<LargeTestItem, 5>,
    bench_mixed_probabilistic_zipf::<SmallTestItem, 8>,
    bench_mixed_probabilistic_zipf::<TestItem, 8>,
    bench_mixed_probabilistic_zipf::<LargeTestItem, 5>,
    bench_churn::<SmallTestItem, 8>,
    bench_churn::<TestItem, 8>,
    bench_churn::<LargeTestItem, 5>,
    bench_collect_find::<SmallTestItem, 8>,
    bench_collect_find::<TestItem, 8>,
    bench_collect_find::<LargeTestItem, 5>,
    bench_collect_find_preallocated::<SmallTestItem, 8>,
    bench_collect_find_preallocated::<TestItem, 8>,
    bench_collect_find_preallocated::<LargeTestItem, 5>,
    bench_insert_random::<SmallTestItem, 8>,
    bench_insert_random::<TestItem, 8>,
    bench_insert_random::<LargeTestItem, 5>,
    bench_insert_random_preallocated::<SmallTestItem, 8>,
    bench_insert_random_preallocated::<TestItem, 8>,
    bench_insert_random_preallocated::<LargeTestItem, 5>,
    bench_find_hit_miss::<SmallTestItem, 8>,
    bench_find_hit_miss::<TestItem, 8>,
    bench_find_hit_miss::<LargeTestItem, 5>,
    bench_find_hit::<SmallTestItem, 8>,
    bench_find_hit::<TestItem, 8>,
    bench_find_hit::<LargeTestItem, 5>,
    bench_find_miss::<SmallTestItem, 8>,
    bench_find_miss::<TestItem, 8>,
    bench_find_miss::<LargeTestItem, 5>,
    bench_remove::<SmallTestItem, 8>,
    bench_remove::<TestItem, 8>,
    bench_remove::<LargeTestItem, 5>,
    bench_iteration::<SmallTestItem, 8>,
    bench_iteration::<TestItem, 8>,
    bench_iteration::<LargeTestItem, 5>,
    bench_drain::<SmallTestItem, 8>,
    bench_drain::<TestItem, 8>,
    bench_drain::<LargeTestItem, 5>,
);

criterion_main!(benches);
