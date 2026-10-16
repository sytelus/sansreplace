#include "groupedchoose.h"
#include "randbelow.h"

extern "C" void sorted_groupedchoose(uint32_t n, uint32_t k, uint32_t* out) {
    groupedchoose::choose(n, k, out, [](uint32_t bound) { return randbelow(bound); });
}
extern "C" void random_groupedchoose(uint32_t n, uint32_t k, uint32_t* out) {
    groupedchoose::choose<true>(n, k, out, [](uint32_t bound) { return randbelow(bound); });
}
extern "C" void sorted_groupedchoose64(uint64_t n, uint64_t k, uint64_t* out) {
    groupedchoose::choose(n, k, out, [](uint64_t bound) { return randbelow64(bound); });
}
extern "C" void random_groupedchoose64(uint64_t n, uint64_t k, uint64_t* out) {
    groupedchoose::choose<true>(n, k, out, [](uint64_t bound) { return randbelow64(bound); });
}
