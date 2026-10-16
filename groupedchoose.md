# Grouped Cardchoose

Grouped Cardchoose removes the algorithm's dependence on a fixed number of integer
digits. It samples a uniform sorted subset using **expected O(k) integer operations,
the k-element output array, and O(1) auxiliary integers**. It uses no hash table,
bitmap, recursion, floating point, or second array. Its small local sorts operate on
contiguous memory; the temporary count table occupies only a prefix of the output.

This is an expected bound, not a worst-case linear sorting guarantee. It counts
integer arithmetic, including division, and a bounded uniform draw as constant-time
operations. Arbitrary-precision arithmetic has additional bit costs. Merely writing
k integers of up to log2(n) bits can require Ω(k log n) bit operations and bits of
storage. No algorithm returning those explicit integers can remove that cost.

The [Python implementation](python/algorithms/groupedchoose.py) accepts arbitrarily
large integer populations. [C++](cpp/groupedchoose.h) and
[Rust](rust/src/algorithms/groupedchoose.rs) provide both 32- and 64-bit entry points.
The native APIs require n to fit their population type; Python is the executable
reference for populations beyond 64 bits. The existing 32-bit `cardchoose` remains
available as a separate, more specialized implementation and benchmark competitor.

## Construction

For a sparse sample, let t = n − k + 1. The original Cardchoose generates a
uniform multiset of k colors in [0,t), sorts it, then adds index i to color i.
The result is a uniform k-subset of [0,n), by the stars-and-bars bijection.

Instead of sorting all k colors together, partition [0,t) into B = ceil(k/16)
consecutive groups. If `width, extra = divmod(t, B)`, the first `extra` groups
have `width+1` colors and the rest have `width` colors. A group's first color is
`b*width + min(b, extra)`. Mapping a color to its group uses one division and a
branch; every intermediate fits in the same word as n. There is no
double-width multiplication requirement and no dependence on a particular ISA.

1. Run the original Pólya urn for k draws, storing **group labels** instead of
   exact colors. An original color ticket maps to its group; a previously drawn
   ticket copies its saved label.
2. Turn those labels into group counts, in place.
3. For each nonempty group, independently run a local urn for that group's count.
   Insertion-sort these local colors and add their **global** output indices.
   Visiting the groups in order gives the complete sorted sample.

The target group size 16 is a tuning constant, not a required cache-line size.
Any fixed value at least 3 supports the storage construction and expected bound.
Typical local working sets are small even when the full output is much larger
than the CPU caches. No cache size, AVX2 instruction, or `-march=native` setting
is assumed.

## Why the distribution is exactly uniform

Let group b contain s_b original colors and receive M_b draws. The aggregated
urn is a Pólya urn with initial group weights s_b. For counts m_b summing to k,

\[
  \Pr(M_b=m_b\ \text{for all }b)
  =\frac{\prod_b {s_b+m_b-1\choose m_b}}{{t+k-1\choose k}}.
\]

The local urn produces each local multiset with probability
`1 / binomial(s_b + m_b - 1, m_b)`. Conditional on the counts, the local draws
are independent. Multiplication cancels every numerator factor, leaving
`1 / binomial(n,k)` for each global multiset, and therefore for each subset.
This argument includes repeated colors and correlated group occupancies; it
does not assume that the urn draws are independent uniform keys.

Uniformity here assumes independent uniform bounded draws. Python uses
`random.randrange`; C++ uses rejection-based range reduction for both widths;
Rust now enables `rand/unbiased` by default for **all** algorithms. The native
64-bit C++ reducer accepts only complete intervals of equal size from a raw
64-bit draw, so all residues have the same number of accepted preimages. It
needs neither a 128-bit type nor floating point. These are exact range reducers
over the respective PRNGs, not a claim that a finite-state PRNG supplies an
infinite independent random source. Seeded sequences differ across algorithms,
word widths, languages, and versions.

## Expected linear work

The exact second moment of the group occupancies is

\[
 \mathbb E\!\left[\sum_b M_b^2\right]
 = k + k(k-1)\frac{\sum_b s_b(s_b+1)}{t(t+1)}.
\]

Balanced group sizes give

\[
 \frac{\sum_b s_b(s_b+1)}{t(t+1)}\le\frac1B+\frac2t.
\]

The sparse path is used only when n > 2k, hence t > k. Since B ≥ k/16,
the expected sum of squared occupancies is at most **19k**. Insertion-sorting
a group of m elements costs O(m²), so the expected total sorting work is O(k).
This also covers the one-group case, where k ≤ 16.
The native implementations also skip grouping for at most 64 outputs; insertion
sorting that bounded range costs at most O(64k), still O(k). The compact Python
reference uses the group construction directly.

Generating labels, counting, packing records, and writing output take O(k)
work. There are at most 2k bounded draws in the sparse sorted path and fewer
than 3k after adding a Fisher–Yates shuffle. Unbiased range reduction takes
expected constant raw draws per bounded draw. Rare occupancies can still make
insertion sorting quadratic; no worst-case linear bound is claimed here.

Writing the output already costs Ω(k) word operations, so the expected bound
is asymptotically optimal in this arithmetic model. This establishes neither
the smallest instruction count nor the fastest implementation on every CPU.

## How one array is enough

The output passes through four representations:

| Phase | Contents |
|---|---|
| Labels | One-based labels 1..B in all k slots |
| Histogram | First B slots contain zero or `B + count`; the rest are zero |
| Packed records | Nonempty-group records packed against the right edge |
| Result | Sorted sample written from the left |

