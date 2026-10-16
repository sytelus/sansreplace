use super::*;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Default)]
struct Fraction(u128, u128);

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Fraction {
    fn divided(self, bound: u64) -> Self {
        let g = gcd(self.0, u128::from(bound));
        Self(
            self.0 / g,
            self.1.checked_mul(u128::from(bound) / g).unwrap(),
        )
    }

    fn add(&mut self, other: Self) {
        if self.0 == 0 {
            *self = other;
            return;
        }
        let g = gcd(self.1, other.1);
        let n = self
            .0
            .checked_mul(other.1 / g)
            .unwrap()
            .checked_add(other.0.checked_mul(self.1 / g).unwrap())
            .unwrap();
        let d = self.1.checked_mul(other.1 / g).unwrap();
        let reduce = gcd(n, d);
        *self = Self(n / reduce, d / reduce);
    }
}

fn visit(
    n: u64,
    k: usize,
    mode: u8,
    script: &mut Vec<u64>,
    probability: Fraction,
    probabilities: &mut BTreeMap<Vec<u64>, Fraction>,
) {
    let (mut cursor, mut next) = (0, 0);
    let mut draw = |bound| {
        assert!(bound > 0);
        if cursor == script.len() {
            if next == 0 {
                next = bound;
            }
            return 0;
        }
        let r = script[cursor];
        cursor += 1;
        assert!(r < bound);
        r
    };
    let mut out = vec![u64::MAX; k];
    match mode {
        0 => word64::choose::<false, 3>(n, &mut out, &mut draw),
        1 => word64::sparse::<3>(n, &mut out, &mut draw),
        2 => word64::choose::<true, 3>(n, &mut out, &mut draw),
        3 => word64::complement::<3>(n, &mut out, &mut draw),
        _ => unreachable!(),
    }
    // The two word widths must make exactly the same decisions for the same
    // scripted bounded draws, including the paths not yet fully enumerated.
    let mut cursor32 = 0;
    let mut draw32 = |bound| {
        if cursor32 == script.len() {
            return 0;
        }
        let r = script[cursor32] as u32;
        cursor32 += 1;
        assert!(r < bound);
        r
    };
    let mut out32 = vec![u32::MAX; k];
    match mode {
        0 => word32::choose::<false, 3>(n as u32, &mut out32, &mut draw32),
        1 => word32::sparse::<3>(n as u32, &mut out32, &mut draw32),
        2 => word32::choose::<true, 3>(n as u32, &mut out32, &mut draw32),
        3 => word32::complement::<3>(n as u32, &mut out32, &mut draw32),
        _ => unreachable!(),
    }
    assert!(out.iter().copied().eq(out32.iter().copied().map(u64::from)));
    if next != 0 {
        for r in 0..next {
            script.push(r);
            visit(n, k, mode, script, probability.divided(next), probabilities);
            script.pop();
        }
    } else {
        let mut sorted = out.clone();
        sorted.sort_unstable();
        assert!(sorted.iter().all(|&x| x < n));
        assert!(sorted.windows(2).all(|w| w[0] < w[1]));
        if mode != 2 {
            assert_eq!(out, sorted);
        }
        probabilities.entry(out).or_default().add(probability);
    }
}

#[test]
fn exact_probabilities() {
    for mode in 0..3 {
        for n in 0..=if mode == 2 { 5 } else { 7 } {
            for k in 0..=n as usize {
                let mut probabilities = BTreeMap::new();
                visit(
                    n,
                    k,
                    mode,
                    &mut Vec::new(),
                    Fraction(1, 1),
                    &mut probabilities,
                );
                let mut count = 1u128;
                for i in 0..k {
                    count *= u128::from(n - i as u64);
                }
                if mode != 2 {
                    for i in 1..=k {
                        count /= i as u128;
                    }
                }
                assert_eq!(probabilities.len() as u128, count);
                for p in probabilities.values() {
                    assert_eq!((p.0, p.1), (1, count));
                }
            }
        }
    }
    for n in 5..=16 {
        for q in 1..=3.min(n / 5) {
            let mut probabilities = BTreeMap::new();
            visit(
                n,
                (n - q) as usize,
                3,
                &mut Vec::new(),
                Fraction(1, 1),
                &mut probabilities,
            );
            let mut count = 1u128;
            for i in 0..q {
                count *= u128::from(n - i);
            }
            for i in 1..=q {
                count /= u128::from(i);
            }
            assert_eq!(probabilities.len() as u128, count);
            for p in probabilities.values() {
                assert_eq!((p.0, p.1), (1, count));
            }
        }
    }
}

