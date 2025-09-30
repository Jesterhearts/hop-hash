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
- Run in Windows safe mode without networking to reduce background noise, pinned to a single CPU
  core, with realtime priority.


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

Values are based on the median point estimate recorded by Criterion.

Keep in mind while reviewing these results that benchmarks on my machine can vary by 5% between
runs. Also keep in mind that hop-hash is running at a load factor of 92%, while hashbrown is running
at a load factor of 87.5%.

### Benchmark: `churn` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 92 ns/op  | 78 ns/op  | **hop_hash** is **1.18x** faster |
| 2048  | 124 ns/op | 83 ns/op  | **hop_hash** is **1.49x** faster |
| 4096  | 155 ns/op | 89 ns/op  | **hop_hash** is **1.74x** faster |
| 8192  | 169 ns/op | 96 ns/op  | **hop_hash** is **1.75x** faster |
| 16384 | 200 ns/op | 131 ns/op | **hop_hash** is **1.53x** faster |
| 32768 | 222 ns/op | 151 ns/op | **hop_hash** is **1.48x** faster |


### Benchmark: `churn` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 20 ns/op  | 23 ns/op  | **hashbrown** is **1.18x** faster |
| 2048   | 20 ns/op  | 24 ns/op  | **hashbrown** is **1.21x** faster |
| 4096   | 21 ns/op  | 24 ns/op  | **hashbrown** is **1.18x** faster |
| 8192   | 23 ns/op  | 25 ns/op  | **hashbrown** is **1.05x** faster |
| 16384  | 30 ns/op  | 27 ns/op  | **hop_hash** is **1.13x** faster  |
| 32768  | 41 ns/op  | 33 ns/op  | **hop_hash** is **1.23x** faster  |
| 65536  | 70 ns/op  | 57 ns/op  | **hop_hash** is **1.21x** faster  |
| 131072 | 106 ns/op | 89 ns/op  | **hop_hash** is **1.19x** faster  |
| 262144 | 136 ns/op | 115 ns/op | **hop_hash** is **1.18x** faster  |


### Benchmark: `collect_find` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 67 ns/op  | 85 ns/op  | **hashbrown** is **1.28x** faster |
| 2048  | 125 ns/op | 113 ns/op | **hop_hash** is **1.11x** faster  |
| 4096  | 156 ns/op | 140 ns/op | **hop_hash** is **1.12x** faster  |
| 8192  | 170 ns/op | 149 ns/op | **hop_hash** is **1.14x** faster  |
| 16384 | 193 ns/op | 165 ns/op | **hop_hash** is **1.17x** faster  |
| 32768 | 217 ns/op | 185 ns/op | **hop_hash** is **1.18x** faster  |


### Benchmark: `collect_find` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 19 ns/op  | 36 ns/op  | **hashbrown** is **1.84x** faster |
| 2048   | 19 ns/op  | 30 ns/op  | **hashbrown** is **1.53x** faster |
| 4096   | 20 ns/op  | 28 ns/op  | **hashbrown** is **1.36x** faster |
| 8192   | 21 ns/op  | 28 ns/op  | **hashbrown** is **1.33x** faster |
| 16384  | 35 ns/op  | 34 ns/op  | **hop_hash** is **1.02x** faster  |
| 32768  | 44 ns/op  | 41 ns/op  | **hop_hash** is **1.08x** faster  |
| 65536  | 71 ns/op  | 70 ns/op  | **hop_hash** is **1.02x** faster  |
| 131072 | 101 ns/op | 101 ns/op | **hashbrown** is **1.01x** faster |
| 262144 | 131 ns/op | 136 ns/op | **hashbrown** is **1.03x** faster |


### Benchmark: `collect_find_preallocated` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 44 ns/op  | 65 ns/op  | **hashbrown** is **1.48x** faster |
| 2048  | 94 ns/op  | 108 ns/op | **hashbrown** is **1.14x** faster |
| 4096  | 94 ns/op  | 109 ns/op | **hashbrown** is **1.17x** faster |
| 8192  | 94 ns/op  | 106 ns/op | **hashbrown** is **1.13x** faster |
| 16384 | 105 ns/op | 118 ns/op | **hashbrown** is **1.12x** faster |
| 32768 | 119 ns/op | 134 ns/op | **hashbrown** is **1.13x** faster |


