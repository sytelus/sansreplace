# Copyright 2018 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

def small_values():
    top = 11
    for n in range(top):
        for k in range(n+1):
            yield f"n = {n}, k = {k}", n, k

def power10_values():
    ptop = 9
    klim = {}
    for pn in range(ptop):
        for pk in range(pn + 1):
            yield f"n = 10^{pn}, k = 10^{pk}", 10**pn, 10**pk

def power2_values():
    ptop = 30
    klim = {}
    for pn in range(ptop):
        for pk in range(pn + 1):
            yield f"n = 2^{pn}, k = 2^{pk}", 2**pn, 2**pk

def fib(m):
    a, b = 0, 1
    while a < m:
        yield a
        a, b = b, a + b

def fib_values():
    ptop = 1E9
    klim = {}
    for pn, n in enumerate(fib(ptop)):
        for pk, k in enumerate(fib(n + 1)):
            yield f"n = F_{pn} = {n}, k = F_{pk} = {k}", n, k

def largen_values():
    n = 10_000_000
    for k in range(0, n+1, 100_000):
        yield f"n = {n}, k = {k}", n, k

def cache_values():
    """Tiny samples, dispatch edges and working sets crossing common caches.

    These are sizes to measure, not assumptions about the host's cache sizes.
    All population sizes remain within the native implementations' u32 API.
    """
    for k in [0, 1, 2, 4, 8, 16, 32, 63, 64, 65, 127, 128, 129, 256,
              512, 1024, 2048, 4096, 8192, 16384, 65536, 131072, 262144,
              1048576, 8388608, 16777216]:
        for n in sorted({k, k + 1, k + 64, k + 65, k + 1024, k + 1025,
                         k + max(1, k // 100),
                         k + max(1, k // 4), max(1, 2 * k), max(1, 16 * k),
                         2**32 - 1}):
            yield f"n = {n}, k = {k}", n, k

def iter_values(names):
    done = set()
    for name in names:
        for descrip, n, k in globals()[f"{name}_values"]():
            if (n, k) in done:
                print(f"skipping {descrip}, already done")
            else:
                yield descrip, n, k
                done.add((n, k))
