# Benchmark Results
## Key Takeaways
- Hop-hash performs well vs Hashbrown for mixed workloads while at a higher load factor (92%),
  sometimes outperforming it for large tables.
- Hop-hash significantly underperforms Hashbrown for single-operation workloads (get-only or insert-only).
- Drain performance is better than Hashbrown. Iteration performance is slightly better for large
  tables, and roughly equal for small tables.

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
  - Data is pre-generated for a benchmark and then used for all iterations of that benchmark. The
    initial data is pre-hashed before any insertion/find/etc. to try to exclude hashing time from
    the benchmarks as much as possible (some rehashing still occurs during table growth, but this
    seems like a fair thing to benchmark).
  - The benchmark suite does include large (280 bytes)/small (8 byte) value types, but those charts
    are not shown here for brevity. In general, the _relative_ performance of Hop-hash decreases for
    small value types, and increases for large value types.
- Run in Windows safe mode without networking to reduce background noise, pinned to a single CPU
  core, with realtime priority. A script was used to automate re-running benchmarks until the
  results had a run-to-run variance and total variance in one direction across 3 runs of under 5%.


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
| 1024  | 63 ns/op  | 57 ns/op  | **hop_hash** is **1.12x** faster |
| 2048  | 90 ns/op  | 81 ns/op  | **hop_hash** is **1.10x** faster |
| 4096  | 135 ns/op | 82 ns/op  | **hop_hash** is **1.64x** faster |
| 8192  | 138 ns/op | 84 ns/op  | **hop_hash** is **1.64x** faster |
| 16384 | 197 ns/op | 130 ns/op | **hop_hash** is **1.51x** faster |
| 32768 | 223 ns/op | 146 ns/op | **hop_hash** is **1.53x** faster |


### Benchmark: `churn` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 19 ns/op  | 24 ns/op  | **hashbrown** is **1.22x** faster |
| 2048   | 19 ns/op  | 24 ns/op  | **hashbrown** is **1.25x** faster |
| 4096   | 21 ns/op  | 24 ns/op  | **hashbrown** is **1.17x** faster |
| 8192   | 23 ns/op  | 25 ns/op  | **hashbrown** is **1.08x** faster |
| 16384  | 29 ns/op  | 27 ns/op  | **hop_hash** is **1.10x** faster  |
| 32768  | 39 ns/op  | 34 ns/op  | **hop_hash** is **1.14x** faster  |
| 65536  | 75 ns/op  | 65 ns/op  | **hop_hash** is **1.15x** faster  |
| 131072 | 107 ns/op | 89 ns/op  | **hop_hash** is **1.20x** faster  |
| 262144 | 137 ns/op | 118 ns/op | **hop_hash** is **1.16x** faster  |


### Benchmark: `collect_find` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 48 ns/op  | 71 ns/op  | **hashbrown** is **1.48x** faster |
| 2048  | 92 ns/op  | 97 ns/op  | **hashbrown** is **1.06x** faster |
| 4096  | 141 ns/op | 124 ns/op | **hop_hash** is **1.14x** faster  |
| 8192  | 151 ns/op | 134 ns/op | **hop_hash** is **1.13x** faster  |
| 16384 | 193 ns/op | 161 ns/op | **hop_hash** is **1.20x** faster  |
| 32768 | 218 ns/op | 181 ns/op | **hop_hash** is **1.21x** faster  |


### Benchmark: `collect_find` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 19 ns/op  | 36 ns/op  | **hashbrown** is **1.89x** faster |
| 2048   | 19 ns/op  | 30 ns/op  | **hashbrown** is **1.61x** faster |
| 4096   | 19 ns/op  | 28 ns/op  | **hashbrown** is **1.43x** faster |
| 8192   | 21 ns/op  | 29 ns/op  | **hashbrown** is **1.37x** faster |
| 16384  | 30 ns/op  | 33 ns/op  | **hashbrown** is **1.09x** faster |
| 32768  | 41 ns/op  | 41 ns/op  | **hashbrown** is **1.00x** faster |
| 65536  | 70 ns/op  | 70 ns/op  | **hashbrown** is **1.01x** faster |
| 131072 | 99 ns/op  | 106 ns/op | **hashbrown** is **1.06x** faster |
| 262144 | 130 ns/op | 142 ns/op | **hashbrown** is **1.09x** faster |


