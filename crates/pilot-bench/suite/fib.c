#include <stdio.h>
#include <stdint.h>
#include <time.h>

static int64_t fib(int64_t n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

int main(void) {
    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);

    int64_t res = fib(36);

    clock_gettime(CLOCK_MONOTONIC, &end);
    double elapsed_ms = (end.tv_sec - start.tv_sec) * 1000.0 + (end.tv_nsec - start.tv_nsec) / 1000000.0;

    printf("RESULT: %lld | TIME: %.3f ms\n", (long long)res, elapsed_ms);
    return 0;
}
