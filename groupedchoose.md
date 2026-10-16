# Grouped Cardchoose

Grouped Cardchoose removes the algorithm's dependence on a fixed number of integer
digits. It samples a uniform sorted subset using **expected O(k) integer operations,
the k-element output array, and O(1) auxiliary integers**. The Python construction
uses only the output and scalar variables. Native fast paths add a **fixed 64-byte
bitmap** for local domains of at most 512 values and bounded standard-library
sorts (at most 128 elements in C++, 1024 in Rust). Their auxiliary workspace remains
constant: the bitmap and bounded sort stack do not grow with n or k. There is no
hash table, heap allocation, population-sized bitmap, or floating point. Local
work operates on contiguous memory; group counts occupy a prefix of the output.

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

The native implementations use power-of-two intervals to replace per-draw
division with a shift. Let B0 = ceil(k/G), with G=16 in C++ and G=64 in Rust,
and let w be the greatest power of two
not exceeding t/B0. There are B = ceil(t/w) groups; all have width w except the
last, which may be shorter. Thus B0 ≤ B ≤ 2B0. For the public grouped path,
k exceeds the bounded small-sample cutoff, so 2B ≤ 4B0 ≤ k. The test hook for
smaller target sizes caps B0 at floor(k/4) to retain this storage inequality.
The C++ setup finds w with at most 63 scalar shifts; Rust uses `leading_zeros`.
Neither operation needs a particular CPU instruction. Python retains the balanced
intervals above, without a fixed integer-width assumption.

## Why the distribution is exactly uniform

For the original urn, an ordered color sequence with multiplicities c_a has
probability `product(c_a!) / (t(t+1)...(t+k-1))`. There are
`k! / product(c_a!)` such orders. Consequently every multiset has probability
`1 / binomial(t+k-1,k)`. If its sorted colors are z_i, the map y_i = z_i + i
is a bijection onto increasing k-tuples in [0,n): its inverse is z_i = y_i − i.
This proves the local urn and the final stars-and-bars step, including duplicates.

Let group b contain s_b original colors and receive M_b draws. The aggregated
urn is a Pólya urn with initial group weights s_b. For counts m_b summing to k,

\[
  \Pr(M_b=m_b\ \text{for all }b)
  =\frac{\prod_b {s_b+m_b-1\choose m_b}}{{t+k-1\choose k}}.
\]

Indeed, each ordered group sequence has probability
`product(s_b(s_b+1)...(s_b+m_b-1)) / (t(t+1)...(t+k-1))`;
multiplying by its `k! / product(m_b!)` possible orders gives this formula.

The local urn produces each local multiset with probability
`1 / binomial(s_b + m_b - 1, m_b)`. Conditional on the counts, the local draws
are independent. Multiplication cancels every numerator factor, leaving
`1 / binomial(n,k)` for each global multiset, and therefore for each subset.
This argument includes repeated colors and correlated group occupancies; it
does not assume that the urn draws are independent uniform keys.

This cancellation applies to **any partition into consecutive color intervals**,
including the native power-of-two intervals. Given a group count m and color
width s, a local sorted m-subset of [0,s+m−1) is equivalent to its uniform color
multiset by the same bijection. Adding the group's first color and global start
index produces exactly the same conditional distribution as the local urn.

For local domain D = s+m−1 ≤ 512, the native code obtains this subset with
Floyd's sampler in eight 64-bit words. Starting with an empty set, for
j = D−m,...,D−1 it draws r uniformly in [0,j] and inserts j if r is already
present, otherwise r. To prove the induction, suppose the old set is a uniform
a-subset of [0,j). A target (a+1)-subset S not containing j has a+1 possible
last choices r in S, each with previous set S without r. If S contains j, its
previous set must be S without j, and the a+1 choices in that set or {j} all
produce S. In either case the probability is
`(a+1) / ((j+1)*binomial(j,a)) = 1 / binomial(j+1,a+1)`.
When m > D/2, sampling D−m omissions and complementing the bitmap has the same
uniform law and uses fewer draws. Every inserted bit is inside [0,D). For an
ordinary sample, set-bit enumeration returns its values in order. A complement
either enumerates the inverted bits (masking unused bits in the final word), or,
when omissions are at most m/16, enumerates omissions and fills the gaps as
contiguous runs. This avoids a bit-by-bit pass over a nearly full sample. No approximate probabilities
or collision retry assumption enters this path.

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

To obtain it, write the sum of squared counts as k plus the number of ordered
pairs of draws sharing a group. Exchangeability of the urn makes that pair
probability `sum(s_b(s_b+1)) / (t(t+1))`, without an independence assumption.

Balanced group sizes give

\[
 \frac{\sum_b s_b(s_b+1)}{t(t+1)}\le\frac1B+\frac2t.
\]

