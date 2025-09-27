use alloc::format;
use core::hash::Hash;
use core::hash::Hasher;
use core::hint::black_box;

use criterion::BatchSize;
use criterion::Criterion;
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestItem {
    key: String,
    value: u64,
}

impl TestItem {
    fn new(key: u64) -> Self {
        black_box(Self {
            key: format!("key_{:016X}", key),
            value: key,
        })
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

fn hash_key(key: &str) -> u64 {
    let mut hasher = SipHasher::new();
    key.hash(&mut hasher);
    black_box(hasher.finish())
}

fn bench_insert_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_random");

    let mut rng = OsRng;

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|_| {
                let key = rng.try_next_u64().unwrap();
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.into_iter().take(hashbrown_capacity) {
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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

fn bench_insert_random_preallocated(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_random_preallocated");

    let mut rng = OsRng;

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|_| {
                let key = rng.try_next_u64().unwrap();
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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

fn bench_find_hit_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_hit_miss");

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|key| {
                let item = TestItem::new(key as u64);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let misses_hash_and_key = (1..=hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|key| {
                let item = TestItem::new(key as u64);
                let hash = hash_key(&item.key);
                (hash, item.key)
            })
            .collect::<Vec<(u64, String)>>();

        let mut combined_hash_and_key = Vec::with_capacity(hop_capacity.max(hashbrown_capacity));
        for i in 0..hop_capacity.max(hashbrown_capacity) {
            if i % 2 == 0 {
                combined_hash_and_key.push((
                    hash_and_item[i / 2].0,
                    Some(hash_and_item[i / 2].1.key.clone()),
                ));
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
            match hop_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                            Some(key) => hop_table.find(*hash, |v| v.key == *key),
                            None => None,
                        };
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });

        for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
            match hashbrown_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                            Some(key) => hashbrown_table.find(*hash, |v| v.key == *key),
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

fn bench_find_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_hit");

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let mut hop_table = HopHashTable::<TestItem>::with_capacity(*size);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(*size);

        for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
            match hop_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        let result = hop_table.find(*hash, |v| v.key == item.key);
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });

        for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
            match hashbrown_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        let result = hashbrown_table.find(*hash, |v| v.key == item.key);
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_find_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_miss");

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|key| {
                let item = TestItem::new(key as u64);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let misses_hash_and_key = (1..=hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|key| {
                let item = TestItem::new(key as u64);
                let hash = hash_key(&item.key);
                (hash, item.key)
            })
            .collect::<Vec<(u64, String)>>();

        let mut hop_table = HopHashTable::<TestItem>::with_capacity(*size);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(*size);

        for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
            match hop_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        let result = hop_table.find(*hash, |v| v.key == *key);
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });

        for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
            match hashbrown_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        let result = hashbrown_table.find(*hash, |v| v.key == *key);
                        black_box(result);
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn bench_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove");

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        let result = table.remove(*hash, |v| v.key == item.key);
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        let result = match table.find_entry(*hash, |v| v.key == item.key) {
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

fn bench_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("iteration");

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let mut hop_table = HopHashTable::<TestItem>::with_capacity(0);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(0);

        for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
            match hop_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
            match hashbrown_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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

fn bench_drain(c: &mut Criterion) {
    let mut group = c.benchmark_group("drain");

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.throughput(Throughput::Elements(hop_capacity as u64));
        group.bench_function("hop_hash", |b| {
            b.iter_batched(
                || {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().take(hop_capacity).cloned() {
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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

fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let initial_hash_and_item = (0..hop_capacity.max(hashbrown_capacity))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let remove_hash_and_key = (0..hop_capacity.max(hashbrown_capacity))
            .step_by(2)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item.key)
            })
            .collect::<Vec<(u64, String)>>();

        let final_hash_and_item = (hop_capacity.max(hashbrown_capacity)
            ..(hop_capacity.max(hashbrown_capacity) + hop_capacity.max(hashbrown_capacity) / 2))
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }

                    for (hash, key) in remove_hash_and_key.iter().take(hop_capacity / 2) {
                        let result = table.remove(*hash, |v| v.key == *key);
                        black_box(result);
                    }

                    for (hash, item) in initial_hash_and_item.iter().take(hop_capacity) {
                        let result = table.find(*hash, |v| v.key == item.key);
                        black_box(result);
                    }

                    for (hash, item) in final_hash_and_item.into_iter().take(hop_capacity / 2) {
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                            HashbrownEntry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }

                    for (hash, key) in remove_hash_and_key.iter().take(hashbrown_capacity / 2) {
                        let result = match table.find_entry(*hash, |v| v.key == *key) {
                            Ok(entry) => Some(entry.remove().0),
                            Err(_) => None,
                        };
                        black_box(result);
                    }

                    for (hash, item) in initial_hash_and_item.iter().take(hashbrown_capacity) {
                        let result = table.find(*hash, |v| v.key == item.key);
                        black_box(result);
                    }

                    for (hash, item) in final_hash_and_item.into_iter().take(hashbrown_capacity / 2)
                    {
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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

fn bench_mixed_probabilistic(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_probabilistic");

    const KEY_SPACE_MULTIPLIER: u64 = 2;

    for size in SIZES.iter() {
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
                                let hash = hash_key(&item.key);
                                match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key))
                                {
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
                                let hash = hash_key(&item.key);
                                black_box(table.remove(hash, |v| v.key == item.key));
                            }
                            Operation::Find => {
                                let key = rng.sample(find_remove_distr) as u64;
                                let item = TestItem::new(key);
                                let hash = hash_key(&item.key);
                                black_box(table.find(hash, |v| v.key == item.key));
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
                                let hash = hash_key(&item.key);
                                match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key))
                                {
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
                                let hash = hash_key(&item.key);
                                let result = match table.find_entry(hash, |v| v.key == item.key) {
                                    Ok(entry) => Some(entry.remove().0),
                                    Err(_) => None,
                                };
                                black_box(result);
                            }
                            Operation::Find => {
                                let key = rng.sample(find_remove_distr) as u64;
                                let item = TestItem::new(key);
                                let hash = hash_key(&item.key);
                                black_box(table.find(hash, |v| v.key == item.key));
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

fn bench_mixed_probabilistic_zipf(c: &mut Criterion) {
    for exponent in [1.0, 1.3] {
        let mut group = c.benchmark_group(format!("mixed_probabilistic_zipf_{:.01}", exponent));

        const KEY_SPACE_MULTIPLIER: u64 = 2;

        for size in SIZES.iter() {
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
                                    let hash = hash_key(&item.key);
                                    match table.entry(
                                        hash,
                                        |v| v.key == item.key,
                                        |v| hash_key(&v.key),
                                    ) {
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
                                    let hash = hash_key(&item.key);
                                    black_box(table.remove(hash, |v| v.key == item.key));
                                }
                                Operation::Find => {
                                    let key = rng.sample(find_remove_distr) as u64;
                                    let item = TestItem::new(key);
                                    let hash = hash_key(&item.key);
                                    black_box(table.find(hash, |v| v.key == item.key));
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
                                    let hash = hash_key(&item.key);
                                    match table.entry(
                                        hash,
                                        |v| v.key == item.key,
                                        |v| hash_key(&v.key),
                                    ) {
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
                                    let hash = hash_key(&item.key);
                                    let result = match table.find_entry(hash, |v| v.key == item.key)
                                    {
                                        Ok(entry) => Some(entry.remove().0),
                                        Err(_) => None,
                                    };
                                    black_box(result);
                                }
                                Operation::Find => {
                                    let key = rng.sample(find_remove_distr) as u64;
                                    let item = TestItem::new(key);
                                    let hash = hash_key(&item.key);
                                    black_box(table.find(hash, |v| v.key == item.key));
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

fn bench_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("churn");

    for size in SIZES.iter() {
        let hop_capacity = HopHashTable::<TestItem>::with_capacity(*size).capacity();
        let hashbrown_capacity = HashbrownHashTable::<TestItem>::with_capacity(*size).capacity();

        let insertions_and_removals = (0..hop_capacity.max(hashbrown_capacity))
            .flat_map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
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
    bench_mixed_workload,
    bench_mixed_probabilistic,
    bench_mixed_probabilistic_zipf,
    bench_churn,
    bench_insert_random,
    bench_insert_random_preallocated,
    bench_find_hit_miss,
    bench_find_hit,
    bench_find_miss,
    bench_remove,
    bench_iteration,
    bench_drain,
);

criterion_main!(benches);