### Benchmark: `collect_find_preallocated` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 44 ns/op  | 46 ns/op  | **hashbrown** is **1.05x** faster |
| 2048  | 92 ns/op  | 89 ns/op  | **hop_hash** is **1.03x** faster  |
| 4096  | 89 ns/op  | 104 ns/op | **hashbrown** is **1.17x** faster |
| 8192  | 87 ns/op  | 104 ns/op | **hashbrown** is **1.20x** faster |
| 16384 | 103 ns/op | 117 ns/op | **hashbrown** is **1.14x** faster |
| 32768 | 118 ns/op | 138 ns/op | **hashbrown** is **1.16x** faster |


### Benchmark: `collect_find_preallocated` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash | Comparison                        |
| ------ | --------- | -------- | --------------------------------- |
| 1024   | 11 ns/op  | 22 ns/op | **hashbrown** is **1.98x** faster |
| 2048   | 11 ns/op  | 16 ns/op | **hashbrown** is **1.47x** faster |
| 4096   | 12 ns/op  | 15 ns/op | **hashbrown** is **1.33x** faster |
| 8192   | 12 ns/op  | 16 ns/op | **hashbrown** is **1.34x** faster |
| 16384  | 24 ns/op  | 27 ns/op | **hashbrown** is **1.16x** faster |
| 32768  | 28 ns/op  | 35 ns/op | **hashbrown** is **1.24x** faster |
| 65536  | 40 ns/op  | 49 ns/op | **hashbrown** is **1.21x** faster |
| 131072 | 61 ns/op  | 76 ns/op | **hashbrown** is **1.24x** faster |
| 262144 | 79 ns/op  | 98 ns/op | **hashbrown** is **1.24x** faster |


### Benchmark: `drain` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash | Comparison                        |
| ----- | --------- | -------- | --------------------------------- |
| 1024  | 21 ns/op  | 19 ns/op | **hop_hash** is **1.08x** faster  |
| 2048  | 30 ns/op  | 22 ns/op | **hop_hash** is **1.33x** faster  |
| 4096  | 25 ns/op  | 30 ns/op | **hashbrown** is **1.19x** faster |
| 8192  | 25 ns/op  | 21 ns/op | **hop_hash** is **1.20x** faster  |
| 16384 | 49 ns/op  | 43 ns/op | **hop_hash** is **1.15x** faster  |
| 32768 | 86 ns/op  | 62 ns/op | **hop_hash** is **1.39x** faster  |


### Benchmark: `drain` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash | Comparison                       |
| ------ | --------- | -------- | -------------------------------- |
| 1024   | 13 ns/op  | 11 ns/op | **hop_hash** is **1.25x** faster |
| 2048   | 13 ns/op  | 11 ns/op | **hop_hash** is **1.22x** faster |
| 4096   | 14 ns/op  | 11 ns/op | **hop_hash** is **1.21x** faster |
| 8192   | 15 ns/op  | 12 ns/op | **hop_hash** is **1.21x** faster |
| 16384  | 17 ns/op  | 13 ns/op | **hop_hash** is **1.32x** faster |
| 32768  | 18 ns/op  | 13 ns/op | **hop_hash** is **1.42x** faster |
| 65536  | 22 ns/op  | 15 ns/op | **hop_hash** is **1.53x** faster |
| 131072 | 42 ns/op  | 32 ns/op | **hop_hash** is **1.32x** faster |
| 262144 | 77 ns/op  | 62 ns/op | **hop_hash** is **1.23x** faster |


### Benchmark: `iteration` | Item Type: `LargeTestItem`

