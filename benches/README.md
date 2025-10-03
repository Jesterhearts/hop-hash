# Benchmark Results
## Key Takeaways
- Hop-hash performs well vs Hashbrown for mixed workloads sometimes outperforming it for large
  tables.
- Hop-hash underperforms Hashbrown for read-only workloads.

## Individual Result Graphs

In all cases, Hashbrown is represented with the red line, and Hop-hash is represented with the green line.

- CPU: AMD Ryzen AI 9 HX 370
- RAM: 32GB
- OS: Windows 11
- Rust Version: 1.89.0 (29483883e 2025-08-04)
- Default release profile
- Default features
- SipHash hasher
- Hashbrown version 0.16.0
- Value-type (32 bytes) is a String of length 20, generated arbitrarily, plus a u64. The String is
  used as the key for hashing and comparisons.
  - Input data is pre-hashed before any insertion/find/etc. to try to exclude hashing time from
    the benchmarks as much as possible (some rehashing still occurs during table growth, but this
    seems like a fair thing to benchmark).
  - The benchmark suite does include large (280 bytes)/small (8 byte) value types, but those charts
    are not shown here for brevity. In general, the _relative_ performance of Hop-hash decreases for
    small value types, and increases for large value types.
- Run in Windows safe mode without networking to reduce background noise, pinned to a single CPU
  core, with realtime priority. A script was used to automate re-running benchmarks until the
  results had a run-to-run variance under 5% and total variance in one direction across 3 runs of
  under 5%.


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
and removals. A batch of items is generated and a table filled to its target capacity and load
factor. Then, in each iteration, the benchmark randomly selects items from the batch using a zipf
distribution with s=1.3 in a loop of size equal to the table capacity. For each selected item, it
has a 50/50 chance of either inserting/updating the item (if it is not already present) or removing
the item (if it is present). This simulates a workload where the table is constantly being updated
with new items while also removing old items, maintaining a steady state size.

![churn workload benchmark results](images/churn.png)

### Single Operation Workloads
#### Iteration
The following benchmark results show the performance of hop-hash vs hashbrown for iterating over
all items in the table with a cold cache.

![iteration benchmark results](images/iteration.png)

#### Drain
The following benchmark results show the performance of hop-hash vs hashbrown for draining all
items from the table with a cold cache.

![drain benchmark results](images/drain.png)

## Selected Result Tables

Values are based on the median point estimate recorded by Criterion.

Keep in mind while reviewing these results that benchmarks on my machine can vary by 5% between
runs.

### Benchmark: `churn` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 239 ns/op | 142 ns/op | **hop_hash** is **1.69x** faster |
| 2048  | 274 ns/op | 153 ns/op | **hop_hash** is **1.80x** faster |
| 4096  | 292 ns/op | 158 ns/op | **hop_hash** is **1.84x** faster |
| 8192  | 323 ns/op | 176 ns/op | **hop_hash** is **1.83x** faster |
| 16384 | 388 ns/op | 192 ns/op | **hop_hash** is **2.03x** faster |
| 32768 | 439 ns/op | 233 ns/op | **hop_hash** is **1.89x** faster |


### Benchmark: `churn` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 73 ns/op  | 75 ns/op  | **hashbrown** is **1.02x** faster |
| 2048   | 73 ns/op  | 75 ns/op  | **hashbrown** is **1.02x** faster |
| 4096   | 75 ns/op  | 76 ns/op  | **hashbrown** is **1.01x** faster |
| 8192   | 98 ns/op  | 82 ns/op  | **hop_hash** is **1.19x** faster  |
| 16384  | 101 ns/op | 85 ns/op  | **hop_hash** is **1.19x** faster  |
| 32768  | 116 ns/op | 97 ns/op  | **hop_hash** is **1.20x** faster  |
| 65536  | 143 ns/op | 66 ns/op  | **hop_hash** is **2.18x** faster  |
| 131072 | 210 ns/op | 143 ns/op | **hop_hash** is **1.47x** faster  |
| 262144 | 257 ns/op | 190 ns/op | **hop_hash** is **1.35x** faster  |


### Benchmark: `collect_find` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 48 ns/op  | 53 ns/op  | **hashbrown** is **1.10x** faster |
| 2048  | 92 ns/op  | 88 ns/op  | **hop_hash** is **1.04x** faster  |
| 4096  | 141 ns/op | 124 ns/op | **hop_hash** is **1.14x** faster  |
| 8192  | 151 ns/op | 139 ns/op | **hop_hash** is **1.09x** faster  |
| 16384 | 193 ns/op | 162 ns/op | **hop_hash** is **1.19x** faster  |
| 32768 | 218 ns/op | 181 ns/op | **hop_hash** is **1.21x** faster  |


