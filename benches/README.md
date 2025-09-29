# Benchmark Results
## Key Takeaways
- Hop-hash performs well vs Hashbrown for mixed workloads while at a higher load factor (92%).
- Hop-hash significantly underperforms Hashbrown for single-operation workloads (get-only or insert-only).
- Iteration performance is better than Hashbrown.

## Individual Result Graphs

In all cases, Hashbrown is represented with the red line, and Hop-hash is represented with the green line.

- CPU: AMD Ryzen AI 9 HX 370
- RAM: 32GB
- OS: Windows 11
- Rust Version: 1.89.0 (29483883e 2025-08-04)
- Default release profile
- Default features
- SipHash hasher
- Value-type (32 bytes) is a String of length 20, generated arbitrarily, plus a u64. The String is
  used as the key for hashing and comparisons.
  - Data is pre-generated for a benchmark and then used for all iterations of that benchmark for
    both hashbrown and hop-hash. This ensures that they're running on identical data. The initial
    data is pre-hashed before any insertion/find/etc. to try to exclude hashing time from the
    benchmarks as much as possible (some rehashing still occurs during table growth, but this seems
    like a fair thing to benchmark).
  - The benchmark suite does include large (280 bytes)/small (8 byte) value types, but those charts
    are not shown here for brevity. In general, the _relative_ performance of Hop-hash decreases for
    small value types, and increases for large value types.


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

## Selected Result Tables

Keep in mind while reviewing these results that benchmarks on my machine can vary by up to 5%
(although <3% is typical) between runs even though I'm running these on a quiet system. Also keep in
mind that hop-hash is running at a load factor of 92%, while hashbrown is running at a load factor
of 87.5%.

### Benchmark: `churn` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                       |
| ----- | ------------ | ------------ | -------------------------------- |
| 1024  | 80.93 ns/op  | 76.64 ns/op  | **hop_hash** is **1.06x** faster |
| 2048  | 120.37 ns/op | 93.62 ns/op  | **hop_hash** is **1.29x** faster |
| 4096  | 147.03 ns/op | 103.32 ns/op | **hop_hash** is **1.42x** faster |
| 8192  | 152.50 ns/op | 110.68 ns/op | **hop_hash** is **1.38x** faster |
| 16384 | 196.82 ns/op | 143.96 ns/op | **hop_hash** is **1.37x** faster |
| 32768 | 221.65 ns/op | 161.27 ns/op | **hop_hash** is **1.37x** faster |


### Benchmark: `churn` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 19.55 ns/op  | 19.50 ns/op  | **hop_hash** is **1.00x** faster  |
| 2048   | 19.98 ns/op  | 21.34 ns/op  | **hashbrown** is **1.07x** faster |
| 4096   | 21.42 ns/op  | 22.52 ns/op  | **hashbrown** is **1.05x** faster |
| 8192   | 23.57 ns/op  | 25.25 ns/op  | **hashbrown** is **1.07x** faster |
| 16384  | 29.60 ns/op  | 28.88 ns/op  | **hop_hash** is **1.03x** faster  |
| 32768  | 41.68 ns/op  | 35.95 ns/op  | **hop_hash** is **1.16x** faster  |
| 65536  | 71.42 ns/op  | 62.56 ns/op  | **hop_hash** is **1.14x** faster  |
| 131072 | 109.27 ns/op | 94.95 ns/op  | **hop_hash** is **1.15x** faster  |
| 262144 | 139.23 ns/op | 127.26 ns/op | **hop_hash** is **1.09x** faster  |


### Benchmark: `collect_find` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                       |
| ----- | ------------ | ------------ | -------------------------------- |
| 1024  | 61.45 ns/op  | 60.97 ns/op  | **hop_hash** is **1.01x** faster |
| 2048  | 120.93 ns/op | 110.80 ns/op | **hop_hash** is **1.09x** faster |
| 4096  | 149.75 ns/op | 128.90 ns/op | **hop_hash** is **1.16x** faster |
| 8192  | 166.17 ns/op | 143.61 ns/op | **hop_hash** is **1.16x** faster |
| 16384 | 190.05 ns/op | 163.11 ns/op | **hop_hash** is **1.17x** faster |
| 32768 | 212.38 ns/op | 185.70 ns/op | **hop_hash** is **1.14x** faster |


