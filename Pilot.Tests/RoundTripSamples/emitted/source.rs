// Code transpiled & optimized from Pilot to Rust (2024 Edition)
#![allow(unused_variables, dead_code, non_snake_case, non_camel_case_types)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Low = 1,
    Medium = 2,
    High = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Circle(f64),
    Rect(f64, f64),
}

pub trait i64Ext {
    fn doubled(&self) -> i64;
}

impl i64Ext for i64 {
    fn doubled(&self) -> i64
    {
        return self * 2;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Counter {}
impl Counter
{
    pub fn step(&self, x: i64, delta: i64) -> i64
    {
        return x + delta;
    }
}

pub fn gcd(a: i64, b: i64) -> i64
{
    let mut x: i64 = a;
    let mut y: i64 = b;
    while y != 0
    {
        let mut rem: i64 = x % y;
        x = y;
        y = rem;
    }
    return x;
}

pub fn compute_weighted_sum(limit: i64, scale: i64) -> i64
{
    let mut base_bonus: i64 = (limit + 2) * scale;
    let mut total: i64 = base_bonus;
    for i in 1..=limit
    {
        if i % 2 == 0
        {
            total += i * scale;
        }
        else if i == 5
        {
            total += 10;
        }
        else
        {
            total += i;
        }
    }
    return total;
}

pub fn main() {
    const BASE_SCALE: i64 = 3;
    let mut g: i64 = gcd(48, 18);
    let mut weighted: i64 = compute_weighted_sum(5, BASE_SCALE);
    let mut squares = ((1..=6)).into_iter().filter(|&x| x > 2).map(|x| x * x).collect::<Vec<_>>();
    assert!(g == 6);
    assert!(weighted == 53);
    println!("GCD={}, Weighted={}", g, weighted);
}
