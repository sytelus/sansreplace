// Same algorithm as cpp/cardchoose.cpp. Leaf cutoffs differ because the
// standard-library sorts and bounds-checking costs differ.
const SMALL_SORT: usize = 256;
const SMALL_COMPLEMENT: usize = 1024;

const RADIX_TILE: usize = 16384;

// Bounded scratch avoids a k-sized allocation. Histogram all needed bytes in
// one scan, then use sequential reads within a small contiguous bucket.
fn tile_sort<const PASSES: usize, const CAPACITY: usize>(data: &mut [u32]) {
    let mut scratch = [0u32; CAPACITY];
    let mut hist = [[0u32; 256]; PASSES];
    for &value in data.iter() {
        for (p, counts) in hist.iter_mut().enumerate() {
            counts[((value >> (8 * p)) & 255) as usize] += 1;
        }
    }
    for counts in &mut hist {
        let mut sum = 0;
        for count in counts {
            let old = *count;
            *count = sum;
            sum += old;
        }
    }
    // Alternate the two slices without unsafe code or heap allocation.
    let mut source: &mut [u32] = data;
    let mut dest = &mut scratch[..source.len()];
    for (p, counts) in hist.iter_mut().enumerate() {
        for &value in source.iter() {
            let offset = &mut counts[((value >> (8 * p)) & 255) as usize];
            dest[*offset as usize] = value;
            *offset += 1;
        }
        std::mem::swap(&mut source, &mut dest);
    }
    if PASSES % 2 == 1 {
        dest.copy_from_slice(source);
    }
}

// Four byte levels at most. Each active level uses two 1 KiB tables, and
// descendants process contiguous buckets. There are no heap allocations.
fn radix_sort(data: &mut [u32], bits: u32) {
    if data.len() <= SMALL_SORT {
        data.sort_unstable();
        return;
    }
    if data.len() <= RADIX_TILE && bits > 8 {
        if data.len() <= 512 {
            match bits.div_ceil(8) {
                2 => tile_sort::<2, 512>(data),
                3 => tile_sort::<3, 512>(data),
                4 => tile_sort::<4, 512>(data),
                _ => unreachable!(),
            }
        } else if data.len() <= 2048 {
            match bits.div_ceil(8) {
                2 => tile_sort::<2, 2048>(data),
                3 => tile_sort::<3, 2048>(data),
                4 => tile_sort::<4, 2048>(data),
                _ => unreachable!(),
            }
        } else {
            match bits.div_ceil(8) {
                2 => tile_sort::<2, 16384>(data),
                3 => tile_sort::<3, 16384>(data),
                4 => tile_sort::<4, 16384>(data),
                _ => unreachable!(),
            }
        }
        return;
    }
    // Sparse last-digit counts cost more to emit than a tiny comparison sort.
    // This extra comparison-sort case is still bounded to fewer than 1024 items.
    if bits <= 8 && data.len() < (4usize << bits) {
        data.sort_unstable();
        return;
    }
    let digit_bits = bits.min(8);
    let shift = bits - digit_bits;
    let buckets = 1usize << digit_bits;
    let mask = buckets as u32 - 1;
    let mut end = [0u32; 256];
    for &value in data.iter() {
        end[((value >> shift) & mask) as usize] += 1;
    }
    if shift == 0 {
        let prefix = data[0] & !mask;
        let mut rest = data;
        for (b, &count) in end[..buckets].iter().enumerate() {
            let (bucket, tail) = rest.split_at_mut(count as usize);
            bucket.fill(prefix + b as u32);
            rest = tail;
        }
        return;
    }
    let mut next = [0u32; 256];
    let mut sum = 0;
    for (start, end) in next[..buckets].iter_mut().zip(end[..buckets].iter_mut()) {
        *start = sum;
        sum += *end;
        *end = sum;
    }
    for b in 0..buckets - 1 {
        while next[b] < end[b] {
            let mut value = data[next[b] as usize];
            let mut dest = ((value >> shift) & mask) as usize;
            while dest != b {
                value = std::mem::replace(&mut data[next[dest] as usize], value);
                next[dest] += 1;
                dest = ((value >> shift) & mask) as usize;
            }
            data[next[b] as usize] = value;
            next[b] += 1;
        }
    }
    let mut start = 0;
    let mut rest = data;
    for &end in &end[..buckets] {
        let count = (end - start) as usize;
        let (bucket, tail) = rest.split_at_mut(count);
        if count > 1 {
            radix_sort(bucket, shift);
        }
        rest = tail;
        start = end;
    }
}