### Benchmark: `collect_find` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 19 ns/op  | 24 ns/op  | **hashbrown** is **1.25x** faster |
| 2048   | 19 ns/op  | 24 ns/op  | **hashbrown** is **1.28x** faster |
| 4096   | 19 ns/op  | 24 ns/op  | **hashbrown** is **1.22x** faster |
| 8192   | 21 ns/op  | 25 ns/op  | **hashbrown** is **1.22x** faster |
| 16384  | 30 ns/op  | 32 ns/op  | **hashbrown** is **1.06x** faster |
| 32768  | 41 ns/op  | 42 ns/op  | **hashbrown** is **1.02x** faster |
| 65536  | 70 ns/op  | 71 ns/op  | **hashbrown** is **1.02x** faster |
| 131072 | 99 ns/op  | 106 ns/op | **hashbrown** is **1.06x** faster |
| 262144 | 130 ns/op | 138 ns/op | **hashbrown** is **1.06x** faster |


### Benchmark: `collect_find_preallocated` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 44 ns/op  | 44 ns/op  | **hashbrown** is **1.01x** faster |
| 2048  | 92 ns/op  | 97 ns/op  | **hashbrown** is **1.06x** faster |
| 4096  | 89 ns/op  | 99 ns/op  | **hashbrown** is **1.12x** faster |
| 8192  | 87 ns/op  | 94 ns/op  | **hashbrown** is **1.08x** faster |
| 16384 | 103 ns/op | 113 ns/op | **hashbrown** is **1.10x** faster |
| 32768 | 118 ns/op | 130 ns/op | **hashbrown** is **1.10x** faster |


### Benchmark: `collect_find_preallocated` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash | Comparison                        |
| ------ | --------- | -------- | --------------------------------- |
| 1024   | 11 ns/op  | 12 ns/op | **hashbrown** is **1.11x** faster |
| 2048   | 11 ns/op  | 13 ns/op | **hashbrown** is **1.20x** faster |
| 4096   | 12 ns/op  | 14 ns/op | **hashbrown** is **1.18x** faster |
| 8192   | 12 ns/op  | 15 ns/op | **hashbrown** is **1.24x** faster |
| 16384  | 24 ns/op  | 26 ns/op | **hashbrown** is **1.12x** faster |
| 32768  | 28 ns/op  | 31 ns/op | **hashbrown** is **1.09x** faster |
| 65536  | 40 ns/op  | 46 ns/op | **hashbrown** is **1.14x** faster |
| 131072 | 61 ns/op  | 71 ns/op | **hashbrown** is **1.17x** faster |
| 262144 | 79 ns/op  | 93 ns/op | **hashbrown** is **1.18x** faster |


### Benchmark: `drain` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash | Comparison                       |
| ----- | --------- | -------- | -------------------------------- |
| 1024  | 22 ns/op  | 17 ns/op | **hop_hash** is **1.33x** faster |
| 2048  | 33 ns/op  | 20 ns/op | **hop_hash** is **1.60x** faster |
| 4096  | 25 ns/op  | 17 ns/op | **hop_hash** is **1.49x** faster |
| 8192  | 24 ns/op  | 20 ns/op | **hop_hash** is **1.23x** faster |
| 16384 | 39 ns/op  | 33 ns/op | **hop_hash** is **1.20x** faster |
| 32768 | 82 ns/op  | 55 ns/op | **hop_hash** is **1.48x** faster |


### Benchmark: `drain` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash    | Comparison                       |
| ------ | --------- | ----------- | -------------------------------- |
| 1024   | 13 ns/op  | 10.63 ns/op | **hop_hash** is **1.25x** faster |
| 2048   | 13 ns/op  | 10.93 ns/op | **hop_hash** is **1.21x** faster |
| 4096   | 13 ns/op  | 11.02 ns/op | **hop_hash** is **1.22x** faster |
| 8192   | 15 ns/op  | 11.36 ns/op | **hop_hash** is **1.31x** faster |
| 16384  | 17 ns/op  | 12 ns/op    | **hop_hash** is **1.40x** faster |
| 32768  | 19 ns/op  | 13 ns/op    | **hop_hash** is **1.45x** faster |
| 65536  | 21 ns/op  | 15 ns/op    | **hop_hash** is **1.40x** faster |
| 131072 | 39 ns/op  | 24 ns/op    | **hop_hash** is **1.61x** faster |
| 262144 | 83 ns/op  | 76 ns/op    | **hop_hash** is **1.09x** faster |