### Benchmark: `collect_find` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 18.33 ns/op  | 19.43 ns/op  | **hashbrown** is **1.06x** faster |
| 2048   | 18.51 ns/op  | 22.01 ns/op  | **hashbrown** is **1.19x** faster |
| 4096   | 20.04 ns/op  | 23.95 ns/op  | **hashbrown** is **1.19x** faster |
| 8192   | 20.99 ns/op  | 25.91 ns/op  | **hashbrown** is **1.23x** faster |
| 16384  | 30.31 ns/op  | 31.98 ns/op  | **hashbrown** is **1.06x** faster |
| 32768  | 43.46 ns/op  | 42.04 ns/op  | **hop_hash** is **1.03x** faster  |
| 65536  | 72.25 ns/op  | 70.03 ns/op  | **hop_hash** is **1.03x** faster  |
| 131072 | 100.20 ns/op | 102.15 ns/op | **hashbrown** is **1.02x** faster |
| 262144 | 130.68 ns/op | 137.10 ns/op | **hashbrown** is **1.05x** faster |


### Benchmark: `collect_find_preallocated` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                        |
| ----- | ------------ | ------------ | --------------------------------- |
| 1024  | 37.75 ns/op  | 52.08 ns/op  | **hashbrown** is **1.38x** faster |
| 2048  | 90.71 ns/op  | 106.52 ns/op | **hashbrown** is **1.17x** faster |
| 4096  | 93.71 ns/op  | 109.86 ns/op | **hashbrown** is **1.17x** faster |
| 8192  | 92.87 ns/op  | 103.95 ns/op | **hashbrown** is **1.12x** faster |
| 16384 | 103.21 ns/op | 117.65 ns/op | **hashbrown** is **1.14x** faster |
| 32768 | 118.07 ns/op | 135.81 ns/op | **hashbrown** is **1.15x** faster |


### Benchmark: `collect_find_preallocated` | Item Type: `TestItem`

| Size   | hashbrown   | hop_hash    | Comparison                        |
| ------ | ----------- | ----------- | --------------------------------- |
| 1024   | 9.55 ns/op  | 12.43 ns/op | **hashbrown** is **1.30x** faster |
| 2048   | 9.86 ns/op  | 13.22 ns/op | **hashbrown** is **1.34x** faster |
| 4096   | 11.00 ns/op | 14.61 ns/op | **hashbrown** is **1.33x** faster |
| 8192   | 12.26 ns/op | 15.86 ns/op | **hashbrown** is **1.29x** faster |
| 16384  | 23.78 ns/op | 27.11 ns/op | **hashbrown** is **1.14x** faster |
| 32768  | 28.66 ns/op | 31.65 ns/op | **hashbrown** is **1.10x** faster |
| 65536  | 42.87 ns/op | 48.47 ns/op | **hashbrown** is **1.13x** faster |
| 131072 | 60.73 ns/op | 72.29 ns/op | **hashbrown** is **1.19x** faster |
| 262144 | 79.33 ns/op | 91.84 ns/op | **hashbrown** is **1.16x** faster |


### Benchmark: `drain` | Item Type: `LargeTestItem`

| Size  | hashbrown   | hop_hash    | Comparison                        |
| ----- | ----------- | ----------- | --------------------------------- |
| 1024  | 29.23 ns/op | 24.90 ns/op | **hop_hash** is **1.17x** faster  |
| 2048  | 24.99 ns/op | 30.87 ns/op | **hashbrown** is **1.24x** faster |
| 4096  | 24.27 ns/op | 26.31 ns/op | **hashbrown** is **1.08x** faster |
| 8192  | 28.81 ns/op | 20.76 ns/op | **hop_hash** is **1.39x** faster  |
| 16384 | 54.06 ns/op | 29.31 ns/op | **hop_hash** is **1.84x** faster  |
| 32768 | 93.34 ns/op | 63.70 ns/op | **hop_hash** is **1.47x** faster  |


