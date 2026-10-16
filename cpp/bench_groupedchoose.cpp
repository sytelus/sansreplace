// Fixed-iteration driver used by python/bench_groupedchoose. Build once before
// timing; allocation, first touch and one warm-up call are outside the clock.
#include "groupedchoose.h"
#include "randbelow.h"

#include <atomic>
#include <chrono>
#include <iomanip>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>

extern "C" void sorted_cardchoose(uint32_t, uint32_t, uint32_t*);
extern "C" void random_cardchoose(uint32_t, uint32_t, uint32_t*);
extern "C" void sorted_reservoirsample(uint32_t, uint32_t, uint32_t*);
extern "C" void random_reservoirsample(uint32_t, uint32_t, uint32_t*);

template<class Word> Word draw(Word bound) {
    if constexpr (sizeof(Word) == 4) return randbelow(bound);
    else return randbelow64(bound);
}

template<class Word, bool Random, unsigned GroupSize = 16>
void grouped(Word n, Word k, Word* out) {
    groupedchoose::choose<Random, GroupSize>(n, k, out, draw<Word>);
}

template<class Word, bool Random>
void urnsort(Word n, Word k, Word* out) {
    if (k == 0) return;
    const Word t = n - k + 1;
    for (Word i = 0; i < k; ++i) {
        const Word r = draw(Word(t + i));
        out[i] = r < t ? r : out[r - t];
    }
    std::sort(out, out + k);
    for (Word i = 0; i < k; ++i) out[i] += i;
    if constexpr (Random) {
        for (Word i = 1; i < k; ++i) std::swap(out[i], out[draw(Word(i + 1))]);
    }
}

template<class Word, bool Random>
void reservoir(Word n, Word k, Word* out) {
    if (k == 0) return;
    for (Word i = 0; i < k; ++i) {
        out[i] = i;
        if constexpr (Random) std::swap(out[i], out[draw(Word(i + 1))]);
    }
    for (Word i = k; i < n; ++i) {
        const Word r = draw(Word(i + 1));
        if (r < k) out[r] = i;
    }
    if constexpr (!Random) std::sort(out, out + k);
}

template<class Word, bool Random>
auto function(const std::string& name) -> void (*)(Word, Word, Word*) {
    if (name == "groupedchoose") return grouped<Word, Random>;
    if (name == "grouped8") return grouped<Word, Random, 8>;
    if (name == "grouped16") return grouped<Word, Random, 16>;
    if (name == "grouped32") return grouped<Word, Random, 32>;
    if (name == "grouped64") return grouped<Word, Random, 64>;
    if (name == "urnsort") return urnsort<Word, Random>;
    if constexpr (sizeof(Word) == 4) {
        if (name == "cardchoose") return Random ? random_cardchoose : sorted_cardchoose;
        if (name == "reservoirsample") return Random ? random_reservoirsample : sorted_reservoirsample;
    } else {
        if (name == "reservoirsample") return reservoir<Word, Random>;
    }
    throw std::invalid_argument("unknown algorithm for this width");
}

template<class Word>
void run(const std::string& algorithm, bool random, uint64_t n, uint64_t k, uint64_t iters) {
    if (n > std::numeric_limits<Word>::max() || k > n || iters == 0 ||
        k > std::numeric_limits<size_t>::max() / sizeof(Word)) {
        throw std::invalid_argument("invalid n, k or iterations");
    }
    const auto f = random ? function<Word, true>(algorithm) : function<Word, false>(algorithm);
    std::vector<Word> out(static_cast<size_t>(k));
    rand_init();
    f(Word(n), Word(k), out.data());
    rand_init();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < iters; ++i) {
        f(Word(n), Word(k), out.data());
        std::atomic_signal_fence(std::memory_order_seq_cst);
    }
    const double seconds = std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
    const Word checksum = out.empty() ? 0 : out.front() ^ out.back();
    std::cout << std::setprecision(17) << "{\"seconds\":" << seconds
              << ",\"checksum\":" << checksum << "}\n";
}

int main(int argc, char** argv) {
    try {
        if (argc != 7) throw std::invalid_argument("usage: bench_groupedchoose WIDTH ALGORITHM ORDER N K ITERS");
        const std::string width = argv[1], order = argv[3];
        if (order != "sorted" && order != "random") throw std::invalid_argument("invalid order");
        for (int i : {4, 5, 6}) {
            if (std::string(argv[i]).find_first_not_of("0123456789") != std::string::npos || !*argv[i])
                throw std::invalid_argument("n, k and iterations must be unsigned integers");
        }
        const auto n = std::stoull(argv[4]), k = std::stoull(argv[5]), iters = std::stoull(argv[6]);
        if (width == "32") run<uint32_t>(argv[2], order == "random", n, k, iters);
        else if (width == "64") run<uint64_t>(argv[2], order == "random", n, k, iters);
        else throw std::invalid_argument("width must be 32 or 64");
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
