//! Grouped Cardchoose: expected O(k) word operations and output + O(1) words.
//!
//! `sorted_order` / `random_order` use u32, and the `64` variants use u64.
//! All accept a caller-owned output slice, allocate nothing, and use unbiased
//! bounded draws. A fixed 64-byte bitmap and bounded small-sort stack are the
//! only non-scalar workspace. See groupedchoose.md for the probability and storage proofs.

const SMALL_SORT: usize = 1024;
const DEFAULT_GROUP_SIZE: usize = 64;

macro_rules! implementation {
    ($module:ident, $word:ty) => {
        mod $module {
            use super::{DEFAULT_GROUP_SIZE, SMALL_SORT};
            pub(super) fn local_sample(
                out: &mut [$word],
                start: usize,
                count: usize,
                low: $word,
                width: $word,
                draw: &mut impl FnMut($word) -> $word,
            ) {
                let data = &mut out[start..start + count];
                if width == 1 {
                    for (j, slot) in data.iter_mut().enumerate() {
                        *slot = low + (start + j) as $word;
                    }
                    return;
                }
                let domain = width + (count as $word - 1);
                if domain <= 512 {
                    // A fixed bitmap implements Floyd's sampler without a sort.
                    let mut bits = [0u64; 8];
                    let invert = count as $word > domain / 2;
                    let draws = if invert {
                        domain - count as $word
                    } else {
                        count as $word
                    };
                    let runs = invert && draws <= count as $word / 16;
                    for j in domain - draws..domain {
                        let r = draw(j + 1);
                        let v = if bits[(r / 64) as usize] & (1u64 << (r % 64)) != 0 {
                            j
                        } else {
                            r
                        };
                        bits[(v / 64) as usize] |= 1u64 << (v % 64);
                    }
                    let (mut written, mut first) = (0, 0 as $word);
                    let words = 1 + (domain as usize - 1) / 64;
                    for (b, &word) in bits[..words].iter().enumerate() {
                        let mut mask = if invert && !runs { !word } else { word };
                        if b + 1 == words && domain % 64 != 0 {
                            mask &= (1u64 << (domain % 64)) - 1;
                        }
                        while mask != 0 {
                            let value = (64 * b + mask.trailing_zeros() as usize) as $word;
                            if runs {
                                // Emit gaps between omissions as contiguous runs.
                                let end = written + (value - first) as usize;
                                for (j, slot) in data[written..end].iter_mut().enumerate() {
                                    *slot = low + start as $word + first + j as $word;
                                }
                                written = end;
                                first = value + 1;
                            } else {
                                data[written] = low + start as $word + value;
                                written += 1;
                            }
                            mask &= mask - 1;
                        }
                    }
                    if runs {
                        for (j, slot) in data[written..].iter_mut().enumerate() {
                            *slot = low + start as $word + first + j as $word;
                        }
                    }
                    return;
                }
                for j in 0..data.len() {
                    let r = draw(width + j as $word);
                    data[j] = if r < width {
                        low + r
                    } else {
                        data[(r - width) as usize]
                    };
                }
                if count <= SMALL_SORT {
                    data.sort_unstable();
                } else {
                    for i in 1..data.len() {
                        let value = data[i];
                        let mut j = i;
                        while j > 0 && data[j - 1] > value {
                            data[j] = data[j - 1];
                            j -= 1;
                        }
                        data[j] = value;
                    }
                }
                for (j, slot) in data.iter_mut().enumerate() {
                    *slot += (start + j) as $word;
                }
            }

            fn emit_records(
                out: &mut [$word],
                mut read: usize,
                t: $word,
                width: $word,
                draw: &mut impl FnMut($word) -> $word,
            ) {
                let k = out.len();
                let mut start = 0;
                while read < k {
                    let tag = out[read];
                    read += 1;
                    if tag < t {
                        out[start] = tag + start as $word;
                        start += 1;
                    } else {
                        let count = (tag - t + 2) as usize;
                        let b = out[read];
                        read += 1;
                        debug_assert!(start + count <= read);
                        let low = b * width;
                        local_sample(out, start, count, low, width.min(t - low), draw);
                        start += count;
                    }
                }
                debug_assert_eq!(start, k);
            }

            pub(super) fn sparse<const GROUP_SIZE: usize>(
                n: $word,
                out: &mut [$word],
                draw: &mut impl FnMut($word) -> $word,
            ) {
                assert!(GROUP_SIZE >= 3);
                let k = out.len();
                if k == 0 {
                    return;
                }
                let t = n - k as $word + 1;
                if k < 4 || (GROUP_SIZE == DEFAULT_GROUP_SIZE && k <= SMALL_SORT) {
                    local_sample(out, 0, k, 0, t, draw);
                    return;
                }
                let wanted = t.min((k / 4).min(1 + (k - 1) / GROUP_SIZE) as $word);
                let shift = <$word>::BITS - 1 - (t / wanted).leading_zeros();
                let width = (1 as $word) << shift;
                let groups = 1 + ((t - 1) >> shift);
                if groups == 1 {
                    local_sample(out, 0, k, 0, t, draw);
                    return;
                }
                debug_assert!(groups as usize <= k / 2);
                // Power-of-two intervals need only a shift per fresh draw.
                // The final interval is the only one that may be shorter.
                for i in 0..k {
                    let r = draw(t + i as $word);
                    out[i] = if r < t {
                        1 + (r >> shift)
                    } else {
                        out[(r - t) as usize]
                    };
                }
                // In-place histogram: labels 1..=groups; count m is groups+m.
                let mut i = 0;
                while i < k {
                    if out[i] == 0 || out[i] > groups {
                        i += 1;
                        continue;
                    }
                    let b = (out[i] - 1) as usize;
                    if out[b] != 0 && out[b] <= groups {
                        out[i] = out[b];
                        out[b] = groups + 1;
                    } else {
                        out[b] = if out[b] == 0 { groups + 1 } else { out[b] + 1 };
                        out[i] = 0;
                        i += 1;
                    }
                }
                let mut read = k;
                for b in (0..groups).rev() {
                    let count = if out[b as usize] == 0 {
                        0
                    } else {
                        out[b as usize] - groups
                    };
                    if count == 1 {
                        read -= 1;
                        let low = b * width;
                        out[read] = low + draw(width.min(t - low));
                    } else if count > 1 {
                        read -= 2;
                        out[read] = t + (count - 2);
                        out[read + 1] = b;
                    }
                    debug_assert!(read >= b as usize);
                }
                emit_records(out, read, t, width, draw);
            }

            pub(super) fn complement<const GROUP_SIZE: usize>(
                n: $word,
                out: &mut [$word],
                draw: &mut impl FnMut($word) -> $word,
            ) {
                let k = out.len();
                let omitted = n - k as $word;
                assert!(omitted > 0 && omitted <= k as $word / 4);
                sparse::<GROUP_SIZE>(n, &mut out[..omitted as usize], draw);
                let t = omitted + 1;
                let (mut read, mut high) = (k, n);
                for b in (0..=omitted).rev() {
                    let low = if b == 0 { 0 } else { out[b as usize - 1] + 1 };
                    let count = high - low;
                    high = if b == 0 { 0 } else { low - 1 };
                    if count == 1 {
                        read -= 1;
                        out[read] = b;
                    } else if count > 1 {
                        read -= 2;
                        out[read] = t + (count - 2);
                        out[read + 1] = b;
                    }
                    debug_assert!(read >= b as usize);
                }
                emit_records(out, read, t, 1, draw);
            }

            pub(super) fn choose<const RANDOM: bool, const GROUP_SIZE: usize>(
                n: $word,
                out: &mut [$word],
                draw: &mut impl FnMut($word) -> $word,
            ) {
                let k = <$word>::try_from(out.len()).expect("sample length exceeds word size");
                assert!(k <= n, "require k <= n");
                if k == 0 {
                    return;
                }
                if k == 1 {
                    out[0] = if n == 1 { 0 } else { draw(n) };
                    return;
                }
                if k == 2 && n > 2 {
                    let a = draw(n);
                    let r = draw(n - 1);
                    let b = r + <$word>::from(r >= a);
                    out[0] = if RANDOM { a } else { a.min(b) };
                    out[1] = if RANDOM { b } else { a.max(b) };
                    return;
                }
                if !RANDOM && n - k == 2 {
                    let a = draw(n);
                    let r = draw(n - 1);
                    let b = r + <$word>::from(r >= a);
                    let (first, second) = (a.min(b), a.max(b));
                    let (prefix, rest) = out.split_at_mut(first as usize);
                    let (middle, suffix) = rest.split_at_mut((second - first - 1) as usize);
                    for (i, slot) in prefix.iter_mut().enumerate() {
                        *slot = i as $word;
                    }
                    for (i, slot) in middle.iter_mut().enumerate() {
                        *slot = first + 1 + i as $word;
                    }
                    for (i, slot) in suffix.iter_mut().enumerate() {
                        *slot = second + 1 + i as $word;
                    }
                    return;
                }
                if !RANDOM && n - k > 1 {
                    if n <= 512 {
                        local_sample(out, 0, k as usize, 0, n - k + 1, draw);
                        return;
                    }
                    if n - k <= k / 4 && (n - k <= 1024 || n - k <= k / 16) {
                        complement::<GROUP_SIZE>(n, out, draw);
                        return;
                    }
                    if k <= SMALL_SORT as $word {
                        local_sample(out, 0, k as usize, 0, n - k + 1, draw);
                        return;
                    }
                }
                if n - k <= k {
                    if RANDOM {
                        out[0] = 0;
                        for i in 1..out.len() {
                            out[i] = i as $word;
                            out.swap(i, draw(i as $word + 1) as usize);
                        }
                        for i in k..n {
                            let r = draw(i + 1);
                            if r < k {
                                out[r as usize] = i;
                            }
                        }
                    } else {
                        if n - k <= 1 {
                            let omitted = if n == k { k } else { draw(n) };
                            let (prefix, suffix) = out.split_at_mut(omitted as usize);
                            for (i, slot) in prefix.iter_mut().enumerate() {
                                *slot = i as $word;
                            }
                            for (i, slot) in suffix.iter_mut().enumerate() {
                                *slot = omitted + 1 + i as $word;
                            }
                            return;
                        }
                        let mut written = 0;
                        for value in 0..n {
                            if draw(n - value) < k - written {
                                out[written as usize] = value;
                                written += 1;
                            }
                        }
                    }
                    return;
                }
                sparse::<GROUP_SIZE>(n, out, draw);
                if RANDOM {
                    for i in 1..out.len() {
                        out.swap(i, draw(i as $word + 1) as usize);
                    }
                }
            }
        }
    };
}

