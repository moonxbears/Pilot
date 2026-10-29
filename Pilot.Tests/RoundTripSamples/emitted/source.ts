// Code transpiled & optimized from Pilot to TypeScript

enum Priority {
    Low = 1,
    Medium = 2,
    High = 3,
}
function Circle(r: number) {
    return { "_tag": "Circle", "r": r };
}
function Rect(w: number, h: number) {
    return { "_tag": "Rect", "w": w, "h": h };
}
type Shape = ReturnType<typeof Circle> | ReturnType<typeof Rect>;
Number.prototype.doubled = function() {
    return this * 2;
};
class Counter {
    step(x: number, delta: number): number {
        return x + delta;
    }
}
function gcd(a: number, b: number): number {
    let x: number = a;
    let y: number = b;
    while (y !== 0) {
        const rem: number = x % y;
        x = y;
        y = rem;
    }
    return x;
}
function compute_weighted_sum(limit: number, scale: number): number {
    const base_bonus: number = (limit + 2) * scale;
    let total: number = base_bonus;
    for (let i = 1; i <= limit; i++) {
        if (i % 2 === 0) {
            total += i * scale;
        } else if (i === 5) {
            total += 10;
        } else {
            total += i;
        }
    }
    return total;
}
const BASE_SCALE: number = 3;
const g: number = gcd(48, 18);
const weighted: number = compute_weighted_sum(5, BASE_SCALE);
const squares = (Array.from({ length: (6 - 1 + 1) }, (_, _i) => (1) + _i)).filter((x) => x > 2).map((x) => x * x);
if (!(g === 6)) throw new Error("Assertion failed");
if (!(weighted === 53)) throw new Error("Assertion failed");
console.log(`GCD=${g}, Weighted=${weighted}`);