### Benchmark: `iteration_cold` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash | Comparison                        |
| ----- | --------- | -------- | --------------------------------- |
| 1024  | 13 ns/op  | 13 ns/op | **hashbrown** is **1.03x** faster |
| 2048  | 28 ns/op  | 20 ns/op | **hop_hash** is **1.41x** faster  |
| 4096  | 29 ns/op  | 18 ns/op | **hop_hash** is **1.58x** faster  |
| 8192  | 26 ns/op  | 19 ns/op | **hop_hash** is **1.42x** faster  |
| 16384 | 38 ns/op  | 22 ns/op | **hop_hash** is **1.69x** faster  |
| 32768 | 62 ns/op  | 51 ns/op | **hop_hash** is **1.21x** faster  |


### Benchmark: `iteration_cold` | Item Type: `TestItem`

| Size   | hashbrown   | hop_hash    | Comparison                       |
| ------ | ----------- | ----------- | -------------------------------- |
| 1024   | 10.63 ns/op | 9.80 ns/op  | **hop_hash** is **1.09x** faster |
| 2048   | 10.83 ns/op | 9.92 ns/op  | **hop_hash** is **1.09x** faster |
| 4096   | 11.08 ns/op | 10.17 ns/op | **hop_hash** is **1.09x** faster |
| 8192   | 12 ns/op    | 10.64 ns/op | **hop_hash** is **1.15x** faster |
| 16384  | 15 ns/op    | 12 ns/op    | **hop_hash** is **1.20x** faster |
| 32768  | 18 ns/op    | 13 ns/op    | **hop_hash** is **1.35x** faster |
| 65536  | 21 ns/op    | 15 ns/op    | **hop_hash** is **1.42x** faster |
| 131072 | 38 ns/op    | 29 ns/op    | **hop_hash** is **1.28x** faster |
| 262144 | 77 ns/op    | 51 ns/op    | **hop_hash** is **1.50x** faster |


### Benchmark: `mixed_probabilistic` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 436 ns/op | 399 ns/op | **hop_hash** is **1.09x** faster |
| 2048  | 442 ns/op | 411 ns/op | **hop_hash** is **1.08x** faster |
| 4096  | 451 ns/op | 409 ns/op | **hop_hash** is **1.10x** faster |
| 8192  | 463 ns/op | 419 ns/op | **hop_hash** is **1.11x** faster |
| 16384 | 491 ns/op | 436 ns/op | **hop_hash** is **1.13x** faster |
| 32768 | 529 ns/op | 464 ns/op | **hop_hash** is **1.14x** faster |