implementation!(word32, u32);
implementation!(word64, u64);

/// Uniform sorted sample from 0..n. Panics if the output is longer than n.
pub fn sorted_order<R: rand::RngExt + ?Sized>(rng: &mut R, n: u32, out: &mut [u32]) {
    word32::choose::<false, DEFAULT_GROUP_SIZE>(n, out, &mut |b| rng.random_range(0..b));
}
/// Uniform ordered sample from 0..n. Panics if the output is longer than n.
pub fn random_order<R: rand::RngExt + ?Sized>(rng: &mut R, n: u32, out: &mut [u32]) {
    word32::choose::<true, DEFAULT_GROUP_SIZE>(n, out, &mut |b| rng.random_range(0..b));
}
/// Uniform sorted sample with a 64-bit population and 64-bit output elements.
pub fn sorted_order64<R: rand::RngExt + ?Sized>(rng: &mut R, n: u64, out: &mut [u64]) {
    word64::choose::<false, DEFAULT_GROUP_SIZE>(n, out, &mut |b| rng.random_range(0..b));
}
/// Uniform ordered sample with a 64-bit population and 64-bit output elements.
pub fn random_order64<R: rand::RngExt + ?Sized>(rng: &mut R, n: u64, out: &mut [u64]) {
    word64::choose::<true, DEFAULT_GROUP_SIZE>(n, out, &mut |b| rng.random_range(0..b));
}

#[cfg(test)]
#[path = "groupedchoose_tests.rs"]
mod tests;
