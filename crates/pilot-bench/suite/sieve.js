const LIMIT = 2000000;
const ITERATIONS = 30;

function runSieve() {
    const isPrime = new Uint8Array(LIMIT + 1);
    isPrime.fill(1);
    isPrime[0] = 0;
    isPrime[1] = 0;

    for (let p = 2; p * p <= LIMIT; p++) {
        if (isPrime[p]) {
            for (let i = p * p; i <= LIMIT; i += p) {
                isPrime[i] = 0;
            }
        }
    }

    let count = 0;
    for (let i = 2; i <= LIMIT; i++) {
        if (isPrime[i]) count++;
    }
    return count;
}

const start = performance.now();
let count = 0;
for (let i = 0; i < ITERATIONS; i++) {
    count = runSieve();
}
const elapsedMs = performance.now() - start;
console.log(`PRIMES: ${count} | TIME: ${elapsedMs.toFixed(3)} ms (across ${ITERATIONS} iterations)`);
