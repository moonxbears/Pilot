import time

def fib(n: int) -> int:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

start = time.perf_counter()
res = fib(36)
elapsed_ms = (time.perf_counter() - start) * 1000.0
print(f"RESULT: {res} | TIME: {elapsed_ms:.3f} ms")
