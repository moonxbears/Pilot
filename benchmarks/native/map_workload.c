/* F6 benchmark reference: map_workload.pilot written directly in C, with an
   open-addressing hash table of copied string keys. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { char *key; int64_t val; } Slot;
static Slot *slots;
static size_t cap = 16, count = 0;

static uint64_t hash(const char *s) {
    uint64_t h = 1469598103934665603u;
    while (*s) { h ^= (unsigned char)*s++; h *= 1099511628211u; }
    return h;
}

static Slot *find(Slot *t, size_t n, const char *k) {
    size_t i = hash(k) & (n - 1);
    while (t[i].key && strcmp(t[i].key, k) != 0) i = (i + 1) & (n - 1);
    return &t[i];
}

static void set(const char *k, int64_t v) {
    if ((count + 1) * 4 > cap * 3) {
        Slot *old = slots;
        size_t on = cap;
        cap *= 2;
        slots = calloc(cap, sizeof(Slot));
        for (size_t i = 0; i < on; i++) {
            if (old[i].key) *find(slots, cap, old[i].key) = old[i];
        }
        free(old);
    }
    Slot *s = find(slots, cap, k);
    if (!s->key) { s->key = strdup(k); count++; }
    s->val = v;
}

int main(void) {
    slots = calloc(cap, sizeof(Slot));
    char key[32];
    for (int64_t i = 0; i < 200000; i++) {
        snprintf(key, sizeof key, "k%" PRId64, i);
        set(key, i);
    }
    int64_t total = 0;
    for (int64_t i = 0; i < 200000; i++) {
        snprintf(key, sizeof key, "k%" PRId64, i);
        total += find(slots, cap, key)->val;
    }
    printf("%zu\n%" PRId64 "\n", count, total);
    return 0;
}
