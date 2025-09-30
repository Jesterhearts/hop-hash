use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use clap::Parser;
use hop_hash::HashTable;
use hop_hash::hash_table::Entry;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short = 'c', long = "target_capacity", default_value_t = 1000)]
    target_capacity: usize,
}

fn hash_u64(value: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn main() {
    let args = Args::parse();

    println!(
        "Creating HashTable with target capacity: {}",
        args.target_capacity
    );

    let mut table: HashTable<u64> = HashTable::with_capacity(args.target_capacity);

    println!("Actual capacity: {}", table.capacity());
    println!("Filling table with u64 values...");

    let num_values = table.capacity();
    for i in 0..num_values {
        let value = i as u64;
        let hash = hash_u64(value);

        match table.entry(hash, |&v| v == value, |&v| hash_u64(v)) {
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
            Entry::Occupied(_) => {
                panic!("Value already exists in table: {}", value);
            }
        }
    }

    println!("Inserted {} values into table", table.len());
    println!(
        "Final load factor: {:.2}%",
        (table.len() as f64 / table.capacity() as f64) * 100.0
    );

    table.probe_histogram().print();
    table.debug_stats().print();
}
