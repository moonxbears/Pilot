/* Native benchmark reference: string_map.pilot in C, with an open-
   addressing table of copied keys hashed by FNV-1a, as a C programmer
   writing a small string map would. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CAP (1u << 17)
static char *keys[CAP];
static long vals[CAP];

static uint64_t fnv(const char *s) {
    uint64_t h = 1469598103934665603u;
    for (; *s; s++) { h ^= (unsigned char)*s; h *= 1099511628211u; }
    return h;
}
static long *slot(const char *k, int insert) {
    for (uint64_t i = fnv(k) & (CAP - 1);; i = (i + 1) & (CAP - 1)) {
        if (!keys[i]) {
            if (!insert) return NULL;
            keys[i] = strdup(k);
            return &vals[i];
        }
        if (strcmp(keys[i], k) == 0) return &vals[i];
    }
}

int main(void) {
    char k[64];
    for (long i = 0; i < 50000; i++) {
        snprintf(k, sizeof k, "config.section.subsection.key_%ld", i);
        *slot(k, 1) = i;
    }
    long sum = 0;
    for (int r = 0; r < 4; r++) {
        for (long i = 0; i < 50000; i++) {
            snprintf(k, sizeof k, "config.section.subsection.key_%ld", i);
            sum += *slot(k, 0);
        }
    }
    printf("%ld\n", sum);
    long misses = 0;
    for (long i = 0; i < 50000; i++) {
        snprintf(k, sizeof k, "config.section.subsection.key_%ldx", i);
        if (!slot(k, 0)) misses++;
    }
    printf("%ld\n", misses);
    return 0;
}