| Size  | hashbrown  | hop_hash   | Comparison                       |
| ----- | ---------- | ---------- | -------------------------------- |
| 1024  | 1.36 ns/op | 0.41 ns/op | **hop_hash** is **3.28x** faster |
| 2048  | 1.36 ns/op | 0.42 ns/op | **hop_hash** is **3.23x** faster |
| 4096  | 1.36 ns/op | 0.45 ns/op | **hop_hash** is **3.06x** faster |
| 8192  | 1.36 ns/op | 0.44 ns/op | **hop_hash** is **3.11x** faster |
| 16384 | 1.37 ns/op | 0.45 ns/op | **hop_hash** is **3.05x** faster |
| 32768 | 1.37 ns/op | 0.46 ns/op | **hop_hash** is **2.99x** faster |


### Benchmark: `iteration` | Item Type: `TestItem`

| Size   | hashbrown  | hop_hash   | Comparison                        |
| ------ | ---------- | ---------- | --------------------------------- |
| 1024   | 0.42 ns/op | 0.42 ns/op | **hop_hash** is **1.01x** faster  |
| 2048   | 0.43 ns/op | 0.43 ns/op | **hop_hash** is **1.00x** faster  |
| 4096   | 0.43 ns/op | 0.44 ns/op | **hashbrown** is **1.01x** faster |
| 8192   | 0.44 ns/op | 0.43 ns/op | **hop_hash** is **1.02x** faster  |
| 16384  | 0.45 ns/op | 0.45 ns/op | **hashbrown** is **1.01x** faster |
| 32768  | 0.48 ns/op | 0.46 ns/op | **hop_hash** is **1.05x** faster  |
| 65536  | 0.54 ns/op | 0.48 ns/op | **hop_hash** is **1.13x** faster  |
| 131072 | 0.60 ns/op | 0.51 ns/op | **hop_hash** is **1.18x** faster  |
| 262144 | 0.65 ns/op | 0.53 ns/op | **hop_hash** is **1.22x** faster  |


### Benchmark: `mixed_probabilistic` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 436 ns/op | 418 ns/op | **hop_hash** is **1.04x** faster |
| 2048  | 442 ns/op | 411 ns/op | **hop_hash** is **1.08x** faster |
| 4096  | 451 ns/op | 414 ns/op | **hop_hash** is **1.09x** faster |
| 8192  | 463 ns/op | 426 ns/op | **hop_hash** is **1.09x** faster |
| 16384 | 491 ns/op | 445 ns/op | **hop_hash** is **1.10x** faster |
| 32768 | 529 ns/op | 456 ns/op | **hop_hash** is **1.16x** faster |