#[inline]
fn sorted_urn<D: FnMut(u32) -> u32>(draw: &mut D, length: u32, res: &mut [u32]) {
    if res.is_empty() {
        return; // length - amount + 1 would overflow for (u32::MAX, 0).
    }
    let t = length - res.len() as u32 + 1;
    for i in 0..res.len() {
        let r = draw(t + i as u32);
        res[i] = if r < t { r } else { res[(r - t) as usize] };
    }
    if res.len() <= SMALL_SORT {
        res.sort_unstable();
    } else {
        let bits = 32 - (t - 1).max(1).leading_zeros();
        radix_sort(res, bits);
    }
}

// Algorithm S scans n values, so use it only when n <= 2*k. This removes
// sorting and random urn reads for dense samples without changing the bound.
fn dense_choose<const SHUFFLE: bool, D: FnMut(u32) -> u32>(
    draw: &mut D,
    length: u32,
    res: &mut [u32],
) {
    let mut written = 0;
    for value in 0..length {
        if draw(length - value) < (res.len() - written) as u32 {
            res[written] = value;
            if SHUFFLE && written != 0 {
                res.swap(written, draw(written as u32 + 1) as usize);
            }
            written += 1;
        }
    }
}

// Fixed storage for tiny complements preserves the output-only scaling.
fn choose_edges<D: FnMut(u32) -> u32>(draw: &mut D, length: u32, res: &mut [u32]) {
    if res.is_empty() {
        return;
    }
    let amount = res.len() as u32;
    if amount == length {
        for (i, value) in res.iter_mut().enumerate() {
            *value = i as u32;
        }
        return;
    }
    let omitted = length - amount;
    if omitted == 1 {
        let excluded = draw(length) as usize;
        let (prefix, suffix) = res.split_at_mut(excluded);
        for (i, value) in prefix.iter_mut().enumerate() {
            *value = i as u32;
        }
        for (i, value) in suffix.iter_mut().enumerate() {
            *value = excluded as u32 + 1 + i as u32;
        }
        return;
    }
    let mut missing = [0u32; SMALL_COMPLEMENT];
    let missing = &mut missing[..omitted as usize];
    sorted_urn(draw, length, missing);
    let mut first = 0;
    let mut rest = res;
    for (j, &value) in missing.iter().enumerate() {
        let excluded = value + j as u32;
        let (run, tail) = rest.split_at_mut((excluded - first) as usize);
        for (i, value) in run.iter_mut().enumerate() {
            *value = first + i as u32;
        }
        rest = tail;
        first = excluded + 1;
    }
    for (i, value) in rest.iter_mut().enumerate() {
        *value = first + i as u32;
    }
}

fn choose<const SHUFFLE: bool, D: FnMut(u32) -> u32>(draw: &mut D, length: u32, res: &mut [u32]) {
    assert!(res.len() <= length as usize);
    if res.len() <= 1 {
        if let Some(value) = res.first_mut() {
            *value = if length == 1 { 0 } else { draw(length) };
        }
        return;
    }
    if res.len() == 2 && length > 2 {
        let a = draw(length);
        let b = draw(length - 1);
        let b = b + u32::from(b >= a);
        res[0] = if SHUFFLE { a } else { a.min(b) };
        res[1] = if SHUFFLE { b } else { a.max(b) };
        return;
    }
    let omitted = length - res.len() as u32;
    if (omitted <= res.len() as u32 / 4 || omitted == 1) && omitted as usize <= SMALL_COMPLEMENT {
        choose_edges(draw, length, res);
        if SHUFFLE {
            for i in 1..res.len() {
                res.swap(i, draw(i as u32 + 1) as usize);
            }
        }
        return;
    }
    if res.len() >= 128 && length - res.len() as u32 <= res.len() as u32 {
        dense_choose::<SHUFFLE, _>(draw, length, res);
        return;
    }
    sorted_urn(draw, length, res);
    if SHUFFLE {
        // Fuse ranking with inside-out Fisher-Yates. Index zero needs no draw.
        for i in 1..res.len() {
            let r = draw(i as u32 + 1) as usize;
            let value = res[i] + i as u32;
            res[i] = res[r];
            res[r] = value;
        }
    } else {
        for (i, value) in res.iter_mut().enumerate() {
            *value += i as u32;
        }
    }
}

