#include "../groupedchoose.h"
#include "../randbelow.h"

#include <cstdio>
#include <limits>
#include <map>
#include <numeric>
#include <random>
#include <vector>

// Rational arithmetic is exact for these small enumerations. In contrast to
// the original urn, paths here have different products of draw bounds.
struct Fraction {
    uint64_t numerator = 0, denominator = 1;
    Fraction divided(uint64_t b) const {
        const auto g = std::gcd(numerator, b);
        return {numerator / g, denominator * (b / g)};
    }
    void add(Fraction other) {
        const auto g = std::gcd(denominator, other.denominator);
        numerator = numerator * (other.denominator / g) + other.numerator * (denominator / g);
        denominator *= other.denominator / g;
        const auto reduce = std::gcd(numerator, denominator);
        numerator /= reduce; denominator /= reduce;
    }
};

template<class Word>
void valid(std::vector<Word> out, Word n, bool sorted) {
    if (sorted) assert(std::is_sorted(out.begin(), out.end()));
    std::sort(out.begin(), out.end());
    assert(std::adjacent_find(out.begin(), out.end()) == out.end());
    if (!out.empty()) assert(out.back() < n);
}

template<class Word, bool Random, bool ForceSparse, bool ForceComplement = false>
void exact(Word n, Word k) {
    std::map<std::vector<Word>, Fraction> probabilities;
    std::vector<Word> script;
    auto visit = [&](auto&& self, Fraction probability) -> void {
        size_t cursor = 0;
        Word next = 0;
        auto draw = [&](Word bound) {
            assert(bound != 0);
            if (cursor == script.size()) {
                if (next == 0) next = bound;
                return Word{0};
            }
            const auto r = script[cursor++];
            assert(r < bound);
            return r;
        };
        std::vector<Word> out(k, std::numeric_limits<Word>::max());
        if constexpr (ForceComplement) groupedchoose::complement<3>(n, k, out.data(), draw);
        else if constexpr (ForceSparse) groupedchoose::sparse<3>(n, k, out.data(), draw);
        else groupedchoose::choose<Random, 3>(n, k, out.data(), draw);
        if (next != 0) {
            for (Word r = 0; r < next; ++r) {
                script.push_back(r);
                self(self, probability.divided(next));
                script.pop_back();
            }
        } else {
            valid(out, n, !Random);
            probabilities[out].add(probability);
        }
    };
    visit(visit, {1, 1});
    uint64_t count = 1;
    for (Word i = 0; i < k; ++i) count *= n - i;
    if constexpr (!Random) for (Word i = 1; i <= k; ++i) count /= i;
    assert(probabilities.size() == count);
    for (const auto& entry : probabilities) {
        assert(entry.second.numerator == 1 && entry.second.denominator == count);
    }
}

template<class Word>
void boundaries() {
    std::mt19937_64 rng(324);
    for (Word n : {Word{0}, Word{1}, Word{2}, Word{65}, Word{129}, Word{255},
                   Word{65535}, Word{65536}, Word{65537},
                   Word(std::numeric_limits<Word>::max() / 2),
                   Word(std::numeric_limits<Word>::max() / 2 + 1),
                   std::numeric_limits<Word>::max()}) {
        for (Word k : {Word{0}, Word{1}, Word{2}, Word{3}, Word{15}, Word{16}, Word{17}, Word{31}, Word{32}, Word{33},
                       Word{63}, Word{64}, Word{65}, Word{127}, Word{128}, Word{129}, Word{255}, Word{256}, Word{257},
                       Word{511}, Word{512}, Word{513},
                       Word{1023}, Word{1024}, Word{1025}, Word{65536}}) {
            if (k > n) continue;
            for (bool random : {false, true}) {
                for (unsigned mode = 0; mode < 3; ++mode) {
                    auto draw = [&](Word bound) {
                        assert(bound != 0);
                        if (mode == 1) return Word{0};
                        if (mode == 2) return Word(bound - 1);
                        return std::uniform_int_distribution<Word>(0, bound - 1)(rng);
                    };
                    const Word guard = std::numeric_limits<Word>::max();
                    std::vector<Word> storage(size_t(k) + 2, guard);
                    if (random) groupedchoose::choose<true>(n, k, storage.data() + 1, draw);
                    else groupedchoose::choose(n, k, storage.data() + 1, draw);
                    assert(storage.front() == guard && storage.back() == guard);
                    valid(std::vector<Word>(storage.begin() + 1, storage.end() - 1), n, !random);
                }
            }
        }
    }
    // Rare occupancies and packing boundaries, with wider n and varying k.
    for (unsigned trial = 0; trial < 300; ++trial) {
        const Word n = Word(rng()) | 1;
        const Word k = std::min(n, Word(rng() % 4097));
        auto draw = [&](Word b) { return std::uniform_int_distribution<Word>(0, b - 1)(rng); };
        std::vector<Word> out(k);
        groupedchoose::choose<false, 3>(n, k, out.data(), draw);
        valid(out, n, true);
    }
    for (Word k : {Word{8}, Word{64}, Word{256}, Word{1023}, Word{1024}, Word{1025}, Word{65536}}) {
        for (Word omitted : {Word{2}, Word{3}, Word(k/4 - 1), Word(k/4), Word(k/4 + 1),
                             Word(k/16), Word(k/16 + 1), Word(k/16 == 0 ? 0 : k/16 - 1)}) {
            for (unsigned mode = 0; mode < 3; ++mode) {
                auto draw = [&](Word b) {
                    if (mode == 1) return Word{0};
                    if (mode == 2) return Word(b - 1);
                    return std::uniform_int_distribution<Word>(0, b - 1)(rng);
                };
                std::vector<Word> out(k);
                groupedchoose::choose(Word(k + omitted), k, out.data(), draw);
                valid(out, Word(k + omitted), true);
            }
        }
    }
}