### Benchmark: `mixed_probabilistic` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 167 ns/op | 171 ns/op | **hashbrown** is **1.02x** faster |
| 2048   | 167 ns/op | 169 ns/op | **hashbrown** is **1.02x** faster |
| 4096   | 169 ns/op | 170 ns/op | **hashbrown** is **1.01x** faster |
| 8192   | 180 ns/op | 176 ns/op | **hop_hash** is **1.02x** faster  |
| 16384  | 179 ns/op | 179 ns/op | **hop_hash** is **1.00x** faster  |
| 32768  | 202 ns/op | 191 ns/op | **hop_hash** is **1.06x** faster  |
| 65536  | 217 ns/op | 207 ns/op | **hop_hash** is **1.05x** faster  |
| 131072 | 245 ns/op | 242 ns/op | **hop_hash** is **1.01x** faster  |
| 262144 | 276 ns/op | 272 ns/op | **hop_hash** is **1.01x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.0` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 430 ns/op | 401 ns/op | **hop_hash** is **1.07x** faster |
| 2048  | 439 ns/op | 403 ns/op | **hop_hash** is **1.09x** faster |
| 4096  | 452 ns/op | 406 ns/op | **hop_hash** is **1.11x** faster |
| 8192  | 460 ns/op | 416 ns/op | **hop_hash** is **1.11x** faster |
| 16384 | 485 ns/op | 430 ns/op | **hop_hash** is **1.13x** faster |
| 32768 | 510 ns/op | 443 ns/op | **hop_hash** is **1.15x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.0` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 168 ns/op | 171 ns/op | **hashbrown** is **1.02x** faster |
| 2048   | 170 ns/op | 174 ns/op | **hashbrown** is **1.03x** faster |
| 4096   | 172 ns/op | 173 ns/op | **hashbrown** is **1.00x** faster |
| 8192   | 179 ns/op | 180 ns/op | **hashbrown** is **1.00x** faster |
| 16384  | 183 ns/op | 177 ns/op | **hop_hash** is **1.03x** faster  |
| 32768  | 197 ns/op | 190 ns/op | **hop_hash** is **1.03x** faster  |
| 65536  | 195 ns/op | 188 ns/op | **hop_hash** is **1.04x** faster  |
| 131072 | 239 ns/op | 219 ns/op | **hop_hash** is **1.09x** faster  |
| 262144 | 269 ns/op | 259 ns/op | **hop_hash** is **1.04x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.3` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 427 ns/op | 405 ns/op | **hop_hash** is **1.05x** faster |
| 2048  | 446 ns/op | 400 ns/op | **hop_hash** is **1.12x** faster |
| 4096  | 445 ns/op | 399 ns/op | **hop_hash** is **1.11x** faster |
| 8192  | 462 ns/op | 412 ns/op | **hop_hash** is **1.12x** faster |
| 16384 | 480 ns/op | 425 ns/op | **hop_hash** is **1.13x** faster |
| 32768 | 505 ns/op | 435 ns/op | **hop_hash** is **1.16x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.3` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 168 ns/op | 173 ns/op | **hashbrown** is **1.03x** faster |
| 2048   | 167 ns/op | 168 ns/op | **hashbrown** is **1.01x** faster |
| 4096   | 170 ns/op | 169 ns/op | **hop_hash** is **1.01x** faster  |
| 8192   | 178 ns/op | 173 ns/op | **hop_hash** is **1.03x** faster  |
| 16384  | 181 ns/op | 174 ns/op | **hop_hash** is **1.04x** faster  |
| 32768  | 184 ns/op | 186 ns/op | **hashbrown** is **1.01x** faster |
| 65536  | 194 ns/op | 190 ns/op | **hop_hash** is **1.02x** faster  |
| 131072 | 224 ns/op | 216 ns/op | **hop_hash** is **1.04x** faster  |
| 262144 | 250 ns/op | 247 ns/op | **hop_hash** is **1.01x** faster  |


### Benchmark: `mixed_probabilistic_zipf_1.8` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 428 ns/op | 402 ns/op | **hop_hash** is **1.07x** faster |
| 2048  | 447 ns/op | 395 ns/op | **hop_hash** is **1.13x** faster |
| 4096  | 446 ns/op | 401 ns/op | **hop_hash** is **1.11x** faster |
| 8192  | 463 ns/op | 413 ns/op | **hop_hash** is **1.12x** faster |
| 16384 | 478 ns/op | 416 ns/op | **hop_hash** is **1.15x** faster |
| 32768 | 502 ns/op | 435 ns/op | **hop_hash** is **1.15x** faster |


### Benchmark: `mixed_probabilistic_zipf_1.8` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 164 ns/op | 170 ns/op | **hashbrown** is **1.04x** faster |
| 2048   | 164 ns/op | 166 ns/op | **hashbrown** is **1.01x** faster |
| 4096   | 165 ns/op | 167 ns/op | **hashbrown** is **1.01x** faster |
| 8192   | 173 ns/op | 169 ns/op | **hop_hash** is **1.02x** faster  |
| 16384  | 176 ns/op | 175 ns/op | **hop_hash** is **1.00x** faster  |
| 32768  | 178 ns/op | 176 ns/op | **hop_hash** is **1.01x** faster  |
| 65536  | 193 ns/op | 186 ns/op | **hop_hash** is **1.03x** faster  |
| 131072 | 232 ns/op | 217 ns/op | **hop_hash** is **1.07x** faster  |
| 262144 | 251 ns/op | 246 ns/op | **hop_hash** is **1.02x** faster  |


