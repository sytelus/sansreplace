# Cardchoose algorithm

Algorithm | Python | C++ | Order guarantee | Data structures | Time
----|----|----|----|----|----
[Cardchoose](cardchoose.md) | [Python](python/algorithms/cardchoose.py) | [C++](cpp/cardchoose.cpp) | Sorted | output plus bounded workspace | _k_ (native implementations)

Cardchoose returns _k_ distinct integers from 0 to _n_-1, with every _k_-element subset equally
likely. The following Python code is the original mathematical reference: its only storage is the
output list, everything except the final sort is O(_k_), and the urn loop has no searches or inner
loops. The native implementations replace that final comparison sort and add the exact fast paths
described below.

```python
def sorted_choose(n, k):
    "k distinct integers 0 <= x < n, sorted"
    t = n - k + 1
    d = [None] * k
    for i in range(k):
        r = random.randrange(t + i)
        if r < t:
            d[i] = r
        else:
            d[i] = d[r - t]
    d.sort()
    for i in range(k):
        d[i] += i
    return d
```

## Linear native implementation

The C++ and Rust APIs operate on 32-bit unsigned integers with `0 <= k <= n`. They preserve the
urn and stars-and-bars distribution; only the implementation and exact special cases change.

* Empty and singleton samples return directly. A full sample is the range `[0, n)`.
* If the complement has at most 1024 elements and at most a quarter as many elements as the
  output, sample it in a fixed stack array and emit the gaps as contiguous runs. Complementation
  is a bijection between equally likely subsets, so this remains uniform. It avoids nearly _k_
  random draws for an almost-full sample.
* For `k >= 128` and `n <= 2*k`, use exact sequential selection (Algorithm S). It needs at most
  `2*k` bounded random draws, touches the output sequentially, and uses constant workspace.
  Random order fuses acceptance with inside-out Fisher–Yates. This restriction is essential:
  unrestricted Algorithm S would take O(_n_) time.
* Otherwise, generate the original urn values and sort them with an in-place most-significant
  digit radix partition. Digits have at most eight bits. The first digit is aligned to the
  significant bits of `n-k`, avoiding mostly empty top-level buckets. Each level has two 256-entry
  `uint32_t` tables; recursive buckets are contiguous, and there are at most four levels.
* Small buckets use bounded comparison sorting; medium buckets use stable least-significant
  digit passes in a fixed stack buffer. Histograms for all needed digits are built in one scan.
  The last digit can instead be emitted from its counts. C++ limits a buffer to 8192 words and
  Rust to 16384 words; Rust uses smaller buffers for smaller buckets. These are portable tuning
  choices, not requirements on cache sizes or instruction sets.
* Add ranks to turn the multiset into the subset. For random order, fuse that addition with
  inside-out Fisher–Yates and omit the unnecessary one-choice draw at index zero.

The general path still takes exactly _k_ bounded draws to generate its urn. Every partition and
radix pass is linear. There are at most four digit levels; every comparison sort is capped at
128 elements in C++, and at 256 (or fewer than 1024 for a sparse last digit) in Rust. Therefore
comparison sorting contributes only a fixed multiple of _k_, even for adversarial inputs.
Initializing and scanning the fixed tables is also O(_k_), since only buckets larger than the
fixed small-sort cutoff use them. Shuffling, dense selection and complement expansion each take
O(_k_). The result is O(_k_) algorithmic work, with no dependence on the size of _n_ in the sparse
path. Including the existing rejection-based bounded random generator makes the C++ running time
**expected O(_k_)**; an arbitrary random bit stream cannot give a finite worst-case bound for
unbiased bounded draws.

The output uses exactly `4*k` bytes. Auxiliary storage is **O(1)**: fixed tables, a fixed radix
buffer, a 4 KiB complement buffer, and bounded call frames. No heap allocation, hash table,
bitmap indexed by _n_, or additional array proportional to _k_ is used. A conservative bound on
the explicit arrays is 48 KiB for C++ and 80 KiB for Rust; actual live storage depends on the
path, and compiler-generated call frames are additional. Total space is `4*k + O(1)` bytes,
the minimum coefficient for this API's uncompressed 32-bit output. The Python reference remains
O(_k_ log _k_); these fixed-width bounds do not extend to arbitrary-precision integers.

