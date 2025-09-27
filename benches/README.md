# Benchmark Results
## Key Takeaways
- Hop-hash performs well vs Hashbrown for mixed workloads at high load factors (92%+).
- Hop-hash significantly underperforms Hashbrown for single-operation workloads (get-only or insert-only).
- Iteration performance is better than Hashbrown.

## Individual Results

In all cases, Hashbrown is represented with the red line, and Hop-hash is represented with the green line.

- CPU: AMD Ryzen AI 9 HX 370
- RAM: 32GB
- OS: Windows 11
- Rust Version: 1.89.0 (29483883e 2025-08-04)
- Default release profile
- Default features
- SipHash hasher
- Value-type is a String of length 20, generated randomly + a u64. The String is used as the key for
  hashing and comparisons.
  - The benchmark suite does include large/small value types, but these are not shown here for
    brevity. In general, the _relative_ performance of Hop-hash decreases for small value types, and
    increases for large value types.


### Mixed Workloads
#### Collect/Find
The following benchmark results show the performance of hop-hash vs hashbrown for a mixed workload
which:
- Inserts up to the target capacity & load factor, so the table is full
- Looks up all of the original elements (all hits)

This is an extremely common workload for hash tables, and hashbrown definitely has the advantage here.

![mixed workload benchmark results](images/collect_find.png)
![mixed workload benchmark results](images/collect_find_preallocated.png)

#### Insert/Remove/Get/Insert Mix
The following benchmark results show the performance of hop-hash vs hashbrown for a mixed workload
which:
- Inserts up to the target capacity & load factor, so the table is full
- Removes half of the items in the table
- Looks up all of the original elements (half will be misses)
- Inserts up to the target capacity and load factor again

![mixed workload benchmark results](images/mixed_batch.png)

#### Find/Insert/Remove Mix (50/25/25)
The following benchmark results show the performance of hop-hash vs hashbrown for a mixed workload
which randomizes between find, insert, and remove operations in a 50/25/25 ratio.

![mixed workload benchmark results](images/mixed_probabilistic.png)

#### Find/Insert/Remove Mix Zipf 1.0/1.3
The following benchmark results show the performance of hop-hash vs hashbrown for a mixed workload
which randomizes between find, insert, and remove operations using a zipf distribution with s=1.0
and s=1.3.

![mixed workload benchmark results](images/mixed_zipf_1.0.png)
![mixed workload benchmark results](images/mixed_zipf_1.3.png)

#### Churn
This benchmark simulates a workload where the table is kept at a steady state, with random inserts
and removals. A batch of items equal to 2x the target capacity is allocated, then iterated over in
random order. If an item is not in the table already, it is inserted. If it is already in the table,
it is removed. 

![churn workload benchmark results](images/churn.png)

### Single Operation Workloads
#### Iteration
The following benchmark results show the performance of hop-hash vs hashbrown for iterating over
all items in the table.

![iteration benchmark results](images/iteration.png)

#### Drain
The following benchmark results show the performance of hop-hash vs hashbrown for draining all
items from the table.

![drain benchmark results](images/drain.png)