### Benchmark: `mixed_workload` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                       |
| ----- | --------- | --------- | -------------------------------- |
| 1024  | 166 ns/op | 114 ns/op | **hop_hash** is **1.46x** faster |
| 2048  | 231 ns/op | 129 ns/op | **hop_hash** is **1.80x** faster |
| 4096  | 267 ns/op | 152 ns/op | **hop_hash** is **1.75x** faster |
| 8192  | 368 ns/op | 228 ns/op | **hop_hash** is **1.62x** faster |
| 16384 | 442 ns/op | 265 ns/op | **hop_hash** is **1.66x** faster |
| 32768 | 495 ns/op | 312 ns/op | **hop_hash** is **1.58x** faster |


### Benchmark: `mixed_workload` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 42 ns/op  | 63 ns/op  | **hashbrown** is **1.50x** faster |
| 2048   | 43 ns/op  | 53 ns/op  | **hashbrown** is **1.23x** faster |
| 4096   | 47 ns/op  | 53 ns/op  | **hashbrown** is **1.14x** faster |
| 8192   | 59 ns/op  | 52 ns/op  | **hop_hash** is **1.12x** faster  |
| 16384  | 71 ns/op  | 57 ns/op  | **hop_hash** is **1.24x** faster  |
| 32768  | 97 ns/op  | 72 ns/op  | **hop_hash** is **1.35x** faster  |
| 65536  | 160 ns/op | 118 ns/op | **hop_hash** is **1.36x** faster  |
| 131072 | 232 ns/op | 180 ns/op | **hop_hash** is **1.29x** faster  |
| 262144 | 288 ns/op | 240 ns/op | **hop_hash** is **1.20x** faster  |


### Benchmark: `find_hit` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 26 ns/op  | 36 ns/op  | **hashbrown** is **1.41x** faster |
| 2048  | 32 ns/op  | 57 ns/op  | **hashbrown** is **1.79x** faster |
| 4096  | 55 ns/op  | 67 ns/op  | **hashbrown** is **1.23x** faster |
| 8192  | 61 ns/op  | 74 ns/op  | **hashbrown** is **1.21x** faster |
| 16384 | 91 ns/op  | 101 ns/op | **hashbrown** is **1.12x** faster |
| 32768 | 122 ns/op | 138 ns/op | **hashbrown** is **1.13x** faster |


### Benchmark: `find_hit` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 13 ns/op  | 23 ns/op  | **hashbrown** is **1.80x** faster |
| 2048   | 13 ns/op  | 18 ns/op  | **hashbrown** is **1.34x** faster |
| 4096   | 14 ns/op  | 18 ns/op  | **hashbrown** is **1.23x** faster |
| 8192   | 17 ns/op  | 26 ns/op  | **hashbrown** is **1.50x** faster |
| 16384  | 22 ns/op  | 24 ns/op  | **hashbrown** is **1.09x** faster |
| 32768  | 38 ns/op  | 30 ns/op  | **hop_hash** is **1.27x** faster  |
| 65536  | 56 ns/op  | 68 ns/op  | **hashbrown** is **1.21x** faster |
| 131072 | 96 ns/op  | 111 ns/op | **hashbrown** is **1.16x** faster |
| 262144 | 124 ns/op | 137 ns/op | **hashbrown** is **1.11x** faster |


### Benchmark: `find_hit_miss` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 29 ns/op  | 36 ns/op  | **hashbrown** is **1.24x** faster |
| 2048  | 36 ns/op  | 59 ns/op  | **hashbrown** is **1.65x** faster |
| 4096  | 55 ns/op  | 60 ns/op  | **hashbrown** is **1.10x** faster |
| 8192  | 68 ns/op  | 63 ns/op  | **hop_hash** is **1.07x** faster  |
| 16384 | 61 ns/op  | 75 ns/op  | **hashbrown** is **1.24x** faster |
| 32768 | 102 ns/op | 113 ns/op | **hashbrown** is **1.12x** faster |