The sparse path is used only when n > 2k, hence t > k. Since B ≥ k/16,
the expected sum of squared occupancies is at most **19k**. Insertion-sorting
a group of m elements costs O(m²), so the expected total sorting work is O(k).
This also covers the one-group case, where k ≤ 16.
For native intervals, s_b ≤ w ≤ t/B0, so
`sum(s_b(s_b+1)) <= t(w+1)`. The second-moment factor is at most
`(w+1)/(t+1) <= 1/B0 + 1/t`, bounding the expected sum of squared occupancies
by **(G+2)k**: 18k for C++, 66k for Rust. These are conservative work bounds,
not instruction-count predictions. The larger Rust groups measured better with
its standard-library sorter and reduce metadata traffic. Native sorts are used only
for m ≤ 128 (C++) or m ≤ 1024 (Rust); larger groups retain insertion sort.
Their cost is bounded by O(m²) in either case. Skipping grouping for a sample
within that fixed small-sort cutoff also preserves O(k) work and O(1) auxiliary
workspace. Bitmap sampling takes at most m draws and O(m+8) work; every invoked
local group is nonempty, so this is O(m). The global bitmap shortcut is likewise
restricted to n ≤ 512 and k > 0.

Generating labels, counting, packing records, and writing output take O(k)
work. There are at most 2k bounded draws in the sparse sorted path and fewer
than 3k after adding a Fisher–Yates shuffle. Unbiased range reduction takes
expected constant raw draws per bounded draw. Rare occupancies can still make
insertion sorting quadratic; no worst-case linear bound is claimed here.

Writing the output already costs Ω(k) word operations, so the expected bound
is asymptotically optimal in this arithmetic model. This establishes neither
the smallest instruction count nor the fastest implementation on every CPU.

## What the performance guarantee means

Expected O(k) word operations match the Ω(k) cost of writing an explicit sample.
This is asymptotic optimality, not a guarantee of the best constant on every CPU,
compiler, population, and sample size. Empty and full samples leave little work
beyond dispatch and output stores. In some sparse 32-bit cases the specialized
Cardchoose radix buffers still beat the grouped construction's second set of
random draws and metadata passes. Both implementations remain available, and the
benchmark records losses as well as wins. Its 64-bit comparator is the original
urn plus comparison sort, not a 64-bit radix implementation.

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

All temporary values stored in the output fit in [0,n). Scalar expressions are also arranged
to avoid overflow: for example `t + (count - 2)`, rather than `(t + count) - 2`.
Empty samples return before computing n−k+1, which would overflow at the native
maximum population. Tests also instantiate the C++ kernel with 8-bit words
across every valid (n,k) pair to detect hidden use of spare bits.

## Dense and nearly full samples

For sorted samples with n ≤ 2k, Algorithm S selects the next value with
probability `remaining selections / remaining population`. It scans at most 2k
values. Full samples and up to two omissions have direct fill paths. Native k=1
and k=2 paths avoid the group machinery. The k=2 map draws a uniformly ordered
pair by bijectively skipping the first value; sorting it gives a uniform subset.
Populations through 512 use the bitmap shortcut. Samples through the small-sort
cutoff use a local sampler when another edge path does not apply. Algorithm S
is exact because each next inclusion has the conditional probability required
by a uniform subset of the remaining population. The general dense scan avoids
extra exit checks on every element. Full, single- and two-omission samples are emitted
as contiguous runs, avoiding a comparison on each output.

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
The native dispatch requires `q <= k/4` and either `q <= 1024` or `q <= k/16`;
Python uses `q <= k/4`. The bounded exception avoids scanning or sorting a large
sample when only a few values are omitted. Complementation is a bijection between
q-subsets and k-subsets, so the output is uniform. The storage proof supports
both dispatch rules. None of these choices assumes a specific cache capacity.

For **random order**, dense samples use Algorithm R: build a uniformly shuffled
initial reservoir, then scan the remaining population and replace a uniformly
chosen reservoir entry when appropriate. The n ≤ 2k restriction makes its work
O(k); applying it to arbitrary sparse populations would instead cost O(n).
Sparse samples use grouped sorted sampling followed by Fisher–Yates. These
return uniform **ordered** samples, a stronger requirement than arbitrary order.
For Algorithm R, induction preserves probability `1 / (i(i-1)...(i-k+1))` for
each ordered reservoir after i population values: either the new item is absent
and the old tuple is unchanged, or it replaces its specified position and the
removed item can be any of the i−k old values outside the new tuple. Inside-out
Fisher–Yates supplies the uniform initial permutation; the same shuffle after
sorted sampling gives probability `1/(binomial(n,k)*k!)` for each ordered sample.
All dispatch decisions depend only on n, k, and fixed constants, so choosing
between these exact samplers preserves their common uniform law.

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
The screenshot states the precondition `0 <= k <= n` and omits defensive input
validation for readability. Production Python retains its validation. Comments
explain the urn, grouping, rank conversion, counters, and safe record decoding.
