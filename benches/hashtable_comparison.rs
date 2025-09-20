use core::hash::Hash;
use core::hash::Hasher;
use std::collections::hash_map::DefaultHasher;
use std::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use hashbrown::hash_table::Entry as HashbrownEntry;
use hashbrown::hash_table::HashTable as HashbrownHashTable;
use hop_hash::HashTable as HopHashTable;
use rand::TryRngCore;
use rand::rngs::OsRng;

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestItem {
    key: u64,
    value: u64,
}

impl TestItem {
    fn new(key: u64) -> Self {
        black_box(Self { key, value: key })
    }
}

const SIZES: &[usize] = &[(1 << 13) - (1 << 9), (1 << 20) - (1 << 16)];

fn hash_key(key: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    black_box(hasher.finish())
}

fn bench_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_sequential");

    for size in SIZES.iter() {
        group.bench_with_input(format!("hop_hash/{}", size), size, |b, &size| {
            b.iter(|| {
                let mut table = HopHashTable::<TestItem>::with_capacity(0);
                for i in 0..size {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let item = TestItem::new(key);
                    match table.entry(hash, |v| v.key == key) {
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
                    for i in 0..size {
                        let key = i as u64;
                        let hash = hash_key(key);
                        let item = TestItem::new(key);
                        match table.entry(hash, |v| v.key == key) {
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

        group.bench_with_input(format!("hashbrown/{}", size), size, |b, &size| {
            b.iter(|| {
                let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                for i in 0..size {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let item = TestItem::new(key);
                    match table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
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
                    for i in 0..size {
                        let key = i as u64;
                        let hash = hash_key(key);
                        let item = TestItem::new(key);
                        match table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
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

        group.bench_function(format!("hop_hash/{}", size), |b| {
            b.iter(|| {
                let mut table = HopHashTable::<TestItem>::with_capacity(0);
                for &key in keys {
                    let hash = hash_key(key);
                    let item = TestItem::new(key);
                    match table.entry(hash, |v| v.key == key) {
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
                for &key in keys {
                    let hash = hash_key(key);
                    let item = TestItem::new(key);
                    match table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
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
        let mut hop_table = HopHashTable::<TestItem>::with_capacity(0);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(0);

        for i in 0..*size {
            let key = i as u64;
            let hash = hash_key(key);
            let item = TestItem::new(key);

            match hop_table.entry(hash, |v| v.key == key) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item.clone());
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }

            match hashbrown_table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.bench_with_input(format!("hop_hash/{}", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = hop_table.find(hash, |v| v.key == key);
                    black_box(result);
                }
            })
        });

        group.bench_with_input(format!("hashbrown/{}", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = hashbrown_table.find(hash, |v| v.key == key);
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
        let mut hop_table = HopHashTable::<TestItem>::with_capacity(0);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(0);

        for i in (0..*size).step_by(2) {
            let key = i as u64;
            let hash = hash_key(key);
            let item = TestItem::new(key);

            match hop_table.entry(hash, |v| v.key == key) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item.clone());
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }

            match hashbrown_table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.bench_with_input(format!("hop_hash/hits/{}", size), size, |b, &size| {
            b.iter(|| {
                for i in (0..size).step_by(2) {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = hop_table.find(hash, |v| v.key == key);
                    black_box(result);
                }
            })
        });

        group.bench_with_input(format!("hop_hash/misses/{}", size), size, |b, &size| {
            b.iter(|| {
                for i in (1..size).step_by(2) {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = hop_table.find(hash, |v| v.key == key);
                    black_box(result);
                }
            })
        });

        group.bench_with_input(format!("hashbrown/hits/{}", size), size, |b, &size| {
            b.iter(|| {
                for i in (0..size).step_by(2) {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = hashbrown_table.find(hash, |v| v.key == key);
                    black_box(result);
                }
            })
        });

        group.bench_with_input(format!("hashbrown/misses/{}", size), size, |b, &size| {
            b.iter(|| {
                for i in (1..size).step_by(2) {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = hashbrown_table.find(hash, |v| v.key == key);
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
        group.bench_with_input(format!("hop_hash/{}", size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for i in 0..size {
                        let key = i as u64;
                        let hash = hash_key(key);
                        let item = TestItem::new(key);
                        match table.entry(hash, |v| v.key == key) {
                            hop_hash::hash_table::Entry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                        }
                    }
                    table
                },
                |mut table: HopHashTable<TestItem>| {
                    for i in 0..size {
                        let key = i as u64;
                        let hash = hash_key(key);
                        let result = table.remove(hash, |v| v.key == key);
                        black_box(result);
                    }
                    black_box(table)
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(format!("hashbrown/{}", size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for i in 0..size {
                        let key = i as u64;
                        let hash = hash_key(key);
                        let item = TestItem::new(key);
                        match table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
                            HashbrownEntry::Vacant(entry) => {
                                entry.insert(item);
                            }
                            HashbrownEntry::Occupied(_) => unreachable!(),
                        }
                    }
                    table
                },
                |mut table| {
                    for i in 0..size {
                        let key = i as u64;
                        let hash = hash_key(key);
                        let result = match table.find_entry(hash, |v| v.key == key) {
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
        let mut hop_table = HopHashTable::<TestItem>::with_capacity(0);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(0);

        for i in 0..*size {
            let key = i as u64;
            let hash = hash_key(key);
            let item = TestItem::new(key);

            match hop_table.entry(hash, |v| v.key == key) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item.clone());
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }

            match hashbrown_table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        group.bench_with_input(format!("hop_hash/{}", size), size, |b, &_size| {
            b.iter(|| {
                let mut count = 0;
                for item in hop_table.iter() {
                    black_box(item);
                    count += 1;
                }
                black_box(count)
            })
        });

        group.bench_with_input(format!("hashbrown/{}", size), size, |b, &_size| {
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
        group.bench_with_input(format!("hop_hash/{}", size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut table = HopHashTable::<TestItem>::with_capacity(0);
                    for i in 0..size {
                        let key = i as u64;
                        let hash = hash_key(key);
                        let item = TestItem::new(key);
                        match table.entry(hash, |v| v.key == key) {
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

        group.bench_with_input(format!("hashbrown/{}", size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
                    for i in 0..size {
                        let key = i as u64;
                        let hash = hash_key(key);
                        let item = TestItem::new(key);
                        match table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
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

fn bench_hash_collisions(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_collisions");
    let size = 64;

    let collision_hash = 0u64;

    group.bench_function("hop_hash/insert_collisions", |b| {
        b.iter(|| {
            let mut table = HopHashTable::<TestItem>::with_capacity(0);
            for i in 0..size {
                let key = i as u64;
                let item = TestItem::new(key);
                match table.entry(collision_hash, |v| v.key == key) {
                    hop_hash::hash_table::Entry::Vacant(entry) => {
                        black_box(entry.insert(item));
                    }
                    hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                }
            }
            black_box(table)
        })
    });

    group.bench_function("hashbrown/insert_collisions", |b| {
        b.iter(|| {
            let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);
            for i in 0..size {
                let key = i as u64;
                let item = TestItem::new(key);
                match table.entry(collision_hash, |v| v.key == key, |v| hash_key(v.key)) {
                    HashbrownEntry::Vacant(entry) => {
                        black_box(entry.insert(item));
                    }
                    HashbrownEntry::Occupied(_) => unreachable!(),
                }
            }
            black_box(table)
        })
    });

    let mut hop_table = HopHashTable::<TestItem>::with_capacity(0);
    let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(0);

    for i in 0..size {
        let key = i as u64;
        let item = TestItem::new(key);

        match hop_table.entry(collision_hash, |v| v.key == key) {
            hop_hash::hash_table::Entry::Vacant(entry) => {
                entry.insert(item.clone());
            }
            hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
        }

        match hashbrown_table.entry(collision_hash, |v| v.key == key, |v| hash_key(v.key)) {
            HashbrownEntry::Vacant(entry) => {
                entry.insert(item);
            }
            HashbrownEntry::Occupied(_) => unreachable!(),
        }
    }

    group.bench_function("hop_hash/find_collisions", |b| {
        b.iter(|| {
            for i in 0..size {
                let key = i as u64;
                let result = hop_table.find(collision_hash, |v| v.key == key);
                black_box(result);
            }
        })
    });

    group.bench_function("hashbrown/find_collisions", |b| {
        b.iter(|| {
            for i in 0..size {
                let key = i as u64;
                let result = hashbrown_table.find(collision_hash, |v| v.key == key);
                black_box(result);
            }
        })
    });

    group.finish();
}

fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");

    for size in SIZES.iter() {
        group.bench_with_input(format!("hop_hash/mixed/{}", size), size, |b, &size| {
            b.iter(|| {
                let mut table = HopHashTable::<TestItem>::with_capacity(0);

                for i in 0..size {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let item = TestItem::new(key);
                    match table.entry(hash, |v| v.key == key) {
                        hop_hash::hash_table::Entry::Vacant(entry) => {
                            entry.insert(item);
                        }
                        hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                    }
                }

                for i in 0..size {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = table.find(hash, |v| v.key == key);
                    black_box(result);
                }

                for i in (0..size).step_by(2) {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = table.remove(hash, |v| v.key == key);
                    black_box(result);
                }

                for i in size..size + size / 2 {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let item = TestItem::new(key);
                    match table.entry(hash, |v| v.key == key) {
                        hop_hash::hash_table::Entry::Vacant(entry) => {
                            entry.insert(item);
                        }
                        hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
                    }
                }

                black_box(table)
            })
        });

        group.bench_with_input(format!("hashbrown/mixed/{}", size), size, |b, &size| {
            b.iter(|| {
                let mut table = HashbrownHashTable::<TestItem>::with_capacity(0);

                for i in 0..size {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let item = TestItem::new(key);
                    match table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
                        HashbrownEntry::Vacant(entry) => {
                            entry.insert(item);
                        }
                        HashbrownEntry::Occupied(_) => unreachable!(),
                    }
                }

                for i in 0..size {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = table.find(hash, |v| v.key == key);
                    black_box(result);
                }

                for i in (0..size).step_by(2) {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let result = match table.find_entry(hash, |v| v.key == key) {
                        Ok(entry) => Some(entry.remove().0),
                        Err(_) => None,
                    };
                    black_box(result);
                }

                for i in size..size + size / 2 {
                    let key = i as u64;
                    let hash = hash_key(key);
                    let item = TestItem::new(key);
                    match table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
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

fn bench_occupancy_ratio(_c: &mut Criterion) {
    for size in SIZES.iter() {
        let mut hop_table = HopHashTable::<TestItem>::with_capacity(0);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(0);

        for i in 0..*size {
            let key = i as u64;
            let hash = hash_key(key);
            let item = TestItem::new(key);

            match hop_table.entry(hash, |v| v.key == key) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item.clone());
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }

            match hashbrown_table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        let hop_occupancy = hop_table.len() as f64 / hop_table.capacity() as f64;
        let hashbrown_occupancy = hashbrown_table.len() as f64 / hashbrown_table.capacity() as f64;

        println!(
            "Size: {}, HopHash occupancy: {:.2}%, Hashbrown occupancy: {:.2}%",
            size,
            hop_occupancy * 100.0,
            hashbrown_occupancy * 100.0
        );

        let mut hop_table = HopHashTable::<TestItem>::with_capacity(*size);
        let mut hashbrown_table = HashbrownHashTable::<TestItem>::with_capacity(*size);
        for i in 0..*size {
            let key = i as u64;
            let hash = hash_key(key);
            let item = TestItem::new(key);

            match hop_table.entry(hash, |v| v.key == key) {
                hop_hash::hash_table::Entry::Vacant(entry) => {
                    entry.insert(item.clone());
                }
                hop_hash::hash_table::Entry::Occupied(_) => unreachable!(),
            }

            match hashbrown_table.entry(hash, |v| v.key == key, |v| hash_key(v.key)) {
                HashbrownEntry::Vacant(entry) => {
                    entry.insert(item);
                }
                HashbrownEntry::Occupied(_) => unreachable!(),
            }
        }

        let hop_occupancy = hop_table.len() as f64 / hop_table.capacity() as f64;
        let hashbrown_occupancy = hashbrown_table.len() as f64 / hashbrown_table.capacity() as f64;

        println!(
            "Preallocated Size: {}, HopHash occupancy: {:.2}%, Hashbrown occupancy: {:.2}%",
            size,
            hop_occupancy * 100.0,
            hashbrown_occupancy * 100.0
        );
    }
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
    bench_hash_collisions,
    bench_mixed_workload,
    bench_occupancy_ratio,
);

criterion_main!(benches);
