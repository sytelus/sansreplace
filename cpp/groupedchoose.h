// Grouped Pólya urn: expected O(k) word operations, output + O(1) words.
// Fixed 64-byte local bitmap and bounded sort stack; no allocation,
// wide products or required instruction-set extensions.
// See groupedchoose.md for uniformity, expected-work and overwrite proofs.
#pragma once

#include <algorithm>
#include <cassert>
#include <cstdint>
#include <type_traits>

namespace groupedchoose {

constexpr unsigned small_sort = 128;
constexpr unsigned default_group_size = 16;

template<class Word, class Draw>
void local_sample(Word* out, Word start, Word count, Word low, Word width, Draw& draw) {
    const Word stop = start + count;
    if (width == 1) {
        for (Word i = start; i < stop; ++i) out[i] = low + i;
        return;
    }
    const Word domain = width + (count - 1);
    if (domain <= 512) {
        // Floyd's sampler on a bounded bitmap: no collision search or sort.
        uint64_t bits[8] = {};
        const bool invert = count > domain / 2;
        const Word draws = invert ? domain - count : count;
        for (Word j = domain - draws; j < domain; ++j) {
            const Word r = draw(Word(j + 1));
            const Word v = (bits[r / 64] & (uint64_t{1} << (r % 64))) ? j : r;
            bits[v / 64] |= uint64_t{1} << (v % 64);
        }
        Word written = start;
        const unsigned words = 1 + (domain - 1) / 64;
        for (unsigned b = 0; b < words; ++b) {
            auto mask = invert ? ~bits[b] : bits[b];
            if (b + 1 == words && domain % 64 != 0)
                mask &= (uint64_t{1} << (domain % 64)) - 1;
            while (mask) {
                unsigned bit = 0;
#if defined(__GNUC__) || defined(__clang__)
                bit = unsigned(__builtin_ctzll(mask));
#else
                for (auto shifted = mask; (shifted & 1) == 0; shifted >>= 1) ++bit;
#endif
                out[written++] = low + start + Word(64 * b + bit);
                mask &= mask - 1;
            }
        }
        return;
    }
    for (Word j = 0; j < count; ++j) {
        const Word r = draw(width + j);
        out[start + j] = r < width ? low + r : out[start + r - width];
    }
    if (count <= small_sort) {
        std::sort(out + start, out + stop);
    } else {
        for (Word i = start + 1; i < stop; ++i) {
            const Word value = out[i];
            Word j = i;
            while (j > start && out[j - 1] > value) {
                out[j] = out[j - 1];
                --j;
            }
            out[j] = value;
        }
    }
    for (Word i = start; i < stop; ++i) out[i] += i;
}

template<class Word, class Draw>
void emit_records(Word* out, Word k, Word read, Word t, Word width, Draw& draw) {
    Word start = 0;
    while (read < k) {
        const Word tag = out[read++];
        if (tag < t) {
            out[start] = tag + start;
            ++start;
        } else {
            const Word count = tag - t + 2, b = out[read++];
            assert(count <= read - start);
            const Word low = b * width;
            local_sample(out, start, count, low, std::min(width, Word(t - low)), draw);
            start += count;
        }
    }
    assert(start == k);
}

template<unsigned GroupSize = default_group_size, class Word, class Draw>
void sparse(Word n, Word k, Word* out, Draw& draw) {
    static_assert(GroupSize >= 3);
    if (k == 0) return;  // n-k+1 need not be representable for an empty sample.
    const Word t = n - k + 1;
    if constexpr (GroupSize == default_group_size) {
        if (k <= small_sort) {
            local_sample(out, Word{0}, k, Word{0}, t, draw);
            return;
        }
    }
    if (k < 4) {
        local_sample(out, Word{0}, k, Word{0}, t, draw);
        return;
    }
    const Word wanted = std::min(t, std::min(Word(k / 4), Word(1 + (k - 1) / GroupSize)));
    Word width = 1;
    unsigned shift = 0;
    for (Word high = t / wanted; high > 1; high >>= 1) { width <<= 1; ++shift; }
    const Word groups = 1 + ((t - 1) >> shift);
    if (groups == 1) {
        local_sample(out, Word{0}, k, Word{0}, t, draw);
        return;
    }
    assert(groups <= k / 2);
    // Power-of-two color intervals replace a division on every fresh draw
    // with a shift. Only the final interval can be shorter.
    for (Word i = 0; i < k; ++i) {
        const Word r = draw(t + i);
        out[i] = r < t ? 1 + (r >> shift) : out[r - t];
    }
    // Destructively histogram one-based labels. Counts are tagged by groups.
    Word i = 0;
    while (i < k) {
        if (out[i] == 0 || out[i] > groups) { ++i; continue; }
        const Word b = out[i] - 1;
        if (out[b] != 0 && out[b] <= groups) {
            out[i] = out[b];
            out[b] = groups + 1;
        } else {
            out[b] = out[b] == 0 ? groups + 1 : out[b] + 1;
            out[i++] = 0;
        }
    }
    Word read = k;
    for (Word b = groups; b-- > 0;) {
        const Word count = out[b] == 0 ? 0 : out[b] - groups;
        if (count == 1) {
            const Word low = b * width;
            out[--read] = low + draw(std::min(width, Word(t - low)));
        } else if (count > 1) {
            read -= 2;
            out[read] = t + (count - 2);  // <= n-1, even when n is WORD_MAX.
            out[read + 1] = b;
        }
        assert(read >= b);
    }
    emit_records(out, k, read, t, width, draw);
}

// Sample the omissions into the output prefix, then describe the included
// runs with the same packed records. A run's urn color is the number of
// omissions preceding it. Empty runs need no record; singleton runs need one.
template<unsigned GroupSize = default_group_size, class Word, class Draw>
void complement(Word n, Word k, Word* out, Draw& draw) {
    const Word omitted = n - k, t = omitted + 1;
    assert(omitted > 0 && omitted <= k / 4);
    sparse<GroupSize>(n, omitted, out, draw);
    Word read = k, high = n;
    for (Word b = omitted + 1; b-- > 0;) {
        const Word low = b == 0 ? 0 : out[b - 1] + 1;
        const Word count = high - low;
        high = b == 0 ? 0 : low - 1;
        if (count == 1) out[--read] = b;
        else if (count > 1) {
            read -= 2;
            out[read] = t + (count - 2);
            out[read + 1] = b;
        }
        assert(read >= b);
    }
    emit_records(out, k, read, t, Word{1}, draw);
}

// Draw must return an independent uniform integer in [0,bound). Word must
// hold n; the caller supplies k writable elements. No temporary allocation.
template<bool RandomOrder = false, unsigned GroupSize = default_group_size, class Word, class Draw>
void choose(Word n, Word k, Word* out, Draw&& draw) {
    static_assert(std::is_unsigned_v<Word> && std::is_integral_v<Word>);
    assert(k <= n);
    if (k == 0) return;
    if (k == 1) { out[0] = n == 1 ? 0 : draw(n); return; }
    if (k == 2 && n > 2) {
        const Word a = draw(n), r = draw(n - 1);
        const Word b = r + (r >= a);
        out[0] = RandomOrder ? a : std::min(a, b);
        out[1] = RandomOrder ? b : std::max(a, b);
        return;
    }
    if constexpr (!RandomOrder) {
        if (n - k > 1) {
            if (n <= 512) {
                local_sample(out, Word{0}, k, Word{0}, Word(n - k + 1), draw);
                return;
            }
            if (n - k <= k / 4 && (n - k <= 1024 || n - k <= k / 16)) {
                complement<GroupSize>(n, k, out, draw);
                return;
            }
            // A bounded local sample avoids grouping and a second set of
            // random draws for tiny samples, including small dense samples.
            if (k <= small_sort) {
                local_sample(out, Word{0}, k, Word{0}, Word(n - k + 1), draw);
                return;
            }
        }
    }
    if (n - k <= k) {
        if constexpr (RandomOrder) {
            // Dense Algorithm R. Reading out[i] before writing it would be UB
            // for an uninitialized caller buffer when the shuffle chooses i.
            out[0] = 0;
            for (Word i = 1; i < k; ++i) {
                out[i] = i;
                std::swap(out[i], out[draw(i + 1)]);
            }
            for (Word i = k; i < n; ++i) {
                const Word r = draw(i + 1);
                if (r < k) out[r] = i;
            }
        } else {
            if (n - k <= 1) {
                const Word omitted = n == k ? k : draw(n);
                for (Word i = 0; i < omitted; ++i) out[i] = i;
                for (Word i = omitted; i < k; ++i) out[i] = i + 1;
                return;
            }
            Word written = 0;
            for (Word value = 0; value < n; ++value) {
                if (draw(n - value) < k - written) {
                    out[written++] = value;
                }
            }
        }
        return;
    }
    sparse<GroupSize>(n, k, out, draw);
    if constexpr (RandomOrder) {
        for (Word i = 1; i < k; ++i) std::swap(out[i], out[draw(i + 1)]);
    }
}

}  // namespace groupedchoose

extern "C" {
void sorted_groupedchoose(uint32_t n, uint32_t k, uint32_t* out);
void random_groupedchoose(uint32_t n, uint32_t k, uint32_t* out);
void sorted_groupedchoose64(uint64_t n, uint64_t k, uint64_t* out);
void random_groupedchoose64(uint64_t n, uint64_t k, uint64_t* out);
}