### Benchmark: `find_hit_miss` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 18 ns/op  | 26 ns/op  | **hashbrown** is **1.48x** faster |
| 2048   | 19 ns/op  | 22 ns/op  | **hashbrown** is **1.16x** faster |
| 4096   | 19 ns/op  | 22 ns/op  | **hashbrown** is **1.14x** faster |
| 8192   | 21 ns/op  | 24 ns/op  | **hashbrown** is **1.14x** faster |
| 16384  | 25 ns/op  | 26 ns/op  | **hashbrown** is **1.05x** faster |
| 32768  | 40 ns/op  | 31 ns/op  | **hop_hash** is **1.30x** faster  |
| 65536  | 43 ns/op  | 59 ns/op  | **hashbrown** is **1.37x** faster |
| 131072 | 77 ns/op  | 96 ns/op  | **hashbrown** is **1.26x** faster |
| 262144 | 99 ns/op  | 120 ns/op | **hashbrown** is **1.22x** faster |


### Benchmark: `find_miss` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash | Comparison                        |
| ----- | --------- | -------- | --------------------------------- |
| 1024  | 32 ns/op  | 37 ns/op | **hashbrown** is **1.17x** faster |
| 2048  | 58 ns/op  | 64 ns/op | **hashbrown** is **1.11x** faster |
| 4096  | 62 ns/op  | 65 ns/op | **hashbrown** is **1.05x** faster |
| 8192  | 60 ns/op  | 69 ns/op | **hashbrown** is **1.16x** faster |
| 16384 | 65 ns/op  | 74 ns/op | **hashbrown** is **1.14x** faster |
| 32768 | 88 ns/op  | 99 ns/op | **hashbrown** is **1.13x** faster |


### Benchmark: `find_miss` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash | Comparison                        |
| ------ | --------- | -------- | --------------------------------- |
| 1024   | 19 ns/op  | 26 ns/op | **hashbrown** is **1.41x** faster |
| 2048   | 22 ns/op  | 25 ns/op | **hashbrown** is **1.14x** faster |
| 4096   | 22 ns/op  | 23 ns/op | **hashbrown** is **1.05x** faster |
| 8192   | 21 ns/op  | 23 ns/op | **hashbrown** is **1.09x** faster |
| 16384  | 23 ns/op  | 24 ns/op | **hashbrown** is **1.06x** faster |
| 32768  | 29 ns/op  | 30 ns/op | **hashbrown** is **1.03x** faster |
| 65536  | 41 ns/op  | 51 ns/op | **hashbrown** is **1.26x** faster |
| 131072 | 59 ns/op  | 73 ns/op | **hashbrown** is **1.24x** faster |
| 262144 | 83 ns/op  | 93 ns/op | **hashbrown** is **1.12x** faster |


### Benchmark: `insert_random` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 66 ns/op  | 103 ns/op | **hashbrown** is **1.56x** faster |
| 2048  | 158 ns/op | 151 ns/op | **hop_hash** is **1.05x** faster  |
| 4096  | 196 ns/op | 157 ns/op | **hop_hash** is **1.25x** faster  |
| 8192  | 227 ns/op | 167 ns/op | **hop_hash** is **1.36x** faster  |
| 16384 | 284 ns/op | 215 ns/op | **hop_hash** is **1.32x** faster  |
| 32768 | 325 ns/op | 248 ns/op | **hop_hash** is **1.31x** faster  |


### Benchmark: `insert_random` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 22 ns/op  | 43 ns/op  | **hashbrown** is **1.95x** faster |
| 2048   | 22 ns/op  | 41 ns/op  | **hashbrown** is **1.88x** faster |
| 4096   | 22 ns/op  | 37 ns/op  | **hashbrown** is **1.68x** faster |
| 8192   | 23 ns/op  | 36 ns/op  | **hashbrown** is **1.58x** faster |
| 16384  | 36 ns/op  | 42 ns/op  | **hashbrown** is **1.18x** faster |
| 32768  | 45 ns/op  | 43 ns/op  | **hop_hash** is **1.03x** faster  |
| 65536  | 59 ns/op  | 57 ns/op  | **hop_hash** is **1.04x** faster  |
| 131072 | 103 ns/op | 98 ns/op  | **hop_hash** is **1.05x** faster  |
| 262144 | 140 ns/op | 135 ns/op | **hop_hash** is **1.04x** faster  |


