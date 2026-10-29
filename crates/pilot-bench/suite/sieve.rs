use std::time::Instant;

const LIMIT: usize = 2_000_000;
const ITERATIONS: usize = 30;

fn run_sieve() -> usize {
    let mut is_prime = vec![true; LIMIT + 1];
    is_prime[0] = false;
    is_prime[1] = false;

    let mut p = 2;
    while p * p <= LIMIT {
        if is_prime[p] {
            let mut i = p * p;
            while i <= LIMIT {
                is_prime[i] = false;
                i += p;
            }
        }
        p += 1;
    }

    is_prime.iter().filter(|&&b| b).count()
}

fn main() {
    let start = Instant::now();
    let mut count = 0;
    for _ in 0..ITERATIONS {
        count = run_sieve();
    }
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!("PRIMES: {} | TIME: {:.3} ms (across {} iterations)", count, elapsed_ms, ITERATIONS);
}
