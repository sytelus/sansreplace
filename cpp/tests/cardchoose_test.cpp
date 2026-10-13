// Exact bounded-draw enumeration, adversarial sorting and boundary checks.
// The production sampler is included so its private radix sorter is tested too.
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <map>
#include <numeric>
#include <random>
#include <vector>

#include "../cardchoose.cpp"

static std::mt19937 rng(324);
static bool enumerate_draws = false;
static std::vector<uint32_t> draws;
static size_t draw_index;
struct NeedDraw { uint32_t range; };

extern "C" uint32_t randbelow(uint32_t range) {
    assert(range > 0);
    if (enumerate_draws) {
        if (draw_index == draws.size()) throw NeedDraw{range};
        auto r = draws[draw_index++];
        assert(r < range);
        return r;
    }
    return std::uniform_int_distribution<uint32_t>(0, range - 1)(rng);
}

using Sampler = void (*)(uint32_t, uint32_t, uint32_t*);
using Counts = std::map<std::vector<uint32_t>, uint64_t>;

static void valid(std::vector<uint32_t> a, uint32_t n, bool sorted) {
    if (sorted) assert(std::is_sorted(a.begin(), a.end()));
    std::sort(a.begin(), a.end());
    assert(std::adjacent_find(a.begin(), a.end()) == a.end());
    if (!a.empty()) assert(a.back() < n);
}

static void enumerate(Sampler f, uint32_t n, uint32_t k, Counts& counts, bool sorted) {
    std::vector<uint32_t> a(k, 0xdeadbeef);
    draw_index = 0;
    try {
        f(n, k, a.data());
        valid(a, n, sorted);
        ++counts[a];
    } catch (NeedDraw next) {
        for (uint32_t r = 0; r < next.range; ++r) {
            draws.push_back(r);
            enumerate(f, n, k, counts, sorted);
            draws.pop_back();
        }
    }
}

static void exact_uniformity() {
    enumerate_draws = true;
    for (bool dense : {false, true}) {
        for (bool sorted : {true, false}) {
            auto f = dense ? (sorted ? dense_choose<false> : dense_choose<true>)
                           : (sorted ? sorted_cardchoose : random_cardchoose);
            auto max_n = dense ? (sorted ? 7u : 5u) : (sorted ? 8u : 6u);
            for (uint32_t n = 0; n <= max_n; ++n) {
                for (uint32_t k = 0; k <= n; ++k) {
                    Counts counts;
                    enumerate(f, n, k, counts, sorted);
                    uint64_t expected = 1;
                    for (uint32_t i = 0; i < k; ++i) expected *= n - i;
                    if (sorted) for (uint32_t i = 1; i <= k; ++i) expected /= i;
                    assert(counts.size() == expected);
                    auto multiplicity = counts.begin()->second;
                    for (const auto& entry : counts) assert(entry.second == multiplicity);
                }
            }
        }
    }
    enumerate_draws = false;
}

static void sorting() {
    for (uint32_t n : {0u, 1u, 2u, 63u, 64u, 127u, 128u, 129u, 255u, 256u,
                       257u, 1024u, 2047u, 2048u, 2049u, 4095u, 4096u, 4097u,
                       8191u, 8192u, 8193u, 16383u, 16384u, 16385u, 65536u, 1048576u}) {
        for (unsigned bits : {1u, 7u, 8u, 9u, 13u, 15u, 16u, 17u, 23u, 24u, 25u, 31u, 32u}) {
            auto mask = UINT32_MAX >> (32 - bits);
            for (unsigned pattern = 0; pattern < 6; ++pattern) {
                std::vector<uint32_t> a(n);
                for (auto& x : a) {
                    x = rng() & mask;
                    if (pattern == 1) x = mask;
                    if (pattern == 2) x &= 3;
                    if (pattern == 3) x |= mask & 0xffffff00u;
                }
                if (pattern == 4) std::sort(a.begin(), a.end());
                if (pattern == 5) std::sort(a.rbegin(), a.rend());
                auto expected = a;
                std::sort(expected.begin(), expected.end());
                radix_sort(a.data(), n, bits);
                assert(a == expected);
            }
        }
    }
}

static void boundaries() {
    for (uint32_t n : {0u, 1u, 2u, 64u, 65u, 128u, 129u, 255u, 256u, 257u,
                       65535u, 65536u, 65537u, 0x7fffffffu, 0x80000000u, UINT32_MAX}) {
        for (uint32_t k : {0u, 1u, 2u, 63u, 64u, 65u, 127u, 128u, 129u, 255u,
                           256u, 257u, 1024u, 65536u}) {
            if (k > n) continue;
            for (bool sorted : {true, false}) {
                auto f = sorted ? sorted_cardchoose : random_cardchoose;
                // Canaries detect writing outside the output even without ASan.
                std::vector<uint32_t> a(k + 2, 0xdeadbeef);
                f(n, k, a.data() + 1);
                assert(a.front() == 0xdeadbeef && a.back() == 0xdeadbeef);
                valid({a.begin() + 1, a.end() - 1}, n, sorted);
            }
        }
    }
    for (uint32_t k : {4096u, 65536u}) {
        for (uint32_t omitted : {0u, 1u, 2u, 63u, 64u, 65u, 255u, 256u, 257u,
                                 1023u, 1024u, 1025u}) {
            for (bool sorted : {true, false}) {
                auto f = sorted ? sorted_cardchoose : random_cardchoose;
                std::vector<uint32_t> a(k + 2, 0xdeadbeef);
                f(k + omitted, k, a.data() + 1);
                assert(a.front() == 0xdeadbeef && a.back() == 0xdeadbeef);
                valid({a.begin() + 1, a.end() - 1}, k + omitted, sorted);
            }
        }
    }
    sorted_cardchoose(UINT32_MAX, 0, nullptr);
    random_cardchoose(UINT32_MAX, 0, nullptr);
    sorted_cardchoose(0, 0, nullptr);
    random_cardchoose(0, 0, nullptr);
}

int main() {
    exact_uniformity();
    sorting();
    boundaries();
    std::puts("Cardchoose: exact uniformity, radix sorting and boundaries passed");
}
