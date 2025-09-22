use alloc::format;
use core::hash::Hash;
use core::hash::Hasher;
use core::hint::black_box;

use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use hashbrown::hash_table::Entry as HashbrownEntry;
use hashbrown::hash_table::HashTable as HashbrownHashTable;
use hop_hash::HashTable as HopHashTable;
use rand::TryRngCore;
use rand::rngs::OsRng;
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
            key: format!("key_{}", key),
            value: key,
        })
    }
}

const SIZES: &[usize] = &[
    ((1 << 10) as f32 * 0.87) as usize,
    ((1 << 15) as f32 * 0.87) as usize,
    ((1 << 19) as f32 * 0.87) as usize,
];

fn hash_key(key: &str) -> u64 {
    let mut hasher = SipHasher::new();
    key.hash(&mut hasher);
    black_box(hasher.finish())
}

fn bench_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_sequential");

    for size in SIZES.iter() {
        let hash_and_item = (0..*size)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.bench_function(format!("hop_hash/{}", size), |b| {
            b.iter(|| {
                let mut table = HopHashTable::<TestItem>::with_capacity(0);
                for (hash, item) in hash_and_item.iter().cloned() {
                    match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                        hop_hash::hash_table::Entry::Vacant(entry) => {
                            black_box(entry.insert(item));
                        }
                        hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                    }
                }
                black_box(table)
            })
        });
        group.bench_with_input(
            format!("hop_hash_preallocated/{}", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut table = HopHashTable::<TestItem>::with_capacity(size);
                    for (hash, item) in hash_and_item.iter().cloned() {
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                black_box(entry.insert(item));
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }
                    black_box(table)
                })
            },
        );

        group.bench_function(format!("hashbrown/{}", size), |b| {
            b.iter(|| {
                let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                for (hash, item) in hash_and_item.iter().cloned() {
                    match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                        HashbrownEntry::Vacant(entry) => {
                            black_box(entry.insert(item));
                        }
                        HashbrownEntry::Occupied(_) => unreachable!(),
                    }
                }
                black_box(table)
            })
        });

        group.bench_with_input(
            format!("hashbrown_preallocated/{}", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(size);
                    for (hash, item) in hash_and_item.iter().cloned() {
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                            HashbrownEntry::Vacant(entry) => {
                                black_box(entry.insert(item));
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }
                    black_box(table)
                })
            },
        );
    }

    group.finish();
}

fn bench_insert_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_random");

    let mut rng = OsRng;

    for size in SIZES.iter() {
        let random_keys: Vec<u64> = (0..*size).map(|_| rng.try_next_u64().unwrap()).collect();

        let keys = &random_keys[0..*size];
        let hash_and_item = keys
            .iter()
            .map(|&key| {
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.bench_function(format!("hop_hash/{}", size), |b| {
            b.iter(|| {
                let mut table = HopHashTable::<TestItem>::with_capacity(0);
                for (hash, item) in hash_and_item.iter().cloned() {
                    match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                        hop_hash::hash_table::Entry::Vacant(entry) => {
                            black_box(entry.insert(item));
                        }
                        hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                    }
                }
                black_box(table)
            })
        });

        group.bench_function(format!("hashbrown/{}", size), |b| {
            b.iter(|| {
                let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                for (hash, item) in hash_and_item.iter().cloned() {
                    match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                        HashbrownEntry::Vacant(entry) => {
                            black_box(entry.insert(item));
                        }
                        HashbrownEntry::Occupied(_) => unreachable!(),
                    }
                }
                black_box(table)
            })
        });
    }

    group.finish();
}

fn bench_find_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_sequential");

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

        for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
            match hashbrown_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements(hop_capacity as u64));
        group.bench_function(format!("hop_hash/{}", size), |b| {
            b.iter(|| {
                for (hash, item) in hash_and_item.iter().take(hop_capacity) {
                    let result = hop_table.find(*hash, |v| v.key == item.key);
                    black_box(result);
                }
            })
        });

        group.throughput(Throughput::Elements(hashbrown_capacity as u64));
        group.bench_function(format!("hashbrown/{}", size), |b| {
            b.iter(|| {
                for (hash, item) in hash_and_item.iter().take(hashbrown_capacity) {
                    let result = hashbrown_table.find(*hash, |v| v.key == item.key);
                    black_box(result);
                }
            })
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
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let misses_hash_and_key = (1..hop_capacity.max(hashbrown_capacity) * 2)
            .step_by(2)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item.key)
            })
            .collect::<Vec<(u64, String)>>();

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

        for (hash, item) in hash_and_item.iter().take(hashbrown_capacity).cloned() {
            match hashbrown_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.throughput(Throughput::Elements((hop_capacity) as u64));
        group.bench_function(format!("hop_hash/hits/{}", size), |b| {
            b.iter(|| {
                for (hash, item) in hash_and_item.iter().take(hop_capacity) {
                    let result = hop_table.find(*hash, |v| v.key == item.key);
                    black_box(result);
                }
            })
        });

        group.bench_function(format!("hop_hash/misses/{}", size), |b| {
            b.iter(|| {
                for (hash, key) in misses_hash_and_key.iter().take(hop_capacity / 2) {
                    let result = hop_table.find(*hash, |v| v.key == *key);
                    black_box(result);
                }
            })
        });

        group.throughput(Throughput::Elements((hashbrown_capacity) as u64));
        group.bench_function(format!("hashbrown/hits/{}", size), |b| {
            b.iter(|| {
                for (hash, item) in hash_and_item.iter().take(hashbrown_capacity) {
                    let result = hashbrown_table.find(*hash, |v| v.key == item.key);
                    black_box(result);
                }
            })
        });

        group.bench_function(format!("hashbrown/misses/{}", size), |b| {
            b.iter(|| {
                for (hash, key) in misses_hash_and_key.iter().take(hashbrown_capacity) {
                    let result = hashbrown_table.find(*hash, |v| v.key == *key);
                    black_box(result);
                }
            })
        });
    }

    group.finish();
}