### Benchmark: `collect_find_preallocated` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash | Comparison                        |
| ------ | --------- | -------- | --------------------------------- |
| 1024   | 11 ns/op  | 22 ns/op | **hashbrown** is **1.98x** faster |
| 2048   | 11 ns/op  | 15 ns/op | **hashbrown** is **1.42x** faster |
| 4096   | 11 ns/op  | 16 ns/op | **hashbrown** is **1.36x** faster |
| 8192   | 13 ns/op  | 16 ns/op | **hashbrown** is **1.22x** faster |
| 16384  | 24 ns/op  | 27 ns/op | **hashbrown** is **1.13x** faster |
| 32768  | 28 ns/op  | 31 ns/op | **hashbrown** is **1.10x** faster |
| 65536  | 42 ns/op  | 47 ns/op | **hashbrown** is **1.12x** faster |
| 131072 | 60 ns/op  | 69 ns/op | **hashbrown** is **1.15x** faster |
| 262144 | 79 ns/op  | 91 ns/op | **hashbrown** is **1.15x** faster |


### Benchmark: `drain` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash | Comparison                        |
| ----- | --------- | -------- | --------------------------------- |
| 1024  | 31 ns/op  | 19 ns/op | **hop_hash** is **1.62x** faster  |
| 2048  | 26 ns/op  | 22 ns/op | **hop_hash** is **1.22x** faster  |
| 4096  | 24 ns/op  | 30 ns/op | **hashbrown** is **1.24x** faster |
| 8192  | 24 ns/op  | 21 ns/op | **hop_hash** is **1.15x** faster  |
| 16384 | 44 ns/op  | 38 ns/op | **hop_hash** is **1.18x** faster  |
| 32768 | 84 ns/op  | 61 ns/op | **hop_hash** is **1.38x** faster  |


### Benchmark: `drain` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash | Comparison                       |
| ------ | --------- | -------- | -------------------------------- |
| 1024   | 13 ns/op  | 11 ns/op | **hop_hash** is **1.23x** faster |
| 2048   | 13 ns/op  | 11 ns/op | **hop_hash** is **1.22x** faster |
| 4096   | 14 ns/op  | 11 ns/op | **hop_hash** is **1.24x** faster |
| 8192   | 14 ns/op  | 12 ns/op | **hop_hash** is **1.20x** faster |
| 16384  | 16 ns/op  | 12 ns/op | **hop_hash** is **1.31x** faster |
| 32768  | 18 ns/op  | 13 ns/op | **hop_hash** is **1.38x** faster |
| 65536  | 21 ns/op  | 15 ns/op | **hop_hash** is **1.45x** faster |
| 131072 | 39 ns/op  | 33 ns/op | **hop_hash** is **1.20x** faster |
| 262144 | 76 ns/op  | 68 ns/op | **hop_hash** is **1.11x** faster |


### Benchmark: `iteration` | Item Type: `LargeTestItem`

| Size  | hashbrown  | hop_hash   | Comparison                       |
| ----- | ---------- | ---------- | -------------------------------- |
| 1024  | 1.36 ns/op | 0.39 ns/op | **hop_hash** is **3.47x** faster |
| 2048  | 1.36 ns/op | 0.45 ns/op | **hop_hash** is **3.00x** faster |
| 4096  | 1.36 ns/op | 0.50 ns/op | **hop_hash** is **2.74x** faster |
| 8192  | 1.36 ns/op | 0.54 ns/op | **hop_hash** is **2.53x** faster |
| 16384 | 1.36 ns/op | 0.56 ns/op | **hop_hash** is **2.45x** faster |
| 32768 | 1.39 ns/op | 0.55 ns/op | **hop_hash** is **2.53x** faster |


### Benchmark: `iteration` | Item Type: `TestItem`

| Size   | hashbrown  | hop_hash   | Comparison                       |
| ------ | ---------- | ---------- | -------------------------------- |
| 1024   | 0.42 ns/op | 0.34 ns/op | **hop_hash** is **1.26x** faster |
| 2048   | 0.43 ns/op | 0.35 ns/op | **hop_hash** is **1.23x** faster |
| 4096   | 0.43 ns/op | 0.35 ns/op | **hop_hash** is **1.22x** faster |
| 8192   | 0.45 ns/op | 0.35 ns/op | **hop_hash** is **1.26x** faster |
| 16384  | 0.44 ns/op | 0.37 ns/op | **hop_hash** is **1.20x** faster |
| 32768  | 0.47 ns/op | 0.37 ns/op | **hop_hash** is **1.25x** faster |
| 65536  | 0.53 ns/op | 0.39 ns/op | **hop_hash** is **1.37x** faster |
| 131072 | 0.61 ns/op | 0.42 ns/op | **hop_hash** is **1.45x** faster |
| 262144 | 0.65 ns/op | 0.45 ns/op | **hop_hash** is **1.45x** faster |


