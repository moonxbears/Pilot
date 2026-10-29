use std::time::Instant;

fn fib(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    let start = Instant::now();
    let res = fib(36);
    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    println!("RESULT: {} | TIME: {:.3} ms", res, elapsed_ms);
}
