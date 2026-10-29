#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>
#include <time.h>

#define LIMIT 2000000
#define ITERATIONS 30

static int run_sieve(void) {
    char *is_prime = (char *)malloc(LIMIT + 1);
    memset(is_prime, 1, LIMIT + 1);
    is_prime[0] = 0;
    is_prime[1] = 0;

    for (int p = 2; p * p <= LIMIT; p++) {
        if (is_prime[p]) {
            for (int i = p * p; i <= LIMIT; i += p) {
                is_prime[i] = 0;
            }
        }
    }

    int count = 0;
    for (int i = 2; i <= LIMIT; i++) {
        if (is_prime[i]) count++;
    }

    free(is_prime);
    return count;
}

int main(void) {
    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);

    int count = 0;
    for (int i = 0; i < ITERATIONS; i++) {
        count = run_sieve();
    }

    clock_gettime(CLOCK_MONOTONIC, &end);
    double elapsed_ms = (end.tv_sec - start.tv_sec) * 1000.0 + (end.tv_nsec - start.tv_nsec) / 1000000.0;

    printf("PRIMES: %d | TIME: %.3f ms (across %d iterations)\n", count, elapsed_ms, ITERATIONS);
    return 0;
}