While histogramming, a slot holding a label is moved to its destination. If that
destination still holds a label, its label is displaced into the current slot
before a counter is installed. Otherwise its counter is incremented. Each
original label is consumed exactly once; at most B counter installations do
not advance the scan. Thus this phase is linear without an auxiliary table.

A singleton group's record is its freshly sampled color, one word less than t.
A larger group stores two words: `t + (count - 2)` and its group index. The tag
is at least t and at most n−1. Histogram counters are at most B+k < n on the
sparse path. No sign bit or extra high bit is reserved.

Packing proceeds from the last group backwards. Since 2B ≤ k and each record
uses at most two slots, writing records cannot overwrite an unread counter.
Decoding proceeds forwards. Each record uses no more slots than the output it
describes, so the remaining records fit in the remaining output space. After
reading a record, that entire group's output ends at or before the next unread
record. Local urn generation and insertion sorting therefore cannot corrupt
metadata, even for an unusually large group.

All stored temporary values fit in [0,n). Scalar expressions are also arranged
to avoid overflow: for example `t + (count - 2)`, rather than `(t + count) - 2`.
Empty samples return before computing n−k+1, which would overflow at the native
maximum population. Tests also instantiate the C++ kernel with 8-bit words
across every valid (n,k) pair to detect hidden use of spare bits.

## Dense and nearly full samples

For sorted samples with n ≤ 2k, Algorithm S selects the next value with
probability `remaining selections / remaining population`. It scans at most 2k
values. Full samples and a single omission have direct fill paths. Native k=1
and k=2 paths avoid the group machinery, and small sorted samples through k=64
use the original urn and a bounded insertion sort when another edge path does
not apply. The general dense scan avoids extra exit checks on every element.

When q = n−k is small, a complement path reduces random draws by sampling q omissions into the output
prefix using the sparse kernel. It then converts gaps between omissions into
the same packed records. A run's urn color is the number of omissions before
it; its length is the record count. Empty runs disappear, singletons use one
word, and other runs use two words. There are at most q+1 runs and
`2(q+1) <= k` whenever `q <= k/4`, so backward packing cannot overwrite unread omissions. The usual
forward decoder emits those runs with local color width 1, requiring no further
random draws or sorting. This takes expected O(q)+O(k) work and no extra array.
Directly expanding the omitted values into the same array would be unsafe;
the packed intermediate representation is what prevents overwrites.
The native dispatch uses the more conservative `q <= k/16` cutoff selected
by measurements; Python uses `q <= k/4`. The helper's storage proof supports
either cutoff. None of these choices assumes a specific cache capacity.

For **random order**, dense samples use Algorithm R: build a uniformly shuffled
initial reservoir, then scan the remaining population and replace a uniformly
chosen reservoir entry when appropriate. The n ≤ 2k restriction makes its work
O(k); applying it to arbitrary sparse populations would instead cost O(n).
Sparse samples use grouped sorted sampling followed by Fisher–Yates. These
return uniform **ordered** samples, a stronger requirement than arbitrary order.

## APIs and reproducibility

Python, including populations larger than a machine word:

```python
from algorithms.groupedchoose import sorted_choose, random_choose

sample = sorted_choose(10**100, 1000)
```

Put the repository's `python` directory on `PYTHONPATH`. Require integer inputs
with `0 <= k <= n`; invalid inequalities raise `ValueError`. As with any explicit
output, k must fit in addressable memory. The standard Python RNG is used;
`random.seed(...)` controls reproducibility.

C++ can use the header with a caller-provided unbiased bounded RNG:

```cpp
#include "groupedchoose.h"
#include "randbelow.h"

uint64_t output[1000];
rand_init();
auto draw = [](uint64_t bound) { return randbelow64(bound); };
groupedchoose::choose(UINT64_MAX, uint64_t{1000}, output, draw);
// choose<true>(...) requests uniform random order.
```

The shared library exports `sorted_groupedchoose` / `random_groupedchoose`
with `(uint32_t n, uint32_t k, uint32_t* out)` and the corresponding names with
suffix `64`, taking `uint64_t` throughout. The caller must supply k writable
elements and k ≤ n; the latter is an assertion/precondition in C++. The bundled
C++ API requires `rand_init()` before sampling; it resets the repository's
deterministic seed 324. That RNG has shared mutable state and is not thread-safe. The header API accepts
a separate RNG per caller when concurrency is needed.
A concrete functor or lambda also lets the compiler specialize the RNG call;
the exported C++ adapters use this form.

Rust exposes `sansreplace::groupedchoose::{sorted_order, random_order,
sorted_order64, random_order64}`. Each takes `(&mut rng, n, &mut output_slice)`;
the slice length is k. The 64-bit functions take `n: u64` and `&mut [u64]`.
An invalid sample length panics. None of these native samplers allocates;
the output takes 4k or 8k bytes and auxiliary storage is a fixed number of words.

```rust
use rand::{SeedableRng, rngs::SmallRng};
let mut rng = SmallRng::seed_from_u64(324);
let mut output = [0u64; 1000];
sansreplace::groupedchoose::sorted_order64(&mut rng, u64::MAX, &mut output);
```

See [running.md](running.md#grouped-cardchoose) for tests and the benchmark,
[results.md](results.md#grouped-cardchoose-results) for measured tradeoffs, and
the [self-contained Python function](examples/groupedchoose.py) and
[code screenshot](examples/groupedchoose.png). The compact function keeps the
general grouped construction and dense selection; it omits optional native
tiny-sample and complement optimizations, retaining the same bounds and law.