The fast paths change seeded output sequences. They preserve the subset and ordering
distributions, not bit-for-bit agreement with older versions. Exhaustive bounded-draw tests check
small subsets and ordered samples, including the dense kernel. Adversarial sort tests cover
partial digits, repeated values, sorted/reverse inputs, dispatch boundaries, and the full 32-bit
range. See [running the checks and benchmarks](running.md#cardchoose-regression-benchmark).

## A subset is a row of stars and bars

Write the numbers 0 to _n_-1 in a row, marking each chosen number as a star and each unchosen one
as a bar. For _n_ = 7, _k_ = 3 and the subset {1, 2, 5}:

```
number:  0 1 2 3 4 5 6
row:     | ★ ★ | | ★ |
```

A row of _k_ stars and _n_-_k_ bars is exactly a _k_-element subset: the stars' positions are the
chosen numbers.

## A row of stars and bars is a multiset

Instead of its position, describe each star by its **value**, the number of bars to its left:

```
row:     | ★ ★ | | ★ |
value:     1 1     3
```

A value can be anything from 0 (before all the bars) to
_n_-_k_ (after all of them), so there are _t_ = _n_-_k_+1 possible values, and stars can share a
value when they sit in the same gap between bars.

The values of the stars form a multiset: a bag of _k_ values, with repeats allowed. This is a
bijection between _k_-element subsets of {0, …, _n_-1} and _k_-element multisets drawn from the
_t_ values {0, …, _n_-_k_}. Going one way, read off each star's value. Going back, the multiset says how many stars have each value, that is, how
many stars go in each gap, and that rebuilds the row. The two directions undo each other, so each
subset corresponds to exactly one multiset and each multiset to exactly one subset.

In code, the way back is: sort the multiset, then add each star's index to its value, because a
star's position is its bars to the left plus its stars to the left:

```
sorted values:  1 1 3
+ index:        0 1 2
positions:      1 2 5
```

That is the last three lines of the code. Because the correspondence is a bijection, choosing a
multiset uniformly at random and converting it gives a subset chosen uniformly at random: each
subset is reached from exactly one multiset. So what remains is to pick a multiset of _k_ values
from _t_ **uniformly at random**.

## The obvious way to pick a multiset is biased

Drawing _k_ values independently from 0 to _t_-1 does not give a uniform multiset. With _t_ = 2,
_k_ = 2, the bag {0, 1} comes up half the time, because it can arrive as 0,1 or as 1,0, while
{0, 0} and {1, 1} get a quarter each. Independent draws favour variety; we need to favour repeats,
by exactly the right amount.

## The fix is to let earlier draws vote

For the _i_-th draw, choose uniformly among _t_+_i_ options: the _t_ fresh values, plus the _i_
values already in the list. That is `randrange(t + i)`. If `r < t`, it's a fresh value. Otherwise
`r - t` picks one of the earlier entries, and we copy it: `d[r - t]`. A value that has already
been drawn once is now twice as likely to be drawn again, a value drawn twice is three times as
likely, and so on.

The list is never searched or modified in place; copies go on the end like everything else. So
each iteration is one random number, one comparison and one write, with no inner loop.

In the picture, this is dropping the new star into a uniformly random gap of the current row.
Before the _i_-th star there are _t_-1+_i_ things in the row, so _t_+_i_ gaps, and the gaps with
_b_ bars to the left are the _c_+1 gaps around the _c_ stars that already have value _b_: one
fresh option, plus one for each existing star with that value.

## Why that's exactly fair

Tag each star with the number of the draw that placed it, and be specific about which gap each
draw uses: a fresh value _b_ goes in the gap just after bar _b_ (at the start of the row for
_b_ = 0), and a copy of star _j_ goes in the gap just after star _j_. Then different draws put
the new star in different gaps, and a tagged row records its whole history: pull out the
highest-tagged star, and its left neighbour says whether it was fresh (a bar, or nothing) or a
copy (a star, and which one).

Here is every possible run for _t_ = 2, _k_ = 2, one bar and two stars:

```mermaid
graph TD
    S["#124;"]
    S -- "r = 0, fresh value 0" --> A["★₁ #124;"]
    S -- "r = 1, fresh value 1" --> B["#124; ★₁"]
    A -- "r = 0, fresh value 0" --> A0["★₂ ★₁ #124;"]
    A -- "r = 1, fresh value 1" --> A1["★₁ #124; ★₂"]
    A -- "r = 2, copy ★₁" --> A2["★₁ ★₂ #124;"]
    B -- "r = 0, fresh value 0" --> B0["★₂ #124; ★₁"]
    B -- "r = 1, fresh value 1" --> B1["#124; ★₂ ★₁"]
    B -- "r = 2, copy ★₁" --> B2["#124; ★₁ ★₂"]
```

Every path has the same probability, 1/2 × 1/3, and the six leaves are the six tagged rows:
three ways to place two stars around a bar, times two ways to tag them. In general there are
_t_ × (_t_+1) × … × (_t_+_k_-1) paths, each with probability 1/_t_ × 1/(_t_+1) × … ×
1/(_t_+_k_-1), and the same number of tagged rows, so every tagged row is equally likely. Each
untagged row has exactly _k_! taggings, so every untagged row is equally likely, which is to say
every multiset, which is to say every subset. In the tree, each of the three untagged rows
`★ ★ |`, `★ | ★` and `| ★ ★` is the leaf of two paths, so each has probability 1/3.

## Worked example

_n_ = 7, _k_ = 3, so _t_ = 5 and we start with four bars: `| | | |`.

draw | `randrange` | `r` | meaning | `d` | row
----|----|----|----|----|----
0 | 5 | 1 | fresh value 1 | `[1]` | `\| ★ \| \| \|`
1 | 6 | 3 | fresh value 3 | `[1, 3]` | `\| ★ \| \| ★ \|`
2 | 7 | 5 | 5 ≥ _t_, copy `d[0]` = 1 | `[1, 3, 1]` | `\| ★ ★ \| \| ★ \|`

Sort: `[1, 1, 3]`. Add indices: `[1, 2, 5]`. Reading the final row left to right, the stars sit at
positions 1, 2 and 5, with the unchosen numbers 0, 3, 4 and 6 as the bars.

## Random order

In the Python reference, `random_choose` runs the sorted version and then shuffles the result.
The native implementations fuse this work where possible and use _k_−1 bounded draws for the
shuffle when _k_ is positive.

How this relates to earlier work is in [prior art](priorart.md).
