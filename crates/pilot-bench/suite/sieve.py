import time

LIMIT = 2000000
ITERATIONS = 5 # 5 iterations in python because cpython byte-interpretation is slower

def run_sieve():
    is_prime = bytearray([1]) * (LIMIT + 1)
    is_prime[0] = 0
    is_prime[1] = 0

    p = 2
    while p * p <= LIMIT:
        if is_prime[p]:
            is_prime[p * p : LIMIT + 1 : p] = b"\x00" * len(range(p * p, LIMIT + 1, p))
        p += 1

    return sum(is_prime)

start = time.perf_counter()
count = 0
for _ in range(ITERATIONS):
    count = run_sieve()

elapsed_ms = (time.perf_counter() - start) * 1000.0
# Normalize to 30 iterations for direct comparison
normalized_ms = (elapsed_ms / ITERATIONS) * 30
print(f"PRIMES: {count} | TIME: {normalized_ms:.3f} ms (normalized to 30 iterations, actual {ITERATIONS} took {elapsed_ms:.1f}ms)")