### Benchmark: `insert_random_preallocated` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 38 ns/op  | 176 ns/op | **hashbrown** is **4.61x** faster |
| 2048  | 122 ns/op | 195 ns/op | **hashbrown** is **1.59x** faster |
| 4096  | 122 ns/op | 188 ns/op | **hashbrown** is **1.54x** faster |
| 8192  | 125 ns/op | 194 ns/op | **hashbrown** is **1.55x** faster |
| 16384 | 125 ns/op | 217 ns/op | **hashbrown** is **1.73x** faster |
| 32768 | 136 ns/op | 218 ns/op | **hashbrown** is **1.60x** faster |


### Benchmark: `insert_random_preallocated` | Item Type: `TestItem`

| Size   | hashbrown  | hop_hash   | Comparison                        |
| ------ | ---------- | ---------- | --------------------------------- |
| 1024   | 6.01 ns/op | 10 ns/op   | **hashbrown** is **1.71x** faster |
| 2048   | 5.35 ns/op | 9.16 ns/op | **hashbrown** is **1.71x** faster |
| 4096   | 5.37 ns/op | 10 ns/op   | **hashbrown** is **1.93x** faster |
| 8192   | 6.61 ns/op | 42 ns/op   | **hashbrown** is **6.33x** faster |
| 16384  | 28 ns/op   | 41 ns/op   | **hashbrown** is **1.48x** faster |
| 32768  | 28 ns/op   | 38 ns/op   | **hashbrown** is **1.33x** faster |
| 65536  | 29 ns/op   | 40 ns/op   | **hashbrown** is **1.40x** faster |
| 131072 | 34 ns/op   | 50 ns/op   | **hashbrown** is **1.49x** faster |
| 262144 | 36 ns/op   | 55 ns/op   | **hashbrown** is **1.51x** faster |


### Benchmark: `remove` | Item Type: `LargeTestItem`

| Size  | hashbrown | hop_hash  | Comparison                        |
| ----- | --------- | --------- | --------------------------------- |
| 1024  | 93 ns/op  | 110 ns/op | **hashbrown** is **1.18x** faster |
| 2048  | 124 ns/op | 147 ns/op | **hashbrown** is **1.19x** faster |
| 4096  | 138 ns/op | 142 ns/op | **hashbrown** is **1.03x** faster |
| 8192  | 116 ns/op | 121 ns/op | **hashbrown** is **1.05x** faster |
| 16384 | 209 ns/op | 203 ns/op | **hop_hash** is **1.03x** faster  |
| 32768 | 296 ns/op | 292 ns/op | **hop_hash** is **1.02x** faster  |


### Benchmark: `remove` | Item Type: `TestItem`

| Size   | hashbrown | hop_hash  | Comparison                        |
| ------ | --------- | --------- | --------------------------------- |
| 1024   | 24 ns/op  | 32 ns/op  | **hashbrown** is **1.34x** faster |
| 2048   | 25 ns/op  | 32 ns/op  | **hashbrown** is **1.30x** faster |
| 4096   | 28 ns/op  | 31 ns/op  | **hashbrown** is **1.13x** faster |
| 8192   | 32 ns/op  | 34 ns/op  | **hashbrown** is **1.09x** faster |
| 16384  | 37 ns/op  | 40 ns/op  | **hashbrown** is **1.07x** faster |
| 32768  | 49 ns/op  | 51 ns/op  | **hashbrown** is **1.03x** faster |
| 65536  | 106 ns/op | 104 ns/op | **hop_hash** is **1.01x** faster  |
| 131072 | 168 ns/op | 173 ns/op | **hashbrown** is **1.03x** faster |
|        |