fn bench_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove");

    for size in SIZES.iter() {
        let hash_and_item = (0..*size)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.bench_function(format!("hop_hash/{}", size), |b| {
            b.iter_batched(
                || {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().cloned() {
                        match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }
                    table
                },
                |mut table: HopHashTable<TestItem>| {
                    for (hash, item) in hash_and_item.iter() {
                        let result = table.remove(*hash, |v| v.key == item.key);
                        black_box(result);
                    }
                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_function(format!("hashbrown/{}", size), |b| {
            b.iter_batched(
                || {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().cloned() {
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
                    for (hash, item) in hash_and_item.iter() {
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
        let hash_and_item = (0..*size)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let mut hop_table = HopHashTable::<TestItem>::with_capacity(0);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(0);

        for (hash, item) in hash_and_item.iter().cloned() {
            match hop_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item.clone());
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }

            match hashbrown_table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.bench_function(format!("hop_hash/{}", size), |b| {
            b.iter(|| {
                let mut count = 0;
                for item in hop_table.iter() {
                    black_box(item);
                    count += 1;
                }
                black_box(count)
            })
        });

        group.bench_function(format!("hashbrown/{}", size), |b| {
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
        let hash_and_item = (0..*size)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.bench_function(format!("hop_hash/{}", size), |b| {
            b.iter_batched(
                || {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().cloned() {
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

        group.bench_function(format!("hashbrown/{}", size), |b| {
            b.iter_batched(
                || {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for (hash, item) in hash_and_item.iter().cloned() {
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
        let initial_hash_and_item = (0..*size)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        let remove_hash_and_key = (0..*size)
            .step_by(2)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item.key)
            })
            .collect::<Vec<(u64, String)>>();

        let final_hash_and_item = (*size..*size + *size / 2)
            .map(|i| {
                let key = i as u64;
                let item = TestItem::new(key);
                let hash = hash_key(&item.key);
                (hash, item)
            })
            .collect::<Vec<(u64, TestItem)>>();

        group.bench_function(format!("hop_hash/mixed/{}", size), |b| {
            b.iter(|| {
                let mut table = HopHashTable::<TestItem>::with_capacity(0);

                for (hash, item) in initial_hash_and_item.iter().cloned() {
                    match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                        hop_hash::hash_table::Entry::Vacant(entry) => {
                            entry.insert(item);
                        }
                        hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                    }
                }

                for (hash, item) in initial_hash_and_item.iter() {
                    let result = table.find(*hash, |v| v.key == item.key);
                    black_box(result);
                }

                for (hash, key) in remove_hash_and_key.iter() {
                    let result = table.remove(*hash, |v| v.key == *key);
                    black_box(result);
                }

                for (hash, item) in final_hash_and_item.iter().cloned() {
                    match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                        hop_hash::hash_table::Entry::Vacant(entry) => {
                            entry.insert(item);
                        }
                        hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                    }
                }

                black_box(table)
            })
        });

        group.bench_function(format!("hashbrown/mixed/{}", size), |b| {
            b.iter(|| {
                let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);

                for (hash, item) in initial_hash_and_item.iter().cloned() {
                    match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                        HashbrownEntry::Vacant(entry) => {
                            entry.insert(item);
                        }
                        HashbrownEntry::Occupied(_) => unreachable!(),
                    }
                }

                for (hash, item) in initial_hash_and_item.iter() {
                    let result = table.find(*hash, |v| v.key == item.key);
                    black_box(result);
                }

                for (hash, key) in remove_hash_and_key.iter() {
                    let result = match table.find_entry(*hash, |v| v.key == *key) {
                        Ok(entry) => Some(entry.remove().0),
                        Err(_) => None,
                    };
                    black_box(result);
                }

                for (hash, item) in final_hash_and_item.iter().cloned() {
                    match table.entry(hash, |v| v.key == item.key, |v| hash_key(&v.key)) {
                        HashbrownEntry::Vacant(entry) => {
                            entry.insert(item);
                        }
                        HashbrownEntry::Occupied(_) => unreachable!(),
                    }
                }

                black_box(table)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_insert_sequential,
    bench_insert_random,
    bench_find_sequential,
    bench_find_hit_miss,
    bench_remove,
    bench_iteration,
    bench_drain,
    bench_mixed_workload,
);

criterion_main!(benches);