### Benchmark: `drain` | Item Type: `TestItem`

| Size   | hashbrown   | hop_hash    | Comparison                       |
| ------ | ----------- | ----------- | -------------------------------- |
| 1024   | 13.19 ns/op | 11.55 ns/op | **hop_hash** is **1.14x** faster |
| 2048   | 13.30 ns/op | 11.14 ns/op | **hop_hash** is **1.19x** faster |
| 4096   | 13.45 ns/op | 11.15 ns/op | **hop_hash** is **1.21x** faster |
| 8192   | 14.15 ns/op | 11.98 ns/op | **hop_hash** is **1.18x** faster |
| 16384  | 16.21 ns/op | 12.29 ns/op | **hop_hash** is **1.32x** faster |
| 32768  | 18.21 ns/op | 13.11 ns/op | **hop_hash** is **1.39x** faster |
| 65536  | 21.25 ns/op | 14.34 ns/op | **hop_hash** is **1.48x** faster |
| 131072 | 42.96 ns/op | 33.51 ns/op | **hop_hash** is **1.28x** faster |
| 262144 | 77.76 ns/op | 70.31 ns/op | **hop_hash** is **1.11x** faster |

### Benchmark: `iteration` | Item Type: `LargeTestItem`

| Size  | hashbrown  | hop_hash   | Comparison                       |
| ----- | ---------- | ---------- | -------------------------------- |
| 1024  | 1.37 ns/op | 0.42 ns/op | **hop_hash** is **3.27x** faster |
| 2048  | 1.38 ns/op | 0.40 ns/op | **hop_hash** is **3.43x** faster |
| 4096  | 1.39 ns/op | 0.40 ns/op | **hop_hash** is **3.48x** faster |
| 8192  | 1.38 ns/op | 0.40 ns/op | **hop_hash** is **3.47x** faster |
| 16384 | 1.38 ns/op | 0.41 ns/op | **hop_hash** is **3.39x** faster |
| 32768 | 1.39 ns/op | 0.42 ns/op | **hop_hash** is **3.30x** faster |


### Benchmark: `iteration` | Item Type: `TestItem`

| Size   | hashbrown  | hop_hash   | Comparison                       |
| ------ | ---------- | ---------- | -------------------------------- |
| 1024   | 0.43 ns/op | 0.38 ns/op | **hop_hash** is **1.15x** faster |
| 2048   | 0.44 ns/op | 0.38 ns/op | **hop_hash** is **1.16x** faster |
| 4096   | 0.43 ns/op | 0.37 ns/op | **hop_hash** is **1.18x** faster |
| 8192   | 0.44 ns/op | 0.35 ns/op | **hop_hash** is **1.25x** faster |
| 16384  | 0.45 ns/op | 0.37 ns/op | **hop_hash** is **1.22x** faster |
| 32768  | 0.49 ns/op | 0.38 ns/op | **hop_hash** is **1.29x** faster |
| 65536  | 0.54 ns/op | 0.39 ns/op | **hop_hash** is **1.37x** faster |
| 131072 | 0.62 ns/op | 0.42 ns/op | **hop_hash** is **1.49x** faster |
| 262144 | 0.66 ns/op | 0.46 ns/op | **hop_hash** is **1.44x** faster |


### Benchmark: `mixed_probabilistic` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                       |
| ----- | ------------ | ------------ | -------------------------------- |
| 1024  | 426.03 ns/op | 399.20 ns/op | **hop_hash** is **1.07x** faster |
| 2048  | 438.03 ns/op | 408.05 ns/op | **hop_hash** is **1.07x** faster |
| 4096  | 436.45 ns/op | 408.73 ns/op | **hop_hash** is **1.07x** faster |
| 8192  | 451.81 ns/op | 412.49 ns/op | **hop_hash** is **1.10x** faster |
| 16384 | 474.55 ns/op | 430.95 ns/op | **hop_hash** is **1.10x** faster |
| 32768 | 491.22 ns/op | 452.52 ns/op | **hop_hash** is **1.09x** faster |