### Benchmark: `mixed_probabilistic` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 430 ns/op | 401 ns/op | **hop_hash** is **1.07x** faster |
| 2048  | 433 ns/op | 407 ns/op | **hop_hash** is **1.06x** faster |
| 4096  | 442 ns/op | 409 ns/op | **hop_hash** is **1.08x** faster |
| 8192  | 451 ns/op | 420 ns/op | **hop_hash** is **1.07x** faster |
| 16384 | 471 ns/op | 430 ns/op | **hop_hash** is **1.09x** faster |
| 32768 | 500 ns/op | 447 ns/op | **hop_hash** is **1.12x** faster |


### Benchmark: `mixed_probabilistic` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 163 ns/op | 170 ns/op | **hashbrown** is **1.04x** faster |
| 2048   | 164 ns/op | 168 ns/op | **hashbrown** is **1.02x** faster |
| 4096   | 164 ns/op | 170 ns/op | **hashbrown** is **1.03x** faster |
| 8192   | 170 ns/op | 170 ns/op | **hashbrown** is **1.00x** faster |
| 16384  | 172 ns/op | 171 ns/op | **hop_hash** is **1.01x** faster  |
| 32768  | 173 ns/op | 173 ns/op | **hashbrown** is **1.00x** faster |
| 65536  | 183 ns/op | 181 ns/op | **hop_hash** is **1.01x** faster  |
| 131072 | 208 ns/op | 202 ns/op | **hop_hash** is **1.03x** faster  |
| 262144 | 241 ns/op | 235 ns/op | **hop_hash** is **1.02x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.0` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 431 ns/op | 401 ns/op | **hop_hash** is **1.07x** faster |
| 2048  | 436 ns/op | 410 ns/op | **hop_hash** is **1.06x** faster |
| 4096  | 443 ns/op | 413 ns/op | **hop_hash** is **1.07x** faster |
| 8192  | 458 ns/op | 419 ns/op | **hop_hash** is **1.09x** faster |
| 16384 | 478 ns/op | 428 ns/op | **hop_hash** is **1.12x** faster |
| 32768 | 501 ns/op | 449 ns/op | **hop_hash** is **1.12x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.0` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 163 ns/op | 168 ns/op | **hashbrown** is **1.03x** faster |
| 2048   | 165 ns/op | 165 ns/op | **hashbrown** is **1.00x** faster |
| 4096   | 164 ns/op | 167 ns/op | **hashbrown** is **1.02x** faster |
| 8192   | 171 ns/op | 171 ns/op | **hashbrown** is **1.00x** faster |
| 16384  | 173 ns/op | 172 ns/op | **hop_hash** is **1.01x** faster  |
| 32768  | 175 ns/op | 174 ns/op | **hop_hash** is **1.01x** faster  |
| 65536  | 185 ns/op | 181 ns/op | **hop_hash** is **1.02x** faster  |
| 131072 | 214 ns/op | 212 ns/op | **hop_hash** is **1.01x** faster  |
| 262144 | 242 ns/op | 239 ns/op | **hop_hash** is **1.01x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.3` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 432 ns/op | 402 ns/op | **hop_hash** is **1.07x** faster |
| 2048  | 437 ns/op | 410 ns/op | **hop_hash** is **1.07x** faster |
| 4096  | 443 ns/op | 412 ns/op | **hop_hash** is **1.07x** faster |
| 8192  | 456 ns/op | 415 ns/op | **hop_hash** is **1.10x** faster |
| 16384 | 479 ns/op | 432 ns/op | **hop_hash** is **1.11x** faster |
| 32768 | 500 ns/op | 452 ns/op | **hop_hash** is **1.10x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.3` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 162 ns/op | 168 ns/op | **hashbrown** is **1.03x** faster |
| 2048   | 162 ns/op | 166 ns/op | **hashbrown** is **1.02x** faster |
| 4096   | 163 ns/op | 165 ns/op | **hashbrown** is **1.01x** faster |
| 8192   | 170 ns/op | 169 ns/op | **hop_hash** is **1.00x** faster  |
| 16384  | 174 ns/op | 172 ns/op | **hop_hash** is **1.02x** faster  |
| 32768  | 174 ns/op | 173 ns/op | **hop_hash** is **1.01x** faster  |
| 65536  | 184 ns/op | 180 ns/op | **hop_hash** is **1.02x** faster  |
| 131072 | 212 ns/op | 206 ns/op | **hop_hash** is **1.03x** faster  |
| 262144 | 243 ns/op | 239 ns/op | **hop_hash** is **1.02x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.8` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 433 ns/op | 401 ns/op | **hop_hash** is **1.08x** faster |
| 2048  | 435 ns/op | 409 ns/op | **hop_hash** is **1.06x** faster |
| 4096  | 442 ns/op | 410 ns/op | **hop_hash** is **1.08x** faster |
| 8192  | 456 ns/op | 415 ns/op | **hop_hash** is **1.10x** faster |
| 16384 | 475 ns/op | 430 ns/op | **hop_hash** is **1.10x** faster |
| 32768 | 498 ns/op | 446 ns/op | **hop_hash** is **1.12x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.8` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 162 ns/op | 165 ns/op | **hashbrown** is **1.02x** faster |
| 2048   | 162 ns/op | 164 ns/op | **hashbrown** is **1.01x** faster |
| 4096   | 162 ns/op | 164 ns/op | **hashbrown** is **1.01x** faster |
| 8192   | 170 ns/op | 167 ns/op | **hop_hash** is **1.01x** faster  |
| 16384  | 171 ns/op | 169 ns/op | **hop_hash** is **1.01x** faster  |
| 32768  | 173 ns/op | 171 ns/op | **hop_hash** is **1.02x** faster  |
| 65536  | 183 ns/op | 179 ns/op | **hop_hash** is **1.02x** faster  |
| 131072 | 211 ns/op | 208 ns/op | **hop_hash** is **1.01x** faster  |
| 262144 | 243 ns/op | 238 ns/op | **hop_hash** is **1.02x** faster  |


