/* Code transpiled & optimized from Pilot to C (C11/C23) */
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <assert.h>

typedef enum Priority {
    Priority_Low = 1,
    Priority_Medium = 2,
    Priority_High = 3,
} Priority;

typedef enum ShapeTag {
    Shape_Circle,
    Shape_Rect,
} ShapeTag;

typedef struct Shape {
    ShapeTag tag;
    union {
        struct { double r; } Circle;
        struct { double w; double h; } Rect;
    } data;
} Shape;

static inline int64_t i64_doubled(int64_t self_val) {
    return self_val * 2;
}

typedef struct Counter {
    int64_t _reserved;
} Counter;
static inline int64_t Counter_step(Counter self_val, int64_t x, int64_t delta) {
    return x + delta;
}

static inline int64_t gcd(int64_t a, int64_t b) {
    int64_t x = a;
    int64_t y = b;
    while (y != 0) {
        const int64_t rem = x % y;
        x = y;
        y = rem;
    }
    return x;
}

static inline int64_t compute_weighted_sum(int64_t limit, int64_t scale) {
    const int64_t base_bonus = (limit + 2) * scale;
    int64_t total = base_bonus;
    for (int64_t i = 1; i <= limit; ++i) {
        if (i % 2 == 0) {
            total += i * scale;
        } else if (i == 5) {
            total += 10;
        } else {
            total += i;
        }
    }
    return total;
}

int main(void) {
    const int64_t BASE_SCALE = 3;
    const int64_t g = gcd(48, 18);
    const int64_t weighted = compute_weighted_sum(5, BASE_SCALE);
    const int64_t squares = 0;
    assert(g == 6);
    assert(weighted == 53);
    printf("GCD=%lld, Weighted=%lld\n", (long long)(g), (long long)(weighted));
    return 0;
}
