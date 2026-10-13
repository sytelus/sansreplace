# Results

The current native Cardchoose implementation and its linear-time bounds are described in
[cardchoose.md](cardchoose.md#linear-native-implementation). The focused before/after benchmark
is `python/bench_cardchoose`; see [running.md](running.md#cardchoose-regression-benchmark).

## Linear native implementation: 2026-10-13

Compared the original `6048513` with the optimized sampler in `624d745`, using the same
compiler settings for both revisions. The machine was an AMD EPYC 7763 Linux VM, pinned to
logical CPU 2, with 32 KiB L1 data cache, 512 KiB L2 and 32 MiB L3 reported to the guest.
C++ used GCC 14.2.0 and Rust used rustc 1.99.0. Builds used the repository defaults, with no
`-march=native`, `target-cpu=native`, AVX2 requirement, or CPU-specific intrinsics.

The run covered **261 (n, k) pairs**, including empty/tiny outputs, dispatch boundaries,
near-full, half-density and sparse samples, populations through `UINT32_MAX`, and outputs
through 16,777,216 elements (64 MiB, exceeding the reported last-level cache). Five repetitions
in both languages and both output orders produced **10,440 recorded measurements**. Revisions
alternated order and benchmarks ran serially. The timing budget/target was 0.02 seconds per case;
large calls naturally exceeded it. Output allocation is outside each timer, as in the original
harness. See the batch-size caveat below.

[Raw samples](results/cardchoose-2026-10-13/raw.csv),
[point-by-point medians and MADs](results/cardchoose-2026-10-13/summary.csv), and
[hardware/compiler/provenance metadata](results/cardchoose-2026-10-13/metadata.json).

The table gives the **median of pointwise speedups** (baseline time / current time). These are
suite summaries, not workload-weighted application speedups. A larger ratio is better.

| Cases | Points per order | C++ sorted | C++ random | Rust sorted | Rust random |
|---|---:|---:|---:|---:|---:|
| All nonempty cases | 254 | 3.84× | 2.80× | 2.29× | 2.29× |
| Small: 1 ≤ k ≤ 128 | 104 | 1.01× | 1.03× | 1.01× | 1.22× |
| Larger sparse: k ≥ 1024, n > 2k | 23 | 3.34× | 2.75× | 1.27× | 1.37× |
| Other dense: k ≥ 1024, n ≤ 2k | 35 | 10.61× | 4.89× | 4.44× | 2.39× |
| Full / small complement: k ≥ 1024 | 61 | 124.06× | 5.59× | 32.95× | 3.88× |

The dense row excludes the full/small-complement path. That path applies when `n-k <= 1024`
and `n-k <= k/4`, or when one element is omitted. The all-cases row also includes the intermediate
sizes not shown separately. Every point with `k >= 1024` had a median at least as fast as the
baseline; some Rust sorted sparse points were effectively ties within measurement variability.
At `(n, k) = (2,097,152, 1,048,576)`, sorted output improved from 76.91 ms to 13.62 ms in C++, and
from 21.97 ms to 11.19 ms in Rust. Random output improved from 80.96 ms to 16.55 ms in C++, and
from 28.93 ms to 13.65 ms in Rust.

### Small regressions and measurement limits

This implementation is not faster at every point. The worst nonempty relative regression in
each language/order was:

| Implementation | (n, k) | Before, ns/call | After, ns/call | Slower by |
|---|---|---:|---:|---:|
| cpp sorted | (8, 4) | 39.73 | 42.66 | 7.4% |
| cpp random | (80, 16) | 284.04 | 299.42 | 5.4% |
| rust sorted | (16, 8) | 91.40 | 96.76 | 5.9% |
| rust random | (4,294,967,295, 512) | 8572.41 | 8911.70 | 4.0% |

C++ empty calls also measured about 2.2 ns before and 2.6 ns after. Rust random output improved
at 253 of the 254 nonempty points; its one slower point had baseline/current MADs of about
0.10/0.24 microseconds. The CSVs retain all regressions and dispersion rather than discarding them.
Shared VM scheduling, clock behavior and allocator/cache state still affect timings despite
CPU affinity and repetition. These are measurements on this host, not proof of universal optimality.

The Rust timer's zeroed vector can be backed by lazily allocated pages, so a very fast large
sample can spend a substantial fraction of its first iteration faulting in output pages. A
separate [batch-budget check](results/cardchoose-2026-10-13/batch-budget-check.csv), pinned to the
same CPU with three repetitions, measured the current sorted Rust `(16,777,217, 16,777,216)` case
at 18.30 ms with a 0.02-second budget and 4.21 ms with a 0.2-second budget,
without changing the sampler. Longer batches amortize first-touch/cache warmup. Those diagnostic
samples are separate from the main table. Use the same budget when comparing revisions, and
compare within-language speedups rather than treating the C++ and Rust timers as identical.

### Changes and validation

The retained design combines in-place radix partitioning, fixed cache buffers, significant-bit
alignment, bounded small-array sorting, direct tiny samples, a fixed small-complement buffer,
and Algorithm S restricted to `n <= 2k`. Branchless urn copies did not produce a broad gain in
tuning. Pure in-place radix sorting regressed against Rust's fast standard sort in some regions;
bounded buffers and separate leaf cutoffs addressed those cases. Larger fixed buffers trade
constant stack space for fewer partitions, so the choices are explicit and bounded.

The native sampler performs O(k) work under the bounded-draw model, and expected O(k) time
including the existing C++ rejection-based RNG. It uses `4*k` output bytes plus O(1) workspace,
with no heap allocation in the sampler. See the [proof and workspace bounds](cardchoose.md#linear-native-implementation).

Validation passed: C++ exhaustive subset/permutation tests, forced dense-kernel enumeration,
adversarial radix cases, complement/dispatch/32-bit boundaries, canaries, ASan and UBSan; six
Rust tests in debug, release, and an explicit baseline x86-64 build with AVX/AVX2 disabled;
`cargo clippy --all-targets -- -D warnings`; and equality of the Python/C++ and Rust cache grids
through k = 1,048,576. The algorithms require no architecture intrinsics or unsafe Rust.

A follow-up review added exhaustive complement-kernel checks (seven Rust tests in total) and
validated the unbiased Rust RNG feature. It also fixed benchmark build isolation: both revisions
now use fresh build directories, and Cargo's reported executable path determines the binary
timed. An integration run with custom C++ flags, a custom Cargo target directory, and an explicit
target triple verified all 192 measurements across 24 cases without touching existing builds.
The production kernels are unchanged from `624d745`; all 10,440 saved measurements and their
1,044 summary rows were rechecked. The saved benchmark uses the original Rust RNG settings;
see the [uniformity assumptions](cardchoose.md#linear-native-implementation).

Reproduce the main run:

```
python/bench_cardchoose results/cardchoose-local --baseline 6048513 --repeats 5 --time-for 0.02 --max-k 16777216 --cpu 2
```

Use an available CPU index, or omit `--cpu` where affinity is unavailable. The benchmark rebuilds
both revisions with the same toolchains. See [running.md](running.md#cardchoose-regression-benchmark)
for controls and correctness checks.

## Historical comparison-sort results

These results were measured on an Apple M4 Mac mini with 16GB of memory, using clang 21.1 and
rustc 1.98, for values of _n_ and _k_ that are Fibonacci numbers, with _k_ at most 102,334,155
(the 40th Fibonacci number). [Full results](results/latest/)

Since both _n_ and _k_ affect the performance, we compare algorithms
using an animated graph. For each frame, we fix the value of _n_ and
graph the performance of each algorithm for various values of _k_;
we graph time/_k_ to show the cost in ns per number we output. Since
we are plotting a linear/log graph of time/_k_, the _k_ log _k_
performance of the algorithm is visible as a straight line.

### C++

The C++ implementations use the standard library's `std::unordered_set` and `std::unordered_map`,
so these results show the performance you get from a standard data structure.
`cardchoose` outperforms `rejectionsample`, `floydf2`, and `hsel` for all
values of _n_ and _k_. `quadraticreject` and `quadraticf2` are good where _k_ < 100 or so, and
`select`, `iterativechoose`, and `reservoirsample`
perform well when _k_/_n_ is large.

![graph](results/latest/cpp-random.webp)

![graph](results/latest/cpp-sorted.webp)

### Rust

The Rust port's hash-based algorithms use Rust's standard `HashMap` and `HashSet`. These are flat
open-addressing hash tables, much faster than the node-based C++ ones; the hash-based algorithms
take about 0.4–0.5× as long in Rust as in C++, while `cardchoose` takes about 0.6×.

In random order, `rejectionsample` beats `cardchoose` for _k_ between about 100 and
10<sup>6</sup> when _k_ is a small fraction of _n_. `cardchoose` is still faster around _k_ = 10,
around _k_ = 10<sup>7</sup>, where the hash table no longer fits in cache, and when _k_/_n_ is
large. `cardchoose` is never more than 1.38× slower than the fastest of `rejectionsample`,
`floydf2`, and `hsel`, while each of those is at least 13× slower than the fastest somewhere. In
sorted order, `cardchoose` outperforms `rejectionsample`, `floydf2`, and `hsel` for all
values of _n_ and _k_.

![graph](results/latest/rust-random.webp)

![graph](results/latest/rust-sorted.webp)

### Comparison

The table compares `cardchoose` with `rejectionsample`, `floydf2`, and `hsel` at the points
with _k_ ≥ 10, giving `cardchoose`'s time as a fraction of the fastest of the other three.

| | `cardchoose` fastest | median | range |
|---|---|---|---|
| C++, random order | 715 of 715 | 0.62× | 0.11–0.95× |
| C++, sorted order | 715 of 715 | 0.37× | 0.05–0.52× |
| Rust, random order | 369 of 715 | 1.00× | 0.08–1.38× |
| Rust, sorted order | 707 of 707 | 0.47× | 0.03–0.58× |

The next table gives each algorithm's worst case over the same points: the largest factor by
which it is slower than the fastest of the four.

| | `cardchoose` | `rejectionsample` | `floydf2` | `hsel` |
|---|---|---|---|---|
| C++, random order | 1.00× | 91× | 15× | 8.7× |
| C++, sorted order | 1.00× | 172× | 23× | 19× |
| Rust, random order | 1.38× | 100× | 13× | 13× |
| Rust, sorted order | 1.00× | 269× | 32× | 49× |

The worst cases for the hash-based algorithms are where _k_ = _n_. Leaving out the points where
_k_ ≥ _n_/2, they are still 2.5–6.4× slower than the fastest somewhere, mostly at large _k_, where
their hash tables no longer fit in cache.