### Benchmark: `mixed_workload` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 164 ns/op | 113 ns/op | **hop_hash** is **1.46x** faster |
| 2048  | 244 ns/op | 153 ns/op | **hop_hash** is **1.59x** faster |
| 4096  | 287 ns/op | 160 ns/op | **hop_hash** is **1.79x** faster |
| 8192  | 365 ns/op | 210 ns/op | **hop_hash** is **1.74x** faster |
| 16384 | 436 ns/op | 260 ns/op | **hop_hash** is **1.68x** faster |
| 32768 | 491 ns/op | 310 ns/op | **hop_hash** is **1.58x** faster |


### Benchmark: `mixed_workload` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 41 ns/op  | 61 ns/op  | **hashbrown** is **1.47x** faster |
| 2048   | 42 ns/op  | 51 ns/op  | **hashbrown** is **1.23x** faster |
| 4096   | 46 ns/op  | 50 ns/op  | **hashbrown** is **1.09x** faster |
| 8192   | 59 ns/op  | 51 ns/op  | **hop_hash** is **1.16x** faster  |
| 16384  | 71 ns/op  | 57 ns/op  | **hop_hash** is **1.25x** faster  |
| 32768  | 95 ns/op  | 71 ns/op  | **hop_hash** is **1.35x** faster  |
| 65536  | 160 ns/op | 124 ns/op | **hop_hash** is **1.30x** faster  |
| 131072 | 226 ns/op | 177 ns/op | **hop_hash** is **1.28x** faster  |
| 262144 | 283 ns/op | 232 ns/op | **hop_hash** is **1.22x** faster  |


### Benchmark: `find_hit` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 38 ns/op  | 39 ns/op  | **hashbrown** is **1.02x** faster |
| 2048  | 56 ns/op  | 56 ns/op  | **hashbrown** is **1.01x** faster |
| 4096  | 60 ns/op  | 63 ns/op  | **hashbrown** is **1.05x** faster |
| 8192  | 81 ns/op  | 73 ns/op  | **hop_hash** is **1.11x** faster  |
| 16384 | 93 ns/op  | 95 ns/op  | **hashbrown** is **1.03x** faster |
| 32768 | 120 ns/op | 125 ns/op | **hashbrown** is **1.04x** faster |