### Benchmark: `mixed_probabilistic` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 167.83 ns/op | 167.88 ns/op | **hashbrown** is **1.00x** faster |
| 2048   | 168.49 ns/op | 167.82 ns/op | **hop_hash** is **1.00x** faster  |
| 4096   | 170.06 ns/op | 169.12 ns/op | **hop_hash** is **1.01x** faster  |
| 8192   | 177.85 ns/op | 170.85 ns/op | **hop_hash** is **1.04x** faster  |
| 16384  | 179.24 ns/op | 173.21 ns/op | **hop_hash** is **1.03x** faster  |
| 32768  | 179.10 ns/op | 173.38 ns/op | **hop_hash** is **1.03x** faster  |
| 65536  | 193.44 ns/op | 183.81 ns/op | **hop_hash** is **1.05x** faster  |
| 131072 | 217.55 ns/op | 206.12 ns/op | **hop_hash** is **1.06x** faster  |
| 262144 | 247.12 ns/op | 235.01 ns/op | **hop_hash** is **1.05x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.0` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                       |
| ----- | ------------ | ------------ | -------------------------------- |
| 1024  | 430.89 ns/op | 405.49 ns/op | **hop_hash** is **1.06x** faster |
| 2048  | 444.08 ns/op | 410.61 ns/op | **hop_hash** is **1.08x** faster |
| 4096  | 445.81 ns/op | 411.23 ns/op | **hop_hash** is **1.08x** faster |
| 8192  | 458.02 ns/op | 418.05 ns/op | **hop_hash** is **1.10x** faster |
| 16384 | 482.80 ns/op | 436.82 ns/op | **hop_hash** is **1.11x** faster |
| 32768 | 512.27 ns/op | 445.49 ns/op | **hop_hash** is **1.15x** faster |

### Benchmark: `mixed_probabilistic_zipf_1.0` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 168.95 ns/op | 169.16 ns/op | **hashbrown** is **1.00x** faster |
| 2048   | 170.04 ns/op | 166.76 ns/op | **hop_hash** is **1.02x** faster  |
| 4096   | 170.28 ns/op | 167.10 ns/op | **hop_hash** is **1.02x** faster  |
| 8192   | 176.68 ns/op | 168.68 ns/op | **hop_hash** is **1.05x** faster  |
| 16384  | 178.00 ns/op | 170.44 ns/op | **hop_hash** is **1.04x** faster  |
| 32768  | 177.45 ns/op | 173.57 ns/op | **hop_hash** is **1.02x** faster  |
| 65536  | 191.70 ns/op | 179.60 ns/op | **hop_hash** is **1.07x** faster  |
| 131072 | 216.76 ns/op | 211.21 ns/op | **hop_hash** is **1.03x** faster  |
| 262144 | 250.98 ns/op | 234.73 ns/op | **hop_hash** is **1.07x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.3` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                       |
| ----- | ------------ | ------------ | -------------------------------- |
| 1024  | 434.90 ns/op | 404.96 ns/op | **hop_hash** is **1.07x** faster |
| 2048  | 442.85 ns/op | 406.77 ns/op | **hop_hash** is **1.09x** faster |
| 4096  | 447.31 ns/op | 407.15 ns/op | **hop_hash** is **1.10x** faster |
| 8192  | 467.58 ns/op | 416.85 ns/op | **hop_hash** is **1.12x** faster |
| 16384 | 485.15 ns/op | 432.85 ns/op | **hop_hash** is **1.12x** faster |
| 32768 | 500.76 ns/op | 447.54 ns/op | **hop_hash** is **1.12x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.3` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                       |
| ------ | ------------ | ------------ | -------------------------------- |
| 1024   | 164.06 ns/op | 163.90 ns/op | **hop_hash** is **1.00x** faster |
| 2048   | 164.88 ns/op | 164.25 ns/op | **hop_hash** is **1.00x** faster |
| 4096   | 169.29 ns/op | 165.66 ns/op | **hop_hash** is **1.02x** faster |
| 8192   | 176.60 ns/op | 169.09 ns/op | **hop_hash** is **1.04x** faster |
| 16384  | 175.35 ns/op | 171.07 ns/op | **hop_hash** is **1.03x** faster |
| 32768  | 177.38 ns/op | 174.13 ns/op | **hop_hash** is **1.02x** faster |
| 65536  | 190.02 ns/op | 182.24 ns/op | **hop_hash** is **1.04x** faster |
| 131072 | 217.14 ns/op | 208.16 ns/op | **hop_hash** is **1.04x** faster |
| 262144 | 250.32 ns/op | 241.93 ns/op | **hop_hash** is **1.03x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.8` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                       |
| ----- | ------------ | ------------ | -------------------------------- |
| 1024  | 426.13 ns/op | 400.27 ns/op | **hop_hash** is **1.06x** faster |
| 2048  | 442.20 ns/op | 405.11 ns/op | **hop_hash** is **1.09x** faster |
| 4096  | 448.31 ns/op | 405.21 ns/op | **hop_hash** is **1.11x** faster |
| 8192  | 460.55 ns/op | 413.17 ns/op | **hop_hash** is **1.11x** faster |
| 16384 | 483.36 ns/op | 431.05 ns/op | **hop_hash** is **1.12x** faster |
| 32768 | 503.63 ns/op | 452.15 ns/op | **hop_hash** is **1.11x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.8` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 165.22 ns/op | 163.25 ns/op | **hop_hash** is **1.01x** faster  |
| 2048   | 161.73 ns/op | 163.56 ns/op | **hashbrown** is **1.01x** faster |
| 4096   | 165.27 ns/op | 165.68 ns/op | **hashbrown** is **1.00x** faster |
| 8192   | 172.56 ns/op | 167.69 ns/op | **hop_hash** is **1.03x** faster  |
| 16384  | 172.95 ns/op | 169.76 ns/op | **hop_hash** is **1.02x** faster  |
| 32768  | 175.03 ns/op | 170.08 ns/op | **hop_hash** is **1.03x** faster  |
| 65536  | 187.73 ns/op | 182.07 ns/op | **hop_hash** is **1.03x** faster  |
| 131072 | 217.47 ns/op | 208.20 ns/op | **hop_hash** is **1.04x** faster  |
| 262144 | 246.75 ns/op | 237.85 ns/op | **hop_hash** is **1.04x** faster  |


### Benchmark: `mixed_workload` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                       |
| ----- | ------------ | ------------ | -------------------------------- |
| 1024  | 159.56 ns/op | 78.04 ns/op  | **hop_hash** is **2.04x** faster |
| 2048  | 233.82 ns/op | 129.00 ns/op | **hop_hash** is **1.81x** faster |
| 4096  | 273.95 ns/op | 188.12 ns/op | **hop_hash** is **1.46x** faster |
| 8192  | 357.19 ns/op | 208.69 ns/op | **hop_hash** is **1.71x** faster |
| 16384 | 429.71 ns/op | 260.19 ns/op | **hop_hash** is **1.65x** faster |
| 32768 | 483.63 ns/op | 305.44 ns/op | **hop_hash** is **1.58x** faster |


### Benchmark: `mixed_workload` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 40.71 ns/op  | 40.15 ns/op  | **hop_hash** is **1.01x** faster  |
| 2048   | 42.33 ns/op  | 42.92 ns/op  | **hashbrown** is **1.01x** faster |
| 4096   | 46.74 ns/op  | 45.44 ns/op  | **hop_hash** is **1.03x** faster  |
| 8192   | 59.69 ns/op  | 47.94 ns/op  | **hop_hash** is **1.25x** faster  |
| 16384  | 69.58 ns/op  | 54.35 ns/op  | **hop_hash** is **1.28x** faster  |
| 32768  | 97.14 ns/op  | 72.63 ns/op  | **hop_hash** is **1.34x** faster  |
| 65536  | 161.17 ns/op | 122.59 ns/op | **hop_hash** is **1.31x** faster  |
| 131072 | 231.71 ns/op | 179.94 ns/op | **hop_hash** is **1.29x** faster  |
| 262144 | 287.69 ns/op | 237.77 ns/op | **hop_hash** is **1.21x** faster  |


### Benchmark: `find_hit` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                        |
| ----- | ------------ | ------------ | --------------------------------- |
| 1024  | 35.63 ns/op  | 36.67 ns/op  | **hashbrown** is **1.03x** faster |
| 2048  | 49.15 ns/op  | 52.05 ns/op  | **hashbrown** is **1.06x** faster |
| 4096  | 60.78 ns/op  | 63.78 ns/op  | **hashbrown** is **1.05x** faster |
| 8192  | 89.30 ns/op  | 87.50 ns/op  | **hop_hash** is **1.02x** faster  |
| 16384 | 91.69 ns/op  | 94.37 ns/op  | **hashbrown** is **1.03x** faster |
| 32768 | 128.57 ns/op | 128.38 ns/op | **hop_hash** is **1.00x** faster  |


### Benchmark: `find_hit` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 14.76 ns/op  | 15.82 ns/op  | **hashbrown** is **1.07x** faster |
| 2048   | 15.68 ns/op  | 16.32 ns/op  | **hashbrown** is **1.04x** faster |
| 4096   | 18.02 ns/op  | 19.62 ns/op  | **hashbrown** is **1.09x** faster |
| 8192   | 25.29 ns/op  | 27.56 ns/op  | **hashbrown** is **1.09x** faster |
| 16384  | 25.81 ns/op  | 27.45 ns/op  | **hashbrown** is **1.06x** faster |
| 32768  | 40.19 ns/op  | 45.57 ns/op  | **hashbrown** is **1.13x** faster |
| 65536  | 61.09 ns/op  | 67.14 ns/op  | **hashbrown** is **1.10x** faster |
| 131072 | 96.35 ns/op  | 108.55 ns/op | **hashbrown** is **1.13x** faster |
| 262144 | 125.19 ns/op | 140.79 ns/op | **hashbrown** is **1.12x** faster |


### Benchmark: `find_hit_miss` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                        |
| ----- | ------------ | ------------ | --------------------------------- |
| 1024  | 40.14 ns/op  | 35.04 ns/op  | **hop_hash** is **1.15x** faster  |
| 2048  | 51.82 ns/op  | 54.41 ns/op  | **hashbrown** is **1.05x** faster |
| 4096  | 58.89 ns/op  | 62.58 ns/op  | **hashbrown** is **1.06x** faster |
| 8192  | 58.47 ns/op  | 61.98 ns/op  | **hashbrown** is **1.06x** faster |
| 16384 | 71.81 ns/op  | 88.74 ns/op  | **hashbrown** is **1.24x** faster |
| 32768 | 104.57 ns/op | 111.51 ns/op | **hashbrown** is **1.07x** faster |


### Benchmark: `find_hit_miss` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 19.46 ns/op  | 20.43 ns/op  | **hashbrown** is **1.05x** faster |
| 2048   | 20.84 ns/op  | 20.96 ns/op  | **hashbrown** is **1.01x** faster |
| 4096   | 22.31 ns/op  | 22.43 ns/op  | **hashbrown** is **1.01x** faster |
| 8192   | 22.21 ns/op  | 23.62 ns/op  | **hashbrown** is **1.06x** faster |
| 16384  | 26.51 ns/op  | 27.40 ns/op  | **hashbrown** is **1.03x** faster |
| 32768  | 40.98 ns/op  | 30.53 ns/op  | **hop_hash** is **1.34x** faster  |
| 65536  | 47.31 ns/op  | 58.94 ns/op  | **hashbrown** is **1.25x** faster |
| 131072 | 78.45 ns/op  | 97.88 ns/op  | **hashbrown** is **1.25x** faster |
| 262144 | 105.50 ns/op | 122.03 ns/op | **hashbrown** is **1.16x** faster |


### Benchmark: `find_miss` | Item Type: `LargeTestItem`

| Size  | hashbrown   | hop_hash    | Comparison                        |
| ----- | ----------- | ----------- | --------------------------------- |
| 1024  | 32.08 ns/op | 35.66 ns/op | **hashbrown** is **1.11x** faster |
| 2048  | 54.59 ns/op | 55.40 ns/op | **hashbrown** is **1.01x** faster |
| 4096  | 58.13 ns/op | 62.05 ns/op | **hashbrown** is **1.07x** faster |
| 8192  | 62.03 ns/op | 58.50 ns/op | **hop_hash** is **1.06x** faster  |
| 16384 | 60.81 ns/op | 69.48 ns/op | **hashbrown** is **1.14x** faster |
| 32768 | 88.34 ns/op | 97.17 ns/op | **hashbrown** is **1.10x** faster |


### Benchmark: `find_miss` | Item Type: `TestItem`

| Size   | hashbrown   | hop_hash    | Comparison                        |
| ------ | ----------- | ----------- | --------------------------------- |
| 1024   | 18.31 ns/op | 20.21 ns/op | **hashbrown** is **1.10x** faster |
| 2048   | 22.14 ns/op | 21.39 ns/op | **hop_hash** is **1.03x** faster  |
| 4096   | 21.72 ns/op | 21.01 ns/op | **hop_hash** is **1.03x** faster  |
| 8192   | 21.33 ns/op | 22.20 ns/op | **hashbrown** is **1.04x** faster |
| 16384  | 24.61 ns/op | 23.55 ns/op | **hop_hash** is **1.04x** faster  |
| 32768  | 29.33 ns/op | 29.15 ns/op | **hop_hash** is **1.01x** faster  |
| 65536  | 38.09 ns/op | 48.32 ns/op | **hashbrown** is **1.27x** faster |
| 131072 | 61.59 ns/op | 71.51 ns/op | **hashbrown** is **1.16x** faster |
| 262144 | 82.78 ns/op | 89.62 ns/op | **hashbrown** is **1.08x** faster |


### Benchmark: `insert_random` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                       |
| ----- | ------------ | ------------ | -------------------------------- |
| 1024  | 81.98 ns/op  | 81.29 ns/op  | **hop_hash** is **1.01x** faster |
| 2048  | 168.95 ns/op | 143.32 ns/op | **hop_hash** is **1.18x** faster |
| 4096  | 218.79 ns/op | 168.53 ns/op | **hop_hash** is **1.30x** faster |
| 8192  | 248.23 ns/op | 190.97 ns/op | **hop_hash** is **1.30x** faster |
| 16384 | 284.73 ns/op | 220.71 ns/op | **hop_hash** is **1.29x** faster |
| 32768 | 325.79 ns/op | 256.77 ns/op | **hop_hash** is **1.27x** faster |


### Benchmark: `insert_random` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 22.08 ns/op  | 25.39 ns/op  | **hashbrown** is **1.15x** faster |
| 2048   | 22.02 ns/op  | 27.58 ns/op  | **hashbrown** is **1.25x** faster |
| 4096   | 23.08 ns/op  | 29.74 ns/op  | **hashbrown** is **1.29x** faster |
| 8192   | 24.63 ns/op  | 32.66 ns/op  | **hashbrown** is **1.33x** faster |
| 16384  | 38.02 ns/op  | 41.86 ns/op  | **hashbrown** is **1.10x** faster |
| 32768  | 46.25 ns/op  | 45.22 ns/op  | **hop_hash** is **1.02x** faster  |
| 65536  | 61.04 ns/op  | 55.33 ns/op  | **hop_hash** is **1.10x** faster  |
| 131072 | 100.91 ns/op | 96.94 ns/op  | **hop_hash** is **1.04x** faster  |
| 262144 | 140.83 ns/op | 137.96 ns/op | **hop_hash** is **1.02x** faster  |


### Benchmark: `insert_random_preallocated` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                        |
| ----- | ------------ | ------------ | --------------------------------- |
| 1024  | 37.50 ns/op  | 58.47 ns/op  | **hashbrown** is **1.56x** faster |
| 2048  | 120.76 ns/op | 143.53 ns/op | **hashbrown** is **1.19x** faster |
| 4096  | 120.47 ns/op | 143.60 ns/op | **hashbrown** is **1.19x** faster |
| 8192  | 124.90 ns/op | 197.66 ns/op | **hashbrown** is **1.58x** faster |
| 16384 | 122.93 ns/op | 144.89 ns/op | **hashbrown** is **1.18x** faster |
| 32768 | 128.94 ns/op | 148.66 ns/op | **hashbrown** is **1.15x** faster |


### Benchmark: `insert_random_preallocated` | Item Type: `TestItem`

| Size   | hashbrown   | hop_hash    | Comparison                        |
| ------ | ----------- | ----------- | --------------------------------- |
| 1024   | 5.39 ns/op  | 10.67 ns/op | **hashbrown** is **1.98x** faster |
| 2048   | 5.26 ns/op  | 10.74 ns/op | **hashbrown** is **2.04x** faster |
| 4096   | 5.93 ns/op  | 11.44 ns/op | **hashbrown** is **1.93x** faster |
| 8192   | 8.07 ns/op  | 42.39 ns/op | **hashbrown** is **5.26x** faster |
| 16384  | 27.84 ns/op | 35.41 ns/op | **hashbrown** is **1.27x** faster |
| 32768  | 28.76 ns/op | 31.10 ns/op | **hashbrown** is **1.08x** faster |
| 65536  | 28.10 ns/op | 31.24 ns/op | **hashbrown** is **1.11x** faster |
| 131072 | 33.78 ns/op | 37.34 ns/op | **hashbrown** is **1.11x** faster |
| 262144 | 36.91 ns/op | 43.60 ns/op | **hashbrown** is **1.18x** faster |


### Benchmark: `remove` | Item Type: `LargeTestItem`

| Size  | hashbrown    | hop_hash     | Comparison                        |
| ----- | ------------ | ------------ | --------------------------------- |
| 1024  | 88.71 ns/op  | 97.36 ns/op  | **hashbrown** is **1.10x** faster |
| 2048  | 127.72 ns/op | 134.10 ns/op | **hashbrown** is **1.05x** faster |
| 4096  | 103.93 ns/op | 113.75 ns/op | **hashbrown** is **1.09x** faster |
| 8192  | 79.05 ns/op  | 90.43 ns/op  | **hashbrown** is **1.14x** faster |
| 16384 | 196.53 ns/op | 205.83 ns/op | **hashbrown** is **1.05x** faster |
| 32768 | 281.28 ns/op | 298.22 ns/op | **hashbrown** is **1.06x** faster |


### Benchmark: `remove` | Item Type: `TestItem`

| Size   | hashbrown    | hop_hash     | Comparison                        |
| ------ | ------------ | ------------ | --------------------------------- |
| 1024   | 25.62 ns/op  | 26.69 ns/op  | **hashbrown** is **1.04x** faster |
| 2048   | 26.45 ns/op  | 27.86 ns/op  | **hashbrown** is **1.05x** faster |
| 4096   | 27.36 ns/op  | 29.26 ns/op  | **hashbrown** is **1.07x** faster |
| 8192   | 30.16 ns/op  | 32.71 ns/op  | **hashbrown** is **1.08x** faster |
| 16384  | 36.01 ns/op  | 37.67 ns/op  | **hashbrown** is **1.05x** faster |
| 32768  | 39.66 ns/op  | 41.69 ns/op  | **hashbrown** is **1.05x** faster |
| 65536  | 83.56 ns/op  | 87.00 ns/op  | **hashbrown** is **1.04x** faster |
| 131072 | 163.32 ns/op | 175.57 ns/op | **hashbrown** is **1.07x** faster |
| 262144 | 204.16 ns/op | 222.00 ns/op | **hashbrown** is **1.09x** faster |