#[test]
fn boundaries_and_packing() {
    let mut rng = SmallRng::seed_from_u64(324);
    for n in [
        0,
        1,
        2,
        65,
        129,
        255,
        65535,
        65536,
        65537,
        u64::from(u32::MAX),
        1 << 32,
        (1 << 32) + 1,
        u64::MAX / 2,
        u64::MAX / 2 + 1,
        u64::MAX,
    ] {
        for k in [
            0, 1, 2, 3, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512, 513, 1024,
            65536,
        ] {
            if k as u64 > n {
                continue;
            }
            for random in [false, true] {
                for mode in 0..3 {
                    let mut draw = |bound| match mode {
                        0 => rng.random_range(0..bound),
                        1 => 0,
                        _ => bound - 1,
                    };
                    let mut out = vec![u64::MAX; k];
                    if random {
                        word64::choose::<true, DEFAULT_GROUP_SIZE>(n, &mut out, &mut draw);
                    } else {
                        word64::choose::<false, DEFAULT_GROUP_SIZE>(n, &mut out, &mut draw);
                    }
                    if !random {
                        assert!(out.is_sorted());
                    }
                    out.sort_unstable();
                    assert!(out.iter().all(|&v| v < n));
                    assert!(out.windows(2).all(|w| w[0] < w[1]));
                }
            }
        }
    }
    for _ in 0..300 {
        let n = rng.random::<u64>() | 1;
        let k = n.min(rng.random_range(0..=4096)) as usize;
        let mut out = vec![u64::MAX; k];
        word64::choose::<false, 3>(n, &mut out, &mut |b| rng.random_range(0..b));
        assert!(out.iter().all(|&v| v < n));
        assert!(out.windows(2).all(|w| w[0] < w[1]));
    }
    // Exercise the public RNG adapters, including a population beyond u32.
    for k in [65, 128, 129, 256, 257, 512, 513, 1024] {
        let mut out32 = vec![0; k];
        let mut out64 = vec![0; k];
        sorted_order(&mut rng, u32::MAX, &mut out32);
        sorted_order64(&mut rng, u64::MAX, &mut out64);
        assert!(out32.windows(2).all(|w| w[0] < w[1]));
        assert!(out64.windows(2).all(|w| w[0] < w[1]));
        assert!(out32.iter().all(|&v| v < u32::MAX));
        assert!(out64.iter().all(|&v| v < u64::MAX));
        random_order(&mut rng, u32::MAX, &mut out32);
        random_order64(&mut rng, u64::MAX, &mut out64);
        out32.sort_unstable();
        out64.sort_unstable();
        assert!(out32.windows(2).all(|w| w[0] < w[1]));
        assert!(out64.windows(2).all(|w| w[0] < w[1]));
        assert!(out32.iter().all(|&v| v < u32::MAX));
        assert!(out64.iter().all(|&v| v < u64::MAX));
    }
}

#[test]
#[should_panic(expected = "require k <= n")]
fn invalid_sample() {
    sorted_order64(&mut SmallRng::seed_from_u64(324), 2, &mut [0; 3]);
}

#[test]
fn complement_boundaries() {
    let mut rng = SmallRng::seed_from_u64(324);
    for k in [8usize, 64, 256, 1024, 65536] {
        for q in [
            2,
            3,
            k / 4 - 1,
            k / 4,
            k / 4 + 1,
            k / 16,
            k / 16 + 1,
            (k / 16).saturating_sub(1),
        ] {
            for mode in 0..3 {
                let n = (k + q) as u64;
                let mut out = vec![u64::MAX; k];
                word64::choose::<false, DEFAULT_GROUP_SIZE>(n, &mut out, &mut |b| match mode {
                    0 => rng.random_range(0..b),
                    1 => 0,
                    _ => b - 1,
                });
                assert!(out.iter().all(|&x| x < n));
                assert!(out.windows(2).all(|w| w[0] < w[1]));
            }
        }
    }
}

#[test]
fn local_bitmap_and_sort_boundaries() {
    let mut rng = SmallRng::seed_from_u64(612);
    for domain in [
        2, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512, 513, 1024,
    ] {
        for count in [
            1, 2, 3, 16, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512,
        ] {
            if count > domain {
                continue;
            }
            for mode in 0..3 {
                macro_rules! check {
                    ($module:ident, $word:ty) => {{
                        let guard = <$word>::MAX;
                        let mut out = vec![guard; count + 5];
                        $module::local_sample(
                            &mut out,
                            3,
                            count,
                            guard - domain as $word - 3,
                            (domain - count + 1) as $word,
                            &mut |bound| match mode {
                                0 => 0,
                                1 => bound - 1,
                                _ => rng.random_range(0..bound),
                            },
                        );
                        assert_eq!(out[..3], [guard; 3]);
                        assert_eq!(out[count + 3..], [guard; 2]);
                        let result = &out[3..count + 3];
                        assert!(result.windows(2).all(|w| w[0] < w[1]));
                        assert!(
                            result
                                .iter()
                                .all(|&x| x >= guard - domain as $word && x < guard)
                        );
                    }};
                }
                check!(word32, u32);
                check!(word64, u64);
            }
        }
    }
}
