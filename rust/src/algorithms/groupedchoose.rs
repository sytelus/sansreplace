//! Grouped Cardchoose: expected O(k) word operations and output + O(1) words.
//!
//! `sorted_order` / `random_order` use u32, and the `64` variants use u64.
//! All accept a caller-owned output slice, allocate nothing, and use unbiased
//! bounded draws. See groupedchoose.md for the probability and storage proofs.

macro_rules! implementation {
    ($module:ident, $word:ty) => {
        mod $module {
            fn local_urn(
                out: &mut [$word],
                start: usize,
                count: usize,
                low: $word,
                width: $word,
                draw: &mut impl FnMut($word) -> $word,
            ) {
                let stop = start + count;
                if width == 1 {
                    for (i, slot) in out.iter_mut().enumerate().take(stop).skip(start) {
                        *slot = low + i as $word;
                    }
                    return;
                }
                for j in 0..count {
                    let r = draw(width + j as $word);
                    out[start + j] = if r < width {
                        low + r
                    } else {
                        out[start + (r - width) as usize]
                    };
                }
                for i in start + 1..stop {
                    let value = out[i];
                    let mut j = i;
                    while j > start && out[j - 1] > value {
                        out[j] = out[j - 1];
                        j -= 1;
                    }
                    out[j] = value;
                }
                for (i, slot) in out.iter_mut().enumerate().take(stop).skip(start) {
                    *slot += i as $word;
                }
            }

            fn emit_records(
                out: &mut [$word],
                mut read: usize,
                t: $word,
                width: $word,
                extra: $word,
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
                        local_urn(
                            out,
                            start,
                            count,
                            b * width + b.min(extra),
                            width + <$word>::from(b < extra),
                            draw,
                        );
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
                let groups = t.min((1 + (k - 1) / GROUP_SIZE) as $word);
                if groups == 1 {
                    local_urn(out, 0, k, 0, t, draw);
                    return;
                }
                debug_assert!(groups as usize <= k / 2);
                let (width, extra) = (t / groups, t % groups);
                let split = (width + 1) * extra;
                for i in 0..k {
                    let r = draw(t + i as $word);
                    out[i] = if r < t {
                        1 + if r < split {
                            r / (width + 1)
                        } else {
                            extra + (r - split) / width
                        }
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
                        out[read] =
                            b * width + b.min(extra) + draw(width + <$word>::from(b < extra));
                    } else if count > 1 {
                        read -= 2;
                        out[read] = t + (count - 2);
                        out[read + 1] = b;
                    }
                    debug_assert!(read >= b as usize);
                }
                emit_records(out, read, t, width, extra, draw);
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
                emit_records(out, read, t, 1, 0, draw);
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
                            let omitted = if n == k { n } else { draw(n) };
                            for (i, slot) in out.iter_mut().enumerate() {
                                *slot = i as $word + <$word>::from(i as $word >= omitted);
                            }
                            return;
                        }
                        if n - k <= k / 4 {
                            complement::<GROUP_SIZE>(n, out, draw);
                            return;
                        }
                        let mut written = 0;
                        for value in 0..n {
                            let remaining = n - value;
                            if k - written == remaining {
                                for (i, slot) in out.iter_mut().enumerate().skip(written as usize) {
                                    *slot = value + (i as $word - written);
                                }
                                break;
                            }
                            if draw(remaining) < k - written {
                                out[written as usize] = value;
                                written += 1;
                                if written == k {
                                    break;
                                }
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
    word32::choose::<false, 16>(n, out, &mut |b| rng.random_range(0..b));
}
/// Uniform ordered sample from 0..n. Panics if the output is longer than n.
pub fn random_order<R: rand::RngExt + ?Sized>(rng: &mut R, n: u32, out: &mut [u32]) {
    word32::choose::<true, 16>(n, out, &mut |b| rng.random_range(0..b));
}
/// Uniform sorted sample with a 64-bit population and 64-bit output elements.
pub fn sorted_order64<R: rand::RngExt + ?Sized>(rng: &mut R, n: u64, out: &mut [u64]) {
    word64::choose::<false, 16>(n, out, &mut |b| rng.random_range(0..b));
}
/// Uniform ordered sample with a 64-bit population and 64-bit output elements.
pub fn random_order64<R: rand::RngExt + ?Sized>(rng: &mut R, n: u64, out: &mut [u64]) {
    word64::choose::<true, 16>(n, out, &mut |b| rng.random_range(0..b));
}

#[cfg(test)]
#[path = "groupedchoose_tests.rs"]
mod tests;