### Benchmark: `find_hit` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 17 ns/op  | 22 ns/op  | **hashbrown** is **1.28x** faster |
| 2048   | 17 ns/op  | 17 ns/op  | **hashbrown** is **1.06x** faster |
| 4096   | 19 ns/op  | 19 ns/op  | **hashbrown** is **1.04x** faster |
| 8192   | 26 ns/op  | 24 ns/op  | **hop_hash** is **1.08x** faster  |
| 16384  | 24 ns/op  | 25 ns/op  | **hashbrown** is **1.04x** faster |
| 32768  | 41 ns/op  | 46 ns/op  | **hashbrown** is **1.11x** faster |
| 65536  | 58 ns/op  | 66 ns/op  | **hashbrown** is **1.14x** faster |
| 131072 | 96 ns/op  | 111 ns/op | **hashbrown** is **1.15x** faster |
| 262144 | 124 ns/op | 134 ns/op | **hashbrown** is **1.08x** faster |


### Benchmark: `find_hit_miss` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 29 ns/op  | 36 ns/op  | **hashbrown** is **1.22x** faster |
| 2048  | 36 ns/op  | 56 ns/op  | **hashbrown** is **1.57x** faster |
| 4096  | 52 ns/op  | 61 ns/op  | **hashbrown** is **1.16x** faster |
| 8192  | 66 ns/op  | 80 ns/op  | **hashbrown** is **1.20x** faster |
| 16384 | 75 ns/op  | 85 ns/op  | **hashbrown** is **1.13x** faster |
| 32768 | 96 ns/op  | 107 ns/op | **hashbrown** is **1.12x** faster |


### Benchmark: `find_hit_miss` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 19 ns/op  | 27 ns/op  | **hashbrown** is **1.42x** faster |
| 2048   | 20 ns/op  | 23 ns/op  | **hashbrown** is **1.12x** faster |
| 4096   | 23 ns/op  | 22 ns/op  | **hop_hash** is **1.02x** faster  |
| 8192   | 22 ns/op  | 23 ns/op  | **hashbrown** is **1.07x** faster |
| 16384  | 27 ns/op  | 27 ns/op  | **hashbrown** is **1.04x** faster |
| 32768  | 30 ns/op  | 30 ns/op  | **hashbrown** is **1.00x** faster |
| 65536  | 48 ns/op  | 59 ns/op  | **hashbrown** is **1.22x** faster |
| 131072 | 77 ns/op  | 95 ns/op  | **hashbrown** is **1.23x** faster |
| 262144 | 104 ns/op | 119 ns/op | **hashbrown** is **1.15x** faster |


### Benchmark: `find_miss` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash | Comparison                        |
| ----- | --------- | -------- | --------------------------------- |
| 1024  | 39 ns/op  | 34 ns/op | **hop_hash** is **1.14x** faster  |
| 2048  | 57 ns/op  | 61 ns/op | **hashbrown** is **1.08x** faster |
| 4096  | 60 ns/op  | 65 ns/op | **hashbrown** is **1.09x** faster |
| 8192  | 59 ns/op  | 57 ns/op | **hop_hash** is **1.03x** faster  |
| 16384 | 63 ns/op  | 72 ns/op | **hashbrown** is **1.13x** faster |
| 32768 | 87 ns/op  | 96 ns/op | **hashbrown** is **1.11x** faster |


### Benchmark: `find_miss` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash | Comparison                        |
| ------ | --------- | -------- | --------------------------------- |
| 1024   | 19 ns/op  | 26 ns/op | **hashbrown** is **1.38x** faster |
| 2048   | 22 ns/op  | 24 ns/op | **hashbrown** is **1.11x** faster |
| 4096   | 22 ns/op  | 22 ns/op | **hashbrown** is **1.01x** faster |
| 8192   | 22 ns/op  | 23 ns/op | **hashbrown** is **1.05x** faster |
| 16384  | 24 ns/op  | 25 ns/op | **hashbrown** is **1.03x** faster |
| 32768  | 42 ns/op  | 30 ns/op | **hop_hash** is **1.42x** faster  |
| 65536  | 38 ns/op  | 47 ns/op | **hashbrown** is **1.22x** faster |
| 131072 | 58 ns/op  | 70 ns/op | **hashbrown** is **1.21x** faster |
| 262144 | 80 ns/op  | 89 ns/op | **hashbrown** is **1.11x** faster |


### Benchmark: `insert_random` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 83 ns/op  | 108 ns/op | **hashbrown** is **1.31x** faster |
| 2048  | 171 ns/op | 174 ns/op | **hashbrown** is **1.02x** faster |
| 4096  | 225 ns/op | 192 ns/op | **hop_hash** is **1.17x** faster  |
| 8192  | 253 ns/op | 199 ns/op | **hop_hash** is **1.27x** faster  |
| 16384 | 296 ns/op | 226 ns/op | **hop_hash** is **1.31x** faster  |
| 32768 | 336 ns/op | 259 ns/op | **hop_hash** is **1.30x** faster  |


