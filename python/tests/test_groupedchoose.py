"""Exact probability tests, including adaptive draw bounds and packed records."""
import importlib.util
from collections import defaultdict
from fractions import Fraction
from math import comb, perm
from pathlib import Path
import random
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from algorithms import groupedchoose as sampler


def exact(function, n, k, ordered=False):
    probabilities = defaultdict(Fraction)
    script = []
    leaves = 0

    def visit(probability):
        nonlocal leaves
        cursor, next_bound = 0, None

        def draw(bound):
            nonlocal cursor, next_bound
            assert bound > 0
            if cursor == len(script):
                if next_bound is None:
                    next_bound = bound
                return 0
            value = script[cursor]
            cursor += 1
            assert value < bound
            return value

        out = function(n, k, draw)
        if next_bound is not None:
            for value in range(next_bound):
                script.append(value)
                visit(probability / next_bound)
                script.pop()
        else:
            assert len(out) == k and len(set(out)) == k
            assert all(0 <= x < n for x in out)
            assert ordered or out == sorted(out)
            probabilities[tuple(out)] += probability
            leaves += 1

    visit(Fraction(1))
    total = perm(n, k) if ordered else comb(n, k)
    assert len(probabilities) == total, (n, k, ordered)
    assert set(probabilities.values()) == {Fraction(1, total)}, (n, k, ordered)
    return leaves


def public(function):
    def run(n, k, draw):
        with patch.object(sampler.random, 'randrange', draw):
            return function(n, k)
    return run


class GroupedChooseTests(unittest.TestCase):
    def test_self_contained_function(self):
        path = Path(__file__).resolve().parents[2] / 'examples/groupedchoose.py'
        spec = importlib.util.spec_from_file_location('standalone_groupedchoose', path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        for n in range(7):
            for k in range(n + 1):
                exact(public(module.sorted_choose), n, k)
        # The complete sparse algorithm must match production draw-for-draw.
        for n in (1000, 2**32 + 1, 2**64 + 1, 10**100, 2**4096 + 123):
            for k in (0, 1, 2, 15, 16, 17, 32, 33, 127, 256, 4096):
                if n <= 2*k:
                    continue
                for seed in (0, 1, 324):
                    got = public(module.sorted_choose)(n, k, random.Random(seed).randrange)
                    expected = public(sampler.sorted_choose)(n, k, random.Random(seed).randrange)
                    self.assertEqual(got, expected)

    def test_exact_probabilities(self):
        leaves = 0
        for n in range(8):
            for k in range(n + 1):
                leaves += exact(public(sampler.sorted_choose), n, k)
                # Force multiple groups at feasible enumeration sizes. Draw
                # bounds depend on occupancies: counting leaves is NOT enough.
                leaves += exact(lambda n, k, draw: sampler._sparse(n, k, draw, 3), n, k)
        for n in range(6):
            for k in range(n + 1):
                leaves += exact(public(sampler.random_choose), n, k, ordered=True)
        for n in range(5, 17):
            for omitted in range(1, min(3, n // 5) + 1):
                leaves += exact(sampler._complement, n, n - omitted)
        print(f'Exact uniformity: {leaves:,} rationally weighted leaves', flush=True)

    def test_boundaries_and_arbitrary_precision(self):
        for n in (0, 1, 7, 129, 1000, 2**32 - 1, 2**32, 2**64 - 1,
                  2**64, 10**100, 2**4096 + 123):
            for k in (0, 1, 2, 3, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 1024):
                if k > n:
                    continue
                for function in (sampler.sorted_choose, sampler.random_choose):
                    out = public(function)(n, k, random.Random(324).randrange)
                    self.assertEqual(len(out), k)
                    self.assertEqual(len(set(out)), k)
                    self.assertTrue(all(0 <= x < n for x in out))
                    if function is sampler.sorted_choose:
                        self.assertEqual(out, sorted(out))
        for n, k in ((-1, 0), (0, -1), (1, 2)):
            for function in (sampler.sorted_choose, sampler.random_choose):
                with self.assertRaises(ValueError):
                    function(n, k)

    def test_packed_records_and_extreme_occupancies(self):
        for seed in range(150):
            rng = random.Random(seed)
            n = rng.randrange(1, 1 << rng.randrange(1, 257))
            k = rng.randrange(min(n, 4096) + 1)
            for size in (3, 8, 16, 32):
                # Force sparse to stress singleton/empty/large groups even at
                # dense sizes, beyond the public dispatch's requirements.
                out = sampler._sparse(n, k, rng.randrange, size)
                self.assertEqual(len(out), k)
                self.assertTrue(all(0 <= x < n for x in out))
                self.assertTrue(all(a < b for a, b in zip(out, out[1:])))
        for n in (1000, 2**32 - 1, 2**64 - 1, 2**4096 + 123):
            for draw in (lambda b: 0, lambda b: b - 1):
                out = sampler._sparse(n, 127, draw, 3)
                self.assertEqual(len(out), 127)
                self.assertTrue(all(a < b for a, b in zip(out, out[1:])))
                self.assertTrue(all(0 <= x < n for x in out))

    def test_complement_boundaries(self):
        for k in (8, 64, 256, 1024, 65536):
            for omitted in (2, 3, k//4 - 1, k//4, k//4 + 1):
                n = k + omitted
                for draw in (lambda b: 0, lambda b: b - 1, random.Random(324).randrange):
                    out = public(sampler.sorted_choose)(n, k, draw)
                    self.assertEqual(len(out), k)
                    self.assertTrue(all(0 <= x < n for x in out))
                    self.assertTrue(all(a < b for a, b in zip(out, out[1:])))


if __name__ == '__main__':
    unittest.main()
