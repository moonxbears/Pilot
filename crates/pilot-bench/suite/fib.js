function fib(n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

const start = performance.now();
const res = fib(36);
const elapsed = performance.now() - start;
console.log(`RESULT: ${res} | TIME: ${elapsed.toFixed(3)} ms`);