static void range_reduction() {
    // Exhaustively verify the same complete-interval acceptance rule at 8 bits.
    for (unsigned bound = 1; bound < 256; ++bound) {
        unsigned counts[256] = {};
        for (unsigned r = 0; r < 256; ++r) {
            const auto remainder = r % bound;
            if (r - remainder <= 255 - (bound - 1)) ++counts[remainder];
        }
        for (unsigned i = 1; i < bound; ++i) assert(counts[i] == counts[0]);
    }
    rand_init();
    for (uint64_t bound : {1ull, 2ull, 3ull, 0x100000000ull, 0x100000001ull,
                          0x8000000000000001ull, 0xffffffffffffffffull}) {
        for (unsigned i = 0; i < 10000; ++i) assert(randbelow64(bound) < bound);
    }
}

static void exported_apis() {
    rand_init();
    for (bool random : {false, true}) {
        for (uint32_t k : {0u, 1u, 2u, 16u, 32u, 64u, 65u, 1000u, 1024u}) {
            std::vector<uint32_t> a(size_t(k) + 2, UINT32_MAX);
            std::vector<uint64_t> b(size_t(k) + 2, UINT64_MAX);
            for (uint64_t n : {uint64_t{1024}, uint64_t{UINT32_MAX}, uint64_t{UINT32_MAX} + 1,
                               uint64_t{UINT64_MAX}}) {
                if (n <= UINT32_MAX) {
                    (random ? random_groupedchoose : sorted_groupedchoose)(uint32_t(n), k, a.data() + 1);
                    assert(a.front() == UINT32_MAX && a.back() == UINT32_MAX);
                    valid(std::vector<uint32_t>(a.begin() + 1, a.end() - 1), uint32_t(n), !random);
                }
                (random ? random_groupedchoose64 : sorted_groupedchoose64)(n, k, b.data() + 1);
                assert(b.front() == UINT64_MAX && b.back() == UINT64_MAX);
                valid(std::vector<uint64_t>(b.begin() + 1, b.end() - 1), n, !random);
            }
        }
    }
}

template<class Word>
void local_boundaries() {
    std::mt19937_64 rng(612);
    const Word guard = std::numeric_limits<Word>::max(), start = 3;
    for (Word domain : {2u, 63u, 64u, 65u, 127u, 128u, 129u, 255u, 256u,
                        257u, 511u, 512u, 513u, 1024u}) {
        for (Word count : {1u, 2u, 3u, 16u, 63u, 64u, 65u, 127u, 128u,
                           129u, 255u, 256u, 257u, 511u, 512u}) {
            if (count > domain) continue;
            for (unsigned mode = 0; mode < 3; ++mode) {
                auto draw = [&](Word bound) {
                    assert(bound != 0);
                    if (mode == 0) return Word{0};
                    if (mode == 1) return Word(bound - 1);
                    return std::uniform_int_distribution<Word>(0, bound - 1)(rng);
                };
                std::vector<Word> out(size_t(count) + start + 2, guard);
                groupedchoose::local_sample(out.data(), start, count,
                    Word(guard - domain - start), Word(domain - count + 1), draw);
                assert(std::all_of(out.begin(), out.begin() + start,
                                   [=](Word x) { return x == guard; }));
                assert(out[out.size() - 2] == guard && out.back() == guard);
                const std::vector<Word> result(out.begin() + start, out.end() - 2);
                valid(result, guard, true);
                assert(result.front() >= guard - domain);
            }
        }
    }
}

int main() {
    // Exercise the complete value domain at a small word size. In particular,
    // this catches reliance on spare high bits for counts or packed records.
    std::mt19937 small_rng(324);
    for (unsigned n = 0; n <= 255; ++n) for (unsigned k = 0; k <= n; ++k) {
        std::vector<uint8_t> out(k);
        auto draw = [&](uint8_t b) {
            return uint8_t(std::uniform_int_distribution<unsigned>(0, b - 1)(small_rng));
        };
        groupedchoose::choose(uint8_t(n), uint8_t(k), out.data(), draw);
        valid(out, uint8_t(n), true);
        // The public bitmap shortcut covers this entire word size. Force the
        // packed representations too, so spare-bit/overflow coverage survives.
        if (n > 2 * k) {
            groupedchoose::sparse<3>(uint8_t(n), uint8_t(k), out.data(), draw);
            valid(out, uint8_t(n), true);
        }
        if (n > k && n - k <= k / 4) {
            groupedchoose::complement<3>(uint8_t(n), uint8_t(k), out.data(), draw);
            valid(out, uint8_t(n), true);
        }
    }
    for (uint32_t n = 0; n <= 7; ++n) for (uint32_t k = 0; k <= n; ++k) {
        exact<uint32_t, false, false>(n, k);
        exact<uint32_t, false, true>(n, k);
        exact<uint64_t, false, true>(n, k);
        if (n <= 5) {
            exact<uint32_t, true, false>(n, k);
            exact<uint64_t, true, false>(n, k);
        }
    }
    boundaries<uint32_t>();
    boundaries<uint64_t>();
    range_reduction();
    exported_apis();
    local_boundaries<uint32_t>();
    local_boundaries<uint64_t>();
    for (uint32_t n = 5; n <= 16; ++n) for (uint32_t q = 1; q <= std::min(3u, n/5); ++q) {
        exact<uint32_t, false, false, true>(n, n-q);
        exact<uint64_t, false, false, true>(n, n-q);
    }
    std::puts("Grouped Cardchoose: exact probabilities, 32/64-bit boundaries, packing and RNG checks passed.");
}
