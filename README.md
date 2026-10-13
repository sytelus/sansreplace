# Sampling without replacement

The problem of __sampling without replacement__: given integers _n_, _k_ such that 0 ≤ _k_ ≤ _n_,
return _k_ **distinct** nonnegative integers < _n_, choosing at random such that every possible
result is equally likely. One way to get a precise understanding of the problem is to look at the
simplest algorithm for solving it, via these [Python](python/algorithms/quadraticreject.py) or
[C++](cpp/quadraticreject.cpp) implementations.

Here, I describe an algorithm I call "Cardchoose" for this problem. The C++ and Rust
implementations now take O(_k_) time for their 32-bit integer API, use no hash tables or heap
allocations, and require only the output vector plus bounded stack workspace. They combine the
Pólya urn with cache-conscious integer sorting and fast paths for dense samples. The short Python
reference below retains the original O(_k_ log _k_) comparison sort.

See the [implementation and complexity notes](cardchoose.md#linear-native-implementation),
[updated measurements](results.md), and the reproducible `python/bench_cardchoose` benchmark.
The O(_k_) bound counts a bounded uniform random draw as constant time; with the C++ generator's
rejection loop included, it is an expected-time bound.

I first proposed Cardchoose in October 2018. Its two ingredients, a Pólya urn and the
stars-and-bars bijection, are classical mathematics; what I have not found elsewhere is the urn
run as a generator, or the two composed into a sampler (see [prior art](priorart.md)). I would
be glad to hear of any earlier description.

The [earlier benchmark results](results.md#historical-comparison-sort-results) compare the
original comparison-sort Cardchoose with other algorithms on an Apple M4. The new benchmark
compares revisions on the same machine, in both output orders, across small samples, cache-sized
and larger working sets, dense samples, and populations up to 2<sup>32</sup>−1. No AVX2, CPU
intrinsics, or `-march=native` build settings are required.

* [Guide to algorithms tested](algorithms.md)
* [Description of Cardchoose](cardchoose.md)
* [Prior art](priorart.md)
* [Benchmark results](results.md)
* [Running this software](running.md)

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

![graph](results/latest/cpp-sorted.webp)


This is not an officially supported Google product.
