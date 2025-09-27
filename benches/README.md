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

## Selected Results Table:

| Benchmark                    |  O(Ops) | hashbrown    | hop_hash     | Relative Performance |
| :--------------------------- | ------: | :----------- | :----------- | :------------------- |
| Churn                        |    4096 | 18.15 ns/op  | 18.75 ns/op  | 0.97x                |
| Churn                        |    8192 | 19.98 ns/op  | 21.47 ns/op  | 0.93x                |
| Churn                        |   16384 | 22.65 ns/op  | 24.39 ns/op  | 0.93x                |
| Churn                        |   32768 | 23.45 ns/op  | 25.79 ns/op  | 0.91x                |
| Churn                        |   65536 | 29.96 ns/op  | 29.02 ns/op  | 1.03x                |
| Churn                        |  131072 | 47.29 ns/op  | 42.12 ns/op  | 1.12x                |
| Churn                        |  262144 | 90.57 ns/op  | 77.51 ns/op  | 1.17x                |
| Churn                        |  524288 | 120.26 ns/op | 104.76 ns/op | 1.15x                |
| Churn                        | 1048576 | 145.87 ns/op | 141.16 ns/op | 1.03x                |
| Collect Find                 |    2048 | 34.56 ns/op  | 39.90 ns/op  | 0.87x                |
| Collect Find                 |    4096 | 36.53 ns/op  | 44.95 ns/op  | 0.81x                |
| Collect Find                 |    8192 | 42.44 ns/op  | 48.08 ns/op  | 0.88x                |
| Collect Find                 |   16384 | 42.81 ns/op  | 50.78 ns/op  | 0.84x                |
| Collect Find                 |   32768 | 61.12 ns/op  | 67.06 ns/op  | 0.91x                |
| Collect Find                 |   65536 | 78.26 ns/op  | 89.22 ns/op  | 0.88x                |
| Collect Find                 |  131072 | 159.80 ns/op | 141.71 ns/op | 1.13x                |
| Collect Find                 |  262144 | 201.23 ns/op | 226.99 ns/op | 0.89x                |
| Collect Find                 |  524288 | 269.92 ns/op | 279.03 ns/op | 0.97x                |
| Collect Find Preallocated    |    2048 | 21.48 ns/op  | 25.81 ns/op  | 0.83x                |
| Collect Find Preallocated    |    4096 | 20.68 ns/op  | 26.80 ns/op  | 0.77x                |
| Collect Find Preallocated    |    8192 | 23.22 ns/op  | 34.58 ns/op  | 0.67x                |
| Collect Find Preallocated    |   16384 | 26.69 ns/op  | 36.74 ns/op  | 0.73x                |
| Collect Find Preallocated    |   32768 | 47.33 ns/op  | 52.50 ns/op  | 0.90x                |
| Collect Find Preallocated    |   65536 | 55.81 ns/op  | 77.78 ns/op  | 0.72x                |
| Collect Find Preallocated    |  131072 | 95.87 ns/op  | 111.13 ns/op | 0.86x                |
| Collect Find Preallocated    |  262144 | 139.33 ns/op | 157.90 ns/op | 0.88x                |
| Collect Find Preallocated    |  524288 | 168.71 ns/op | 187.89 ns/op | 0.90x                |
| Drain                        |    2048 | 13.07 ns/op  | 11.46 ns/op  | 1.14x                |
| Drain                        |    4096 | 13.76 ns/op  | 11.36 ns/op  | 1.21x                |
| Drain                        |    8192 | 13.67 ns/op  | 11.80 ns/op  | 1.16x                |
| Drain                        |   16384 | 15.15 ns/op  | 12.20 ns/op  | 1.24x                |
| Drain                        |   32768 | 16.37 ns/op  | 12.52 ns/op  | 1.31x                |
| Drain                        |   65536 | 18.66 ns/op  | 13.35 ns/op  | 1.40x                |
| Drain                        |  131072 | 21.00 ns/op  | 15.55 ns/op  | 1.35x                |
| Drain                        |  262144 | 47.00 ns/op  | 31.63 ns/op  | 1.49x                |
| Drain                        |  524288 | 82.31 ns/op  | 54.62 ns/op  | 1.51x                |
| Iteration                    |    2048 | 0.42 ns/op   | 0.39 ns/op   | 1.08x                |
| Iteration                    |    4096 | 0.44 ns/op   | 0.39 ns/op   | 1.11x                |
| Iteration                    |    8192 | 0.43 ns/op   | 0.37 ns/op   | 1.18x                |
| Iteration                    |   16384 | 0.44 ns/op   | 0.36 ns/op   | 1.23x                |
| Iteration                    |   32768 | 0.45 ns/op   | 0.36 ns/op   | 1.23x                |
| Iteration                    |   65536 | 0.49 ns/op   | 0.37 ns/op   | 1.32x                |
| Iteration                    |  131072 | 0.55 ns/op   | 0.39 ns/op   | 1.40x                |
| Iteration                    |  262144 | 0.60 ns/op   | 0.41 ns/op   | 1.47x                |
| Iteration                    |  524288 | 0.66 ns/op   | 0.45 ns/op   | 1.45x                |
| Mixed Probabilistic          |    8192 | 168.40 ns/op | 168.53 ns/op | 1.00x                |
| Mixed Probabilistic          |   16384 | 173.33 ns/op | 168.57 ns/op | 1.03x                |
| Mixed Probabilistic          |   32768 | 172.92 ns/op | 173.35 ns/op | 1.00x                |
| Mixed Probabilistic          |   65536 | 174.94 ns/op | 170.65 ns/op | 1.03x                |
| Mixed Probabilistic          |  131072 | 175.58 ns/op | 171.21 ns/op | 1.03x                |
| Mixed Probabilistic          |  262144 | 177.47 ns/op | 175.26 ns/op | 1.01x                |
| Mixed Probabilistic          |  524288 | 191.76 ns/op | 185.82 ns/op | 1.03x                |
| Mixed Probabilistic          | 1048576 | 236.89 ns/op | 211.27 ns/op | 1.12x                |
| Mixed Probabilistic          | 2097152 | 253.68 ns/op | 246.70 ns/op | 1.03x                |
| Mixed Probabilistic Zipf 1.0 |    8192 | 166.86 ns/op | 163.42 ns/op | 1.02x                |
| Mixed Probabilistic Zipf 1.0 |   16384 | 164.36 ns/op | 165.79 ns/op | 0.99x                |
| Mixed Probabilistic Zipf 1.0 |   32768 | 165.57 ns/op | 165.36 ns/op | 1.00x                |
| Mixed Probabilistic Zipf 1.0 |   65536 | 174.72 ns/op | 167.74 ns/op | 1.04x                |
| Mixed Probabilistic Zipf 1.0 |  131072 | 175.09 ns/op | 170.38 ns/op | 1.03x                |
| Mixed Probabilistic Zipf 1.0 |  262144 | 177.53 ns/op | 172.40 ns/op | 1.03x                |
| Mixed Probabilistic Zipf 1.0 |  524288 | 192.09 ns/op | 180.73 ns/op | 1.06x                |
| Mixed Probabilistic Zipf 1.0 | 1048576 | 239.26 ns/op | 221.58 ns/op | 1.08x                |
| Mixed Probabilistic Zipf 1.0 | 2097152 | 258.71 ns/op | 249.70 ns/op | 1.04x                |
| Mixed Probabilistic Zipf 1.3 |    8192 | 162.90 ns/op | 165.21 ns/op | 0.99x                |
| Mixed Probabilistic Zipf 1.3 |   16384 | 164.30 ns/op | 165.40 ns/op | 0.99x                |
| Mixed Probabilistic Zipf 1.3 |   32768 | 159.75 ns/op | 166.72 ns/op | 0.96x                |
| Mixed Probabilistic Zipf 1.3 |   65536 | 172.65 ns/op | 168.15 ns/op | 1.03x                |
| Mixed Probabilistic Zipf 1.3 |  131072 | 175.07 ns/op | 168.90 ns/op | 1.04x                |
| Mixed Probabilistic Zipf 1.3 |  262144 | 176.60 ns/op | 171.43 ns/op | 1.03x                |
| Mixed Probabilistic Zipf 1.3 |  524288 | 193.39 ns/op | 185.01 ns/op | 1.05x                |
| Mixed Probabilistic Zipf 1.3 | 1048576 | 223.08 ns/op | 213.92 ns/op | 1.04x                |
| Mixed Probabilistic Zipf 1.3 | 2097152 | 255.72 ns/op | 250.13 ns/op | 1.02x                |
| Mixed Workload               |    4096 | 40.43 ns/op  | 38.76 ns/op  | 1.04x                |
| Mixed Workload               |    8192 | 42.37 ns/op  | 42.28 ns/op  | 1.00x                |
| Mixed Workload               |   16384 | 46.20 ns/op  | 45.43 ns/op  | 1.02x                |
| Mixed Workload               |   32768 | 59.51 ns/op  | 47.42 ns/op  | 1.26x                |
| Mixed Workload               |   65536 | 70.30 ns/op  | 55.33 ns/op  | 1.27x                |
| Mixed Workload               |  131072 | 91.43 ns/op  | 67.25 ns/op  | 1.36x                |
| Mixed Workload               |  262144 | 160.65 ns/op | 122.19 ns/op | 1.31x                |
| Mixed Workload               |  524288 | 237.00 ns/op | 181.28 ns/op | 1.31x                |
| Mixed Workload               | 1048576 | 305.75 ns/op | 238.51 ns/op | 1.28x                |