pub fn sorted_order<R: rand::RngExt + ?Sized>(rng: &mut R, length: u32, res: &mut [u32]) {
    choose::<false, _>(&mut |bound| rng.random_range(0..bound), length, res);
}

pub fn random_order<R: rand::RngExt + ?Sized>(rng: &mut R, length: u32, res: &mut [u32]) {
    choose::<true, _>(&mut |bound| rng.random_range(0..bound), length, res);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{RngExt, SeedableRng, rngs::SmallRng};
    use std::collections::BTreeMap;

    fn valid(mut data: Vec<u32>, n: u32, sorted: bool) {
        if sorted {
            assert!(data.is_sorted());
        }
        data.sort_unstable();
        assert!(data.windows(2).all(|w| w[0] < w[1]));
        assert!(data.last().is_none_or(|&x| x < n));
    }

    fn exact<const SHUFFLE: bool, const COMPLEMENT: bool>(n: u32, k: u32) {
        let q = n - k;
        let amount = if k == 0 || k == n {
            0
        } else if COMPLEMENT || ((q <= k / 4 || q == 1) && q as usize <= SMALL_COMPLEMENT) {
            q
        } else {
            k
        };
        let pair = !COMPLEMENT && k == 2 && n > 2;
        let mut bounds: Vec<u32> = if pair {
            vec![n, n - 1]
        } else {
            (0..amount).map(|i| n - amount + 1 + i).collect()
        };
        if SHUFFLE && !pair {
            bounds.extend(2..=k);
        }
        let mut draws = vec![0; bounds.len()];
        let mut counts = BTreeMap::<Vec<u32>, u64>::new();
        loop {
            let mut index = 0;
            let mut data = vec![u32::MAX; k as usize];
            let mut draw = |bound| {
                assert_eq!(bound, bounds[index]);
                let r = draws[index];
                index += 1;
                r
            };
            if COMPLEMENT {
                choose_edges(&mut draw, n, &mut data);
                if SHUFFLE {
                    for i in 1..data.len() {
                        data.swap(i, draw(i as u32 + 1) as usize);
                    }
                }
            } else {
                choose::<SHUFFLE, _>(&mut draw, n, &mut data);
            }
            assert_eq!(index, bounds.len());
            valid(data.clone(), n, !SHUFFLE);
            *counts.entry(data).or_default() += 1;
            let mut i = 0;
            while i < draws.len() && draws[i] + 1 == bounds[i] {
                draws[i] = 0;
                i += 1;
            }
            if i == draws.len() {
                break;
            }
            draws[i] += 1;
        }
        let mut expected = (0..k).map(|i| (n - i) as u64).product::<u64>();
        if !SHUFFLE {
            for i in 1..=k {
                expected /= i as u64;
            }
        }
        assert_eq!(counts.len() as u64, expected);
        let first = *counts.values().next().unwrap();
        assert!(counts.values().all(|&count| count == first));
    }

    #[test]
    fn exact_uniformity() {
        for n in 0..=8 {
            for k in 0..=n {
                exact::<false, false>(n, k);
            }
        }
        for n in 0..=6 {
            for k in 0..=n {
                exact::<true, false>(n, k);
            }
        }
    }

    #[test]
    fn complement_exact_uniformity() {
        // Public dispatch needs larger n for complements with multiple values.
        // Forcing the kernel allows exhaustive checking at small sizes.
        for n in 0..=8 {
            for k in 0..=n {
                exact::<false, true>(n, k);
            }
        }
        for n in 0..=6 {
            for k in 0..=n {
                exact::<true, true>(n, k);
            }
        }
    }

    fn exact_dense<const SHUFFLE: bool>(n: u32, k: u32) {
        let factorial = |n: u32| (1..=n as u64).product::<u64>();
        // Each path has product n! (or n!*k! with the interleaved shuffle).
        // Enumerate mixed-radix codes even though the order of bounds varies.
        let paths = factorial(n) * if SHUFFLE { factorial(k) } else { 1 };
        let mut counts = BTreeMap::<Vec<u32>, u64>::new();
        for mut code in 0..paths {
            let mut product = 1u64;
            let mut data = vec![u32::MAX; k as usize];
            dense_choose::<SHUFFLE, _>(
                &mut |bound| {
                    product *= bound as u64;
                    let r = (code % bound as u64) as u32;
                    code /= bound as u64;
                    r
                },
                n,
                &mut data,
            );
            assert_eq!(product, paths);
            assert_eq!(code, 0);
            valid(data.clone(), n, !SHUFFLE);
            *counts.entry(data).or_default() += 1;
        }
        let expected = factorial(n) / factorial(n - k) / if SHUFFLE { 1 } else { factorial(k) };
        assert_eq!(counts.len() as u64, expected);
        let first = *counts.values().next().unwrap();
        assert!(counts.values().all(|&count| count == first));
    }

    #[test]
    fn dense_exact_uniformity() {
        for n in 0..=7 {
            for k in 0..=n {
                exact_dense::<false>(n, k);
            }
        }
        for n in 0..=5 {
            for k in 0..=n {
                exact_dense::<true>(n, k);
            }
        }
    }

    #[test]
    fn radix_matches_standard_sort() {
        let mut rng = SmallRng::seed_from_u64(324);
        for size in [
            0, 1, 2, 63, 64, 127, 128, 129, 255, 256, 257, 1024, 2047, 2048, 2049, 4095, 4096,
            4097, 8191, 8192, 8193, 16383, 16384, 16385, 65536, 1048576,
        ] {
            for bits in [1, 7, 8, 9, 13, 15, 16, 17, 23, 24, 25, 31, 32] {
                let mask = u32::MAX >> (32 - bits);
                for pattern in 0..6 {
                    let mut data: Vec<u32> = (0..size)
                        .map(|_| {
                            let x = rng.random::<u32>() & mask;
                            match pattern {
                                1 => mask,
                                2 => x & 3,
                                3 => x | (mask & 0xffffff00),
                                _ => x,
                            }
                        })
                        .collect();
                    if pattern >= 4 {
                        data.sort_unstable();
                    }
                    if pattern == 5 {
                        data.reverse();
                    }
                    let mut expected = data.clone();
                    expected.sort_unstable();
                    radix_sort(&mut data, bits);
                    assert_eq!(data, expected);
                }
            }
        }
    }

    #[test]
    fn boundaries() {
        let mut rng = SmallRng::seed_from_u64(324);
        for n in [
            0,
            1,
            2,
            64,
            65,
            128,
            129,
            255,
            256,
            257,
            65535,
            65536,
            65537,
            0x7fffffff,
            0x80000000,
            u32::MAX,
        ] {
            for k in [
                0, 1, 2, 63, 64, 65, 127, 128, 129, 255, 256, 257, 1024, 65536,
            ] {
                if k > n {
                    continue;
                }
                for sorted in [true, false] {
                    let mut data = vec![u32::MAX; k as usize];
                    if sorted {
                        sorted_order(&mut rng, n, &mut data);
                    } else {
                        random_order(&mut rng, n, &mut data);
                    }
                    valid(data, n, sorted);
                }
            }
        }
    }

    #[test]
    fn complement_boundaries() {
        let mut rng = SmallRng::seed_from_u64(324);
        for k in [4096, 65536] {
            for omitted in [0, 1, 2, 63, 64, 65, 255, 256, 257, 1023, 1024, 1025] {
                for sorted in [true, false] {
                    let mut data = vec![u32::MAX; k as usize];
                    if sorted {
                        sorted_order(&mut rng, k + omitted, &mut data);
                    } else {
                        random_order(&mut rng, k + omitted, &mut data);
                    }
                    valid(data, k + omitted, sorted);
                }
            }
        }
    }

    #[test]
    #[should_panic]
    fn rejects_k_larger_than_n() {
        sorted_order(&mut SmallRng::seed_from_u64(324), 1, &mut [0; 2]);
    }
}
