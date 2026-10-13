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

Even the C++ tests are driven from Python. The C++ tools automatically create the build directory
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
compiler defaults. It needs the usual C++/Rust tools, CFFI, and Python 3.12 or newer. It alternates
revision order, measures both output orders serially, and reports medians and median absolute
deviations. Output allocation is outside the clock, as in the original harness; all work inside
the sampler is timed. On Linux, an optional CPU affinity reduces scheduling noise.

```
python/bench_cardchoose results/cardchoose-local --baseline 6048513 --cpu 2
```

Omit `--cpu` on other platforms or choose an available logical CPU. Options include `--language
cpp`, `--language rust`, `--repeats`, `--time-for`, and `--max-k`. The output directory contains
`raw.csv`, `summary.csv`, compiler/CPU/cache metadata, and the working-tree patch. Speedup is
baseline time divided by current time. Tiny timings include function-call overhead, and shared
machines still introduce noise; use repeated runs before interpreting small differences.
The C++ generator is reset to its existing seed (324) for each batch; Rust retains the existing
OS-seeded `SmallRng` behavior in both revisions.

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
```

For C++ address/undefined-behavior checks with the baseline x86-64 instruction set:

```
g++ -std=c++17 -O2 -march=x86-64 -mno-avx -mno-avx2 -fsanitize=address,undefined -fno-omit-frame-pointer cpp/tests/cardchoose_test.cpp -o /tmp/cardchoose-test
/tmp/cardchoose-test
```

The architecture flags in that last example are for an x86-64 test host only; they are not part
of the portable project build.

## Building the paper

The paper's figures and tables are generated from the CSVs in `results/latest`. After changing
those, regenerate them with:

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
