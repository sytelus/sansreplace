# Running this software

This is experimental software; it exists only to compare the various implementations, and it will
change in arbitrary ways as I extend my tests.

The development environment is managed with [devenv](https://devenv.sh/), which provides Python
and its needed packages, Meson, Ninja, and Rust.

* [Install devenv](https://devenv.sh/getting-started/).
* Either:
    * install [direnv](https://direnv.net/) and run `direnv allow` in the top directory of this
      repository, so the environment is loaded whenever you `cd` into it, or
    * run `devenv shell` in the top directory to start a shell with the environment loaded.
* Try it out. While all of the scripts provide help from the command line, you will almost certainly
need to read the source code to figure out what it does.

```
cd python
./test_choose --help
./time_choose --help
./plot --help
./scatter --help
```

The original C++ test driver runs from Python; the focused Cardchoose checks also run directly
through Meson (see below). The C++ tools automatically create the build directory
and build the code before running if needed; try eg `./test_choose 5 3`.

The Rust implementation is separate, but its subcommands mirror the Python tools and take the same
options: `time` is like `time_choose`, and writes CSV files in the same format, and `test` is like
`test_choose`:

```
cd rust
cargo run --release -- time --values small --values fib --csv results.csv
cargo run --release -- time --sorttype random --nk 1000000 1000
cargo run --release -- test 5 3 --algorithm hsel
```

To run the full C++ and Rust benchmarks in both orders, writing a CSV for each to a directory
(this takes several hours):

```
python/bench_all results/my-run
```

## Cardchoose regression benchmark

The focused benchmark builds the working tree and an archived Git revision with the same
compiler environment in fresh, separate temporary build directories. Existing Meson build
settings and Cargo target directories cannot supply stale benchmark binaries. It needs the
usual C++/Rust tools, CFFI, and Python 3.12 or newer. It alternates
revision order, measures both output orders serially, and reports medians and median absolute
deviations. Output allocation is outside the clock, as in the original harness; all work inside
the sampler is timed. On Linux, an optional CPU affinity reduces scheduling noise.

```
python/bench_cardchoose results/cardchoose-local --baseline 6048513 --cpu 2
```

Omit `--cpu` on other platforms or choose an available logical CPU. Options include `--language
cpp`, `--language rust`, `--repeats`, `--time-for`, and `--max-k`. The output directory contains
`raw.csv`, `summary.csv`, compiler/CPU/cache metadata (including both C++ compile commands and
Rust executable hashes), and the working-tree patch. Use a new or empty output directory;
existing results are never overwritten. Stage newly added files under `cpp`, `rust`, or `python`
before running so the patch captures them. Speedup is
baseline time divided by current time. Tiny timings include function-call overhead, and shared
machines still introduce noise; use repeated runs before interpreting small differences.
Large, very fast outputs can also be sensitive to lazy page allocation and batch size: Rust's
zeroed vector allocation can defer first-touch costs until the timed call. A longer `--time-for`
amortizes those costs across more iterations. Compare revisions with the same budget, and use
within-language speedups rather than treating the C++ and Rust timers as identical.
The C++ generator is reset to its existing seed (324) for each batch; Rust retains the existing
OS-seeded `SmallRng` behavior in both revisions.

The October 13 saved results used the former default Rust RNG settings in both revisions,
including the tiny bias in `rand`'s single bounded draw. Rust now enables `rand/unbiased` by
default for all algorithms, and `bench_cardchoose` explicitly enables it in both builds even
when the baseline predates that change. The explicit feature flag remains valid:

```
cargo run --release --features rand/unbiased --manifest-path rust/Cargo.toml -- time --algorithm cardchoose --nk 1000000 1000
```

Including unbiased range reduction gives an expected O(_k_) runtime bound for Cardchoose.
The exact enumeration tests inject ideal bounded draws, independently of RNG settings.

The `cache` value set probes small samples, implementation boundaries, typical cache working-set
sizes, nearly full samples, half-density and sparse samples, and `UINT32_MAX` populations. It is
also available in the original harnesses:

```
python/time_choose --algorithm cardchoose --values cache --max-k 8388608
cargo run --release --manifest-path rust/Cargo.toml -- time --algorithm cardchoose --values cache --max-k 8388608
```

Select algorithms explicitly for this grid: an O(_n_) competitor such as SELECT may try to
allocate for the full 32-bit population even when _k_ is tiny.

Run deterministic correctness checks:

```
meson setup cpp/build/host cpp
meson test -C cpp/build/host --print-errorlogs
cargo test --manifest-path rust/Cargo.toml
cargo test --release --manifest-path rust/Cargo.toml
cargo test --release --features rand/unbiased --manifest-path rust/Cargo.toml
```

For C++ address/undefined-behavior checks with the baseline x86-64 instruction set:

```
g++ -std=c++17 -O2 -march=x86-64 -mno-avx -mno-avx2 -fsanitize=address,undefined -fno-omit-frame-pointer cpp/tests/cardchoose_test.cpp -o /tmp/cardchoose-test
/tmp/cardchoose-test
```

The architecture flags in that last example are for an x86-64 test host only; they are not part
of the portable project build.

## Grouped Cardchoose

`groupedchoose` is registered in the standard Python, C++, and Rust harnesses alongside
`cardchoose`. The standard native harnesses retain their 32-bit arguments. For example:

```
python/time_choose --algorithm groupedchoose --values cache --max-k 1048576
cargo run --release --manifest-path rust/Cargo.toml -- time --algorithm groupedchoose --nk 1000000 1000
PYTHONPATH=python python -c 'from algorithms.groupedchoose import sorted_choose; print(sorted_choose(10**100, 8))'
```

The focused benchmark adds native 64-bit populations and Python populations through 4096 bits:

```
python/bench_groupedchoose results/grouped-local --repeats 5 --time-for 0.02 --max-k 8388608 --cpu 2
```

The full native grid includes the 128/256 sample cutoffs and both sides of the
512-value bitmap boundary, in addition to tiny, dense, nearly full and sparse samples.

Choose an available CPU or omit `--cpu`. The default maximum k is 1,048,576; the command above
also includes 8,388,608-element outputs (32 MiB / 64 MiB). `--quick` reduces the grid, and
`--language cpp`, `--language rust`, or `--language python` select implementations. A new or
empty result directory is required; stage new source files first. C++ and Cargo use fresh
build directories, all compilation finishes before measurements, and comparisons run serially
with rotating algorithm order. Compiler commands, binary hashes, source revision/patch,
CPU/cache details, seeds, raw calibrated batches, medians and MADs are saved.

All algorithms use unbiased range reduction. The C++ driver calls the exported library
samplers directly. Native outputs are allocated, first-touched, and
warmed once before each timed batch; RNG state is then reset to seed 324. Python includes
returned-list allocation and temporarily disables cyclic garbage collection during each
batch. Each reported batch meets `--time-for`, though a large sample may need only one
iteration. Compare within a language and word width. These timers differ from the earlier
Cardchoose regression timer; do not treat historical and new raw timings as a controlled
before/after comparison.

The 32-bit baseline is the existing optimized Cardchoose. The 64-bit baseline, `urnsort`, is
the original urn with a standard comparison sort, using the same 64-bit RNG as Grouped
Cardchoose. Reservoir sampling is included only for n ≤ 2k to avoid scanning enormous sparse
populations. Python compares against the original `cardchoose` reference.

The native fixed-iteration drivers can also be called directly:

```
meson setup cpp/build/host cpp
ninja -C cpp/build/host
cpp/build/host/bench_groupedchoose 64 groupedchoose sorted 18446744073709551615 1000 100
cargo run --release --manifest-path rust/Cargo.toml --bin bench_groupedchoose -- 64 groupedchoose random 18446744073709551615 1000 100
```

Arguments are `WIDTH ALGORITHM ORDER N K ITERATIONS`; output is a JSON object with elapsed
seconds and an output checksum. Use the library APIs in [groupedchoose.md](groupedchoose.md#apis-and-reproducibility)
for sampling into your own buffer.

Run the correctness checks (Python needs no third-party test dependency):

```
python -m unittest discover -s python/tests -v
meson test -C cpp/build/host --print-errorlogs
cargo test --manifest-path rust/Cargo.toml
cargo test --release --manifest-path rust/Cargo.toml
g++ -std=c++17 -O2 -march=x86-64 -mno-avx -mno-avx2 -fsanitize=address,undefined -fno-omit-frame-pointer cpp/tests/groupedchoose_test.cpp cpp/groupedchoose.cpp cpp/randbelow.c -o /tmp/groupedchoose-test
/tmp/groupedchoose-test
```

The last command's architecture flags are an x86-64 verification option, not a build
requirement. Checks include rationally weighted exhaustive draw trees, forced grouped and
complement kernels, packing boundaries, extreme occupancies, maximum native populations,
Python populations beyond 2^4096, and agreement with the standalone screenshot function.
The C++ test also exhausts all valid 8-bit (n,k) pairs with seeded draws.

The [self-contained function](examples/groupedchoose.py) uses only the Python standard library.
To regenerate its PNG and SVG screenshot, install the optional Pillow and Pygments packages
and run `python examples/render_groupedchoose.py`; `--font` selects a local monospace font.
The renderer uses the actual source text, and the SVG records its SHA-256 hash.

## Building the paper

The paper describes the original comparison-sort algorithm and historical benchmarks. The new
native implementation and measurements are documented in [cardchoose.md](cardchoose.md) and
[results.md](results.md). The paper's figures and tables are generated from the CSVs in
`results/latest`. After changing those, regenerate them with:

```
cd python
./make_plots
./all_least_sq
```

`make_plots` also writes the animated graphs in `results/latest`. Then build the PDF:

```
cd latex
./do_build
```

This writes `latex/cardchoose.pdf`; the first run downloads `latexrun` into `latex/build`. The
build prints some warnings from `latexrun` and from the LaTeX packages that can be ignored.
