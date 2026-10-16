/*
 * Copyright 2018 Google LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include <stdint.h>

#include <algorithm>
#include <cassert>
#include <numeric>

#include "randbelow.h"

namespace {

// Fixed bounds matter: comparison sorting never sees more than small_sort
// elements, and the radix recursion visits at most four bytes of a uint32_t.
constexpr uint32_t small_sort = 128;
constexpr uint32_t small_complement = 1024;

// A bounded LSD leaf turns several random-access partition steps into
// sequential scans. The 32 KiB scratch bound is independent of k. Small
// histograms stay hot while the contiguous source and scratch use L1/L2 caches.
constexpr uint32_t radix_tile = 8192;

template<unsigned Passes>
void tile_sort(uint32_t* data, uint32_t size) {
    uint32_t scratch[radix_tile];
    uint32_t hist[Passes][256] = {};
    for (uint32_t i = 0; i < size; ++i) {
        auto value = data[i];
        for (unsigned p = 0; p < Passes; ++p) ++hist[p][(value >> (8 * p)) & 255];
    }
    for (auto& counts : hist) {
        uint32_t sum = 0;
        for (auto& count : counts) {
            auto old = count;
            count = sum;
            sum += old;
        }
    }
    auto* in = data;
    auto* out = scratch;
    for (unsigned p = 0; p < Passes; ++p) {
        for (uint32_t i = 0; i < size; ++i) {
            auto value = in[i];
            out[hist[p][(value >> (8 * p)) & 255]++] = value;
        }
        std::swap(in, out);
    }
    if (in != data) std::copy_n(in, size, data);
}

// In-place American flag sort. Two 1 KiB tables describe the current byte's
// buckets; descendants work on contiguous subarrays, improving cache locality.
// Align the first digit to the significant bits, not to a byte boundary, so
// a partly occupied high byte does not leave most of the first buckets empty.
// No allocation, population-sized bitmap, or output-sized temporary is needed.
void radix_sort(uint32_t* data, uint32_t size, unsigned bits) {
    if (size < 2) return;
    if (size <= small_sort) {
        std::sort(data, data + size);
        return;
    }

    if (size <= radix_tile && bits > 8) {
        switch ((bits + 7) / 8) {
            case 2: tile_sort<2>(data, size); break;
            case 3: tile_sort<3>(data, size); break;
            case 4: tile_sort<4>(data, size); break;
        }
        return;
    }

    const unsigned digit_bits = std::min(bits, 8u);
    const unsigned shift = bits - digit_bits;
    const unsigned buckets = 1u << digit_bits;
    const uint32_t mask = buckets - 1;
    uint32_t end[256] = {};
    for (uint32_t i = 0; i < size; ++i) ++end[(data[i] >> shift) & mask];

    // All higher bits already agree. Counting out the last digit avoids a
    // final scatter, and is particularly cheap for repeated urn values.
    if (shift == 0) {
        auto prefix = data[0] & ~mask;
        for (unsigned b = 0; b < buckets; ++b) {
            data = std::fill_n(data, end[b], prefix + b);
        }
        return;
    }

    uint32_t next[256];
    uint32_t sum = 0;
    for (unsigned b = 0; b < buckets; ++b) {
        next[b] = sum;
        sum += end[b];
        end[b] = sum;
    }
    for (unsigned b = 0; b + 1 < buckets; ++b) {
        while (next[b] < end[b]) {
            auto value = data[next[b]];
            auto dest = (value >> shift) & mask;
            while (dest != b) {
                std::swap(value, data[next[dest]++]);
                dest = (value >> shift) & mask;
            }
            data[next[b]++] = value;
        }
    }
    // The last bucket is already in place once the others are finished.
    uint32_t start = 0;
    for (unsigned b = 0; b < buckets; ++b) {
        auto count = end[b] - start;
        if (count > 1) radix_sort(data + start, count, shift);
        start = end[b];
    }
}

// The original Pólya urn, retaining its one bounded draw per element.
inline void sorted_urn(uint32_t n, uint32_t k, uint32_t* result) {
    if (k == 0) return;  // Also prevents n - k + 1 overflowing at UINT32_MAX.
    auto t = n - k + 1;
    for (uint32_t i = 0; i < k; ++i) {
        uint32_t r = randbelow(t + i);
        result[i] = r < t ? r : result[r - t];
    }
    if (k <= small_sort) {
        std::sort(result, result + k);
    } else {
        unsigned bits = 0;
        for (auto high = t - 1; high; high >>= 1) ++bits;
        radix_sort(result, k, std::max(bits, 1u));
    }
}

// Algorithm S is linear in n. Restricting it to n <= 2*k makes it linear
// in k too, and avoids sorting and scattered urn reads for dense samples.
// Random order fuses selection with inside-out Fisher-Yates.
template<bool Shuffle>
void dense_choose(uint32_t n, uint32_t k, uint32_t* result) {
    uint32_t written = 0;
    for (uint32_t value = 0; value < n; ++value) {
        if (randbelow(n - value) < k - written) {
            result[written] = value;
            if constexpr (Shuffle) {
                if (written != 0) {
                    std::swap(result[written], result[randbelow(written + 1)]);
                }
            }
            ++written;
        }
    }
}

// A fixed-size complement handles almost-full samples without allocating a
// second array proportional to k. Complementation is a bijection on subsets.
// The caller bounds omitted by small_complement before entering this helper.
void choose_edges(uint32_t n, uint32_t k, uint32_t* result) {
    if (k == 0) return;
    if (k == n) {
        std::iota(result, result + k, uint32_t{0});
        return;
    }
    auto omitted = n - k;
    if (omitted == 1) {
        auto excluded = randbelow(n);
        std::iota(result, result + excluded, uint32_t{0});
        std::iota(result + excluded, result + k, excluded + 1);
        return;
    }
    uint32_t missing[small_complement];
    sorted_urn(n, omitted, missing);
    uint32_t first = 0;
    uint32_t written = 0;
    for (uint32_t j = 0; j < omitted; ++j) {
        auto excluded = missing[j] + j;
        auto count = excluded - first;
        std::iota(result + written, result + written + count, first);
        written += count;
        first = excluded + 1;
    }
    std::iota(result + written, result + k, first);
}

}  // namespace

extern "C" void sorted_cardchoose(uint32_t n, uint32_t k, uint32_t* result) {
    assert(k <= n);
    if (k <= 1) {
        if (k == 1) result[0] = n == 1 ? 0 : randbelow(n);
        return;
    }
    if (k == 2 && n > 2) {
        auto a = randbelow(n);
        auto b = randbelow(n - 1);
        b += b >= a;  // Map [0, n-1) bijectively onto [0, n) excluding a.
        result[0] = std::min(a, b);
        result[1] = std::max(a, b);
        return;
    }
    auto omitted = n - k;
    if ((omitted <= k / 4 || omitted == 1) && omitted <= small_complement) {
        choose_edges(n, k, result);
        return;
    }
    if (k >= 128 && n - k <= k) {
        dense_choose<false>(n, k, result);
        return;
    }
    sorted_urn(n, k, result);
    for (uint32_t i = 0; i < k; ++i) result[i] += i;
}

extern "C" void random_cardchoose(uint32_t n, uint32_t k, uint32_t* result) {
    assert(k <= n);
    if (k <= 1) {
        if (k == 1) result[0] = n == 1 ? 0 : randbelow(n);
        return;
    }
    if (k == 2 && n > 2) {
        auto a = randbelow(n);
        auto b = randbelow(n - 1);
        result[0] = a;
        result[1] = b + (b >= a);
        return;
    }
    auto omitted = n - k;
    if ((omitted <= k / 4 || omitted == 1) && omitted <= small_complement) {
        choose_edges(n, k, result);
        for (uint32_t i = 1; i < k; ++i) {
            std::swap(result[i], result[randbelow(i + 1)]);
        }
        return;
    }
    if (k >= 128 && n - k <= k) {
        dense_choose<true>(n, k, result);
        return;
    }
    sorted_urn(n, k, result);
    // Fuse the stars-and-bars conversion with inside-out Fisher-Yates. The
    // element at index zero needs neither an addition nor a random draw.
    for (uint32_t i = 1; i < k; ++i) {
        uint32_t r = randbelow(i + 1);
        auto value = result[i] + i;
        result[i] = result[r];
        result[r] = value;
    }
}
