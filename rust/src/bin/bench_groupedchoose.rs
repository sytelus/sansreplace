//! Fixed-iteration driver for python/bench_groupedchoose; all buffers are warm.
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use sansreplace::{cardchoose, groupedchoose, reservoirsample};
use std::{hint::black_box, time::Instant};

fn urnsort64<const RANDOM: bool>(rng: &mut SmallRng, n: u64, out: &mut [u64]) {
    if out.is_empty() {
        return;
    }
    let t = n - out.len() as u64 + 1;
    for i in 0..out.len() {
        let r = rng.random_range(0..t + i as u64);
        out[i] = if r < t { r } else { out[(r - t) as usize] };
    }
    out.sort_unstable();
    for (i, value) in out.iter_mut().enumerate() {
        *value += i as u64;
    }
    if RANDOM {
        for i in 1..out.len() {
            out.swap(i, rng.random_range(0..=i));
        }
    }
}

fn reservoir64<const RANDOM: bool>(rng: &mut SmallRng, n: u64, out: &mut [u64]) {
    if out.is_empty() {
        return;
    }
    for i in 0..out.len() {
        out[i] = i as u64;
        if RANDOM {
            out.swap(i, rng.random_range(0..=i));
        }
    }
    for i in out.len() as u64..n {
        let r = rng.random_range(0..=i);
        if r < out.len() as u64 {
            out[r as usize] = i;
        }
    }
    if !RANDOM {
        out.sort_unstable();
    }
}

fn measure<T: Copy + Default + Into<u64>>(
    n: T,
    k: usize,
    iters: u64,
    f: fn(&mut SmallRng, T, &mut [T]),
) {
    let mut out = vec![T::default(); k];
    let mut rng = SmallRng::seed_from_u64(324);
    f(&mut rng, n, &mut out);
    black_box(&out);
    let mut rng = SmallRng::seed_from_u64(324);
    let start = Instant::now();
    for _ in 0..iters {
        f(&mut rng, n, &mut out);
        black_box(&out);
    }
    let seconds = start.elapsed().as_secs_f64();
    let checksum = if k == 0 {
        0
    } else {
        out[0].into() ^ out[k - 1].into()
    };
    println!("{{\"seconds\":{seconds:.17},\"checksum\":{checksum}}}");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err("usage: bench_groupedchoose WIDTH ALGORITHM ORDER N K ITERS".into());
    }
    let n: u64 = args[4].parse()?;
    let k: usize = args[5].parse()?;
    let iters: u64 = args[6].parse()?;
    if k as u64 > n || iters == 0 {
        return Err("require k <= n and iters > 0".into());
    }
    let random = match args[3].as_str() {
        "sorted" => false,
        "random" => true,
        _ => return Err("invalid order".into()),
    };
    match args[1].as_str() {
        "32" => {
            let n = u32::try_from(n)?;
            let f = match (args[2].as_str(), random) {
                ("groupedchoose", false) => groupedchoose::sorted_order,
                ("groupedchoose", true) => groupedchoose::random_order,
                ("cardchoose", false) => cardchoose::sorted_order,
                ("cardchoose", true) => cardchoose::random_order,
                ("reservoirsample", false) => reservoirsample::sorted_order,
                ("reservoirsample", true) => reservoirsample::random_order,
                _ => return Err("unknown 32-bit algorithm".into()),
            };
            measure(n, k, iters, f);
        }
        "64" => {
            let f = match (args[2].as_str(), random) {
                ("groupedchoose", false) => groupedchoose::sorted_order64,
                ("groupedchoose", true) => groupedchoose::random_order64,
                ("urnsort", false) => urnsort64::<false>,
                ("urnsort", true) => urnsort64::<true>,
                ("reservoirsample", false) => reservoir64::<false>,
                ("reservoirsample", true) => reservoir64::<true>,
                _ => return Err("unknown 64-bit algorithm".into()),
            };
            measure(n, k, iters, f);
        }
        _ => return Err("width must be 32 or 64".into()),
    }
    Ok(())
}