### Benchmark: `insert_random` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 23 ns/op  | 47 ns/op  | **hashbrown** is **2.08x** faster |
| 2048   | 22 ns/op  | 40 ns/op  | **hashbrown** is **1.82x** faster |
| 4096   | 22 ns/op  | 37 ns/op  | **hashbrown** is **1.63x** faster |
| 8192   | 25 ns/op  | 36 ns/op  | **hashbrown** is **1.45x** faster |
| 16384  | 38 ns/op  | 44 ns/op  | **hashbrown** is **1.16x** faster |
| 32768  | 46 ns/op  | 49 ns/op  | **hashbrown** is **1.06x** faster |
| 65536  | 60 ns/op  | 55 ns/op  | **hop_hash** is **1.09x** faster  |
| 131072 | 95 ns/op  | 96 ns/op  | **hashbrown** is **1.00x** faster |
| 262144 | 139 ns/op | 138 ns/op | **hop_hash** is **1.01x** faster  |


### Benchmark: `insert_random_preallocated` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 44 ns/op  | 181 ns/op | **hashbrown** is **4.07x** faster |
| 2048  | 125 ns/op | 199 ns/op | **hashbrown** is **1.59x** faster |
| 4096  | 125 ns/op | 202 ns/op | **hashbrown** is **1.62x** faster |
| 8192  | 127 ns/op | 203 ns/op | **hashbrown** is **1.59x** faster |
| 16384 | 126 ns/op | 208 ns/op | **hashbrown** is **1.65x** faster |
| 32768 | 132 ns/op | 209 ns/op | **hashbrown** is **1.59x** faster |


### Benchmark: `insert_random_preallocated` | Item Type: `TestItem`

| Size   | hashbrown  | hop_hash   | Comparison                        |
| ------ | ---------- | ---------- | --------------------------------- |
| 1024   | 6.01 ns/op | 9.08 ns/op | **hashbrown** is **1.51x** faster |
| 2048   | 5.44 ns/op | 9.66 ns/op | **hashbrown** is **1.78x** faster |
| 4096   | 5.32 ns/op | 10 ns/op   | **hashbrown** is **1.90x** faster |
| 8192   | 7.30 ns/op | 42 ns/op   | **hashbrown** is **5.81x** faster |
| 16384  | 28 ns/op   | 43 ns/op   | **hashbrown** is **1.55x** faster |
| 32768  | 28 ns/op   | 38 ns/op   | **hashbrown** is **1.34x** faster |
| 65536  | 28 ns/op   | 39 ns/op   | **hashbrown** is **1.39x** faster |
| 131072 | 33 ns/op   | 46 ns/op   | **hashbrown** is **1.38x** faster |
| 262144 | 36 ns/op   | 51 ns/op   | **hashbrown** is **1.42x** faster |


### Benchmark: `remove` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 91 ns/op  | 111 ns/op | **hashbrown** is **1.22x** faster |
| 2048  | 128 ns/op | 151 ns/op | **hashbrown** is **1.18x** faster |
| 4096  | 134 ns/op | 136 ns/op | **hashbrown** is **1.02x** faster |
| 8192  | 113 ns/op | 122 ns/op | **hashbrown** is **1.08x** faster |
| 16384 | 213 ns/op | 215 ns/op | **hashbrown** is **1.01x** faster |
| 32768 | 292 ns/op | 291 ns/op | **hop_hash** is **1.00x** faster  |


### Benchmark: `remove` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 26 ns/op  | 32 ns/op  | **hashbrown** is **1.25x** faster |
| 2048   | 26 ns/op  | 29 ns/op  | **hashbrown** is **1.10x** faster |
| 4096   | 27 ns/op  | 30 ns/op  | **hashbrown** is **1.08x** faster |
| 8192   | 31 ns/op  | 33 ns/op  | **hashbrown** is **1.07x** faster |
| 16384  | 36 ns/op  | 38 ns/op  | **hashbrown** is **1.04x** faster |
| 32768  | 39 ns/op  | 43 ns/op  | **hashbrown** is **1.09x** faster |
| 65536  | 80 ns/op  | 84 ns/op  | **hashbrown** is **1.06x** faster |
| 131072 | 159 ns/op | 172 ns/op | **hashbrown** is **1.09x** faster |
| 262144 | 202 ns/op | 217 ns/op | **hashbrown** is **1.07x** faster |