### Benchmark: `mixed_probabilistic` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 167 ns/op | 168 ns/op | **hashbrown** is **1.01x** faster |
| 2048   | 167 ns/op | 170 ns/op | **hashbrown** is **1.02x** faster |
| 4096   | 169 ns/op | 170 ns/op | **hashbrown** is **1.01x** faster |
| 8192   | 180 ns/op | 171 ns/op | **hop_hash** is **1.05x** faster  |
| 16384  | 179 ns/op | 174 ns/op | **hop_hash** is **1.03x** faster  |
| 32768  | 202 ns/op | 177 ns/op | **hop_hash** is **1.14x** faster  |
| 65536  | 217 ns/op | 187 ns/op | **hop_hash** is **1.16x** faster  |
| 131072 | 245 ns/op | 209 ns/op | **hop_hash** is **1.17x** faster  |
| 262144 | 276 ns/op | 246 ns/op | **hop_hash** is **1.12x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.0` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 430 ns/op | 413 ns/op | **hop_hash** is **1.04x** faster |
| 2048  | 439 ns/op | 422 ns/op | **hop_hash** is **1.04x** faster |
| 4096  | 452 ns/op | 418 ns/op | **hop_hash** is **1.08x** faster |
| 8192  | 460 ns/op | 426 ns/op | **hop_hash** is **1.08x** faster |
| 16384 | 485 ns/op | 444 ns/op | **hop_hash** is **1.09x** faster |
| 32768 | 510 ns/op | 455 ns/op | **hop_hash** is **1.12x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.0` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                       |
| ------ | --------- | --------- | -------------------------------- |
| 1024   | 168 ns/op | 165 ns/op | **hop_hash** is **1.02x** faster |
| 2048   | 170 ns/op | 169 ns/op | **hop_hash** is **1.01x** faster |
| 4096   | 172 ns/op | 170 ns/op | **hop_hash** is **1.01x** faster |
| 8192   | 179 ns/op | 172 ns/op | **hop_hash** is **1.04x** faster |
| 16384  | 183 ns/op | 173 ns/op | **hop_hash** is **1.05x** faster |
| 32768  | 197 ns/op | 176 ns/op | **hop_hash** is **1.12x** faster |
| 65536  | 195 ns/op | 191 ns/op | **hop_hash** is **1.02x** faster |
| 131072 | 239 ns/op | 214 ns/op | **hop_hash** is **1.11x** faster |
| 262144 | 269 ns/op | 245 ns/op | **hop_hash** is **1.10x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.3` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 427 ns/op | 409 ns/op | **hop_hash** is **1.04x** faster |
| 2048  | 446 ns/op | 414 ns/op | **hop_hash** is **1.08x** faster |
| 4096  | 445 ns/op | 415 ns/op | **hop_hash** is **1.07x** faster |
| 8192  | 462 ns/op | 423 ns/op | **hop_hash** is **1.09x** faster |
| 16384 | 480 ns/op | 442 ns/op | **hop_hash** is **1.09x** faster |
| 32768 | 505 ns/op | 458 ns/op | **hop_hash** is **1.10x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.3` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 168 ns/op | 165 ns/op | **hop_hash** is **1.02x** faster  |
| 2048   | 167 ns/op | 171 ns/op | **hashbrown** is **1.03x** faster |
| 4096   | 170 ns/op | 169 ns/op | **hop_hash** is **1.01x** faster  |
| 8192   | 178 ns/op | 171 ns/op | **hop_hash** is **1.04x** faster  |
| 16384  | 181 ns/op | 173 ns/op | **hop_hash** is **1.05x** faster  |
| 32768  | 184 ns/op | 178 ns/op | **hop_hash** is **1.04x** faster  |
| 65536  | 194 ns/op | 187 ns/op | **hop_hash** is **1.04x** faster  |
| 131072 | 224 ns/op | 212 ns/op | **hop_hash** is **1.06x** faster  |
| 262144 | 250 ns/op | 245 ns/op | **hop_hash** is **1.02x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.8` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 428 ns/op | 408 ns/op | **hop_hash** is **1.05x** faster |
| 2048  | 447 ns/op | 415 ns/op | **hop_hash** is **1.08x** faster |
| 4096  | 446 ns/op | 418 ns/op | **hop_hash** is **1.07x** faster |
| 8192  | 463 ns/op | 425 ns/op | **hop_hash** is **1.09x** faster |
| 16384 | 478 ns/op | 427 ns/op | **hop_hash** is **1.12x** faster |
| 32768 | 502 ns/op | 458 ns/op | **hop_hash** is **1.10x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.8` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 164 ns/op | 165 ns/op | **hashbrown** is **1.01x** faster |
| 2048   | 164 ns/op | 164 ns/op | **hop_hash** is **1.00x** faster  |
| 4096   | 165 ns/op | 166 ns/op | **hashbrown** is **1.00x** faster |
| 8192   | 173 ns/op | 173 ns/op | **hop_hash** is **1.00x** faster  |
| 16384  | 176 ns/op | 172 ns/op | **hop_hash** is **1.02x** faster  |
| 32768  | 178 ns/op | 174 ns/op | **hop_hash** is **1.02x** faster  |
| 65536  | 193 ns/op | 185 ns/op | **hop_hash** is **1.04x** faster  |
| 131072 | 232 ns/op | 215 ns/op | **hop_hash** is **1.07x** faster  |
| 262144 | 251 ns/op | 244 ns/op | **hop_hash** is **1.03x** faster  |


### Benchmark: `mixed_workload` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 166 ns/op | 84 ns/op  | **hop_hash** is **1.99x** faster |
| 2048  | 231 ns/op | 123 ns/op | **hop_hash** is **1.88x** faster |
| 4096  | 267 ns/op | 152 ns/op | **hop_hash** is **1.75x** faster |
| 8192  | 368 ns/op | 209 ns/op | **hop_hash** is **1.77x** faster |
| 16384 | 442 ns/op | 260 ns/op | **hop_hash** is **1.70x** faster |
| 32768 | 495 ns/op | 308 ns/op | **hop_hash** is **1.60x** faster |


### Benchmark: `mixed_workload` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 42 ns/op  | 43 ns/op  | **hashbrown** is **1.02x** faster |
| 2048   | 43 ns/op  | 43 ns/op  | **hop_hash** is **1.01x** faster  |
| 4096   | 47 ns/op  | 44 ns/op  | **hop_hash** is **1.06x** faster  |
| 8192   | 59 ns/op  | 46 ns/op  | **hop_hash** is **1.28x** faster  |
| 16384  | 71 ns/op  | 53 ns/op  | **hop_hash** is **1.35x** faster  |
| 32768  | 97 ns/op  | 70 ns/op  | **hop_hash** is **1.40x** faster  |
| 65536  | 160 ns/op | 118 ns/op | **hop_hash** is **1.36x** faster  |
| 131072 | 232 ns/op | 173 ns/op | **hop_hash** is **1.34x** faster  |
| 262144 | 288 ns/op | 230 ns/op | **hop_hash** is **1.26x** faster  |
