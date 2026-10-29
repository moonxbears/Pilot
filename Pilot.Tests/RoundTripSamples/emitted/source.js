// Code transpiled & optimized from Pilot to JavaScript (ES2024+)
"use strict";

const Priority = Object.freeze({
    Low: 1,
    Medium: 2,
    High: 3,
});
function Circle(r) {
    return { "_tag": "Circle", "r": r };
}
function Rect(w, h) {
    return { "_tag": "Rect", "w": w, "h": h };
}
Number.prototype.doubled = function() {
    return this * 2;
};
class Counter {
    step(x, delta) {
        return x + delta;
    }
}
function gcd(a, b) {
    let x = a;
    let y = b;
    while (y !== 0) {
        const rem = x % y;
        x = y;
        y = rem;
    }
    return x;
}
function compute_weighted_sum(limit, scale) {
    const base_bonus = (limit + 2) * scale;
    let total = base_bonus;
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
const BASE_SCALE = 3;
const g = gcd(48, 18);
const weighted = compute_weighted_sum(5, BASE_SCALE);
const squares = (Array.from({ length: (6 - 1 + 1) }, (_, _i) => (1) + _i)).filter((x) => x > 2).map((x) => x * x);
if (!(g === 6)) throw new Error("Assertion failed");
if (!(weighted === 53)) throw new Error("Assertion failed");
console.log(`GCD=${g}, Weighted=${weighted